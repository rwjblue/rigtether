//! Deterministic transcript parsing and replay.
//!
//! The fixture format is deliberately line-oriented and dependency-free so firmware,
//! Swift, host tools, and a future Android adapter can consume the same records:
//! `delay_ms|command|response`. A blank response models a SET with no immediate
//! response. `!timeout` models expiration. Metadata uses `# key=value`.

use crate::{Exchange, IoError, Profile, RadioIo, ResponseWindow};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::time::Duration;

/// Evidence class declared by a transcript.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Provenance {
    /// Synthetic bytes and values derived from cited documents.
    DocumentDerived,
    /// Future bytes captured from a radio under human supervision.
    HumanCaptured,
}

/// One deterministic transcript exchange.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranscriptStep {
    /// Delay before completion or timeout.
    pub delay: Duration,
    /// Exact command expected from the core.
    pub command: String,
    /// Complete response bytes, an empty SET response, or timeout.
    pub outcome: StepOutcome,
}

/// Result supplied by a transcript step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StepOutcome {
    /// One response, split into the listed deterministic chunks.
    Response(Vec<Vec<u8>>),
    /// No immediate response to a SET command.
    NoResponse,
    /// The exchange times out, optionally with partial chunks.
    Timeout(Vec<Vec<u8>>),
}

/// Parsed cross-platform transcript fixture.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transcript {
    /// Evidence classification.
    pub provenance: Provenance,
    /// Explicit KX2 or KX3 profile.
    pub profile: Profile,
    /// Source URL or future capture artifact identifier.
    pub source: String,
    /// Ordered exchanges.
    pub steps: Vec<TranscriptStep>,
}

impl Transcript {
    /// Parse the stable line-oriented fixture format.
    ///
    /// # Errors
    ///
    /// Returns a line-addressed error when metadata or a replay record is invalid.
    pub fn parse(input: &str) -> Result<Self, TranscriptError> {
        let mut provenance = None;
        let mut profile = None;
        let mut source = None;
        let mut capture_id = None;
        let mut steps = Vec::new();

        for (line_index, raw_line) in input.lines().enumerate() {
            let line_number = line_index + 1;
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('#') {
                parse_metadata(
                    line,
                    line_number,
                    &mut provenance,
                    &mut profile,
                    &mut source,
                    &mut capture_id,
                )?;
                continue;
            }

            steps.push(parse_step(line, line_number)?);
        }

        let provenance =
            provenance.ok_or_else(|| TranscriptError::new(0, "missing provenance metadata"))?;
        let profile = profile.ok_or_else(|| TranscriptError::new(0, "missing profile metadata"))?;
        let source = source.ok_or_else(|| TranscriptError::new(0, "missing source metadata"))?;
        if source.is_empty() {
            return Err(TranscriptError::new(0, "source metadata must not be empty"));
        }
        if provenance == Provenance::HumanCaptured && capture_id.as_deref().unwrap_or("").is_empty()
        {
            return Err(TranscriptError::new(
                0,
                "human-captured transcript requires capture-id metadata",
            ));
        }
        if steps.is_empty() {
            return Err(TranscriptError::new(0, "transcript has no exchanges"));
        }
        Ok(Self {
            provenance,
            profile,
            source,
            steps,
        })
    }
}

fn parse_metadata(
    line: &str,
    line_number: usize,
    provenance: &mut Option<Provenance>,
    profile: &mut Option<Profile>,
    source: &mut Option<String>,
    capture_id: &mut Option<String>,
) -> Result<(), TranscriptError> {
    let Some((key, value)) = line
        .strip_prefix('#')
        .and_then(|metadata| metadata.trim().split_once('='))
    else {
        return Err(TranscriptError::new(
            line_number,
            "metadata requires key=value",
        ));
    };
    match key.trim() {
        "provenance" => {
            *provenance = Some(match value.trim() {
                "document-derived" => Provenance::DocumentDerived,
                "human-captured" => Provenance::HumanCaptured,
                _ => return Err(TranscriptError::new(line_number, "unknown provenance")),
            });
        }
        "profile" => {
            *profile = Some(match value.trim() {
                "kx2" => Profile::Kx2,
                "kx3" => Profile::Kx3,
                _ => return Err(TranscriptError::new(line_number, "unknown profile")),
            });
        }
        "source" => *source = Some(value.trim().to_string()),
        "capture-id" => *capture_id = Some(value.trim().to_string()),
        _ => return Err(TranscriptError::new(line_number, "unknown metadata key")),
    }
    Ok(())
}

fn parse_step(line: &str, line_number: usize) -> Result<TranscriptStep, TranscriptError> {
    let mut fields = line.splitn(3, '|');
    let delay = fields
        .next()
        .ok_or_else(|| TranscriptError::new(line_number, "missing delay"))?
        .parse::<u64>()
        .map_err(|_| TranscriptError::new(line_number, "delay must be milliseconds"))?;
    let command = fields
        .next()
        .ok_or_else(|| TranscriptError::new(line_number, "missing command"))?;
    let response = fields
        .next()
        .ok_or_else(|| TranscriptError::new(line_number, "missing response field"))?;
    if !command.ends_with(';') || !command.is_ascii() {
        return Err(TranscriptError::new(
            line_number,
            "command must be semicolon-terminated ASCII",
        ));
    }
    let outcome = if response.is_empty() {
        StepOutcome::NoResponse
    } else if response == "!timeout" {
        StepOutcome::Timeout(Vec::new())
    } else if let Some(partial) = response.strip_prefix("!timeout:") {
        StepOutcome::Timeout(parse_chunks(partial, line_number)?)
    } else {
        let chunks = parse_chunks(response, line_number)?;
        if !response.ends_with(';') {
            return Err(TranscriptError::new(
                line_number,
                "complete response must end in a semicolon",
            ));
        }
        StepOutcome::Response(chunks)
    };
    Ok(TranscriptStep {
        delay: Duration::from_millis(delay),
        command: command.to_string(),
        outcome,
    })
}

fn parse_chunks(value: &str, line_number: usize) -> Result<Vec<Vec<u8>>, TranscriptError> {
    let mut chunks = Vec::new();
    for chunk in value.split('~') {
        if chunk.is_empty() || !chunk.is_ascii() {
            return Err(TranscriptError::new(
                line_number,
                "response chunks must be nonempty ASCII",
            ));
        }
        chunks.push(chunk.as_bytes().to_vec());
    }
    Ok(chunks)
}

/// Fixture parsing failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranscriptError {
    /// One-based line number, or zero for missing file metadata.
    pub line: usize,
    /// Stable validation detail.
    pub detail: &'static str,
}

impl TranscriptError {
    fn new(line: usize, detail: &'static str) -> Self {
        Self { line, detail }
    }
}

/// Deterministic [`RadioIo`] implementation backed by one transcript.
#[derive(Clone, Debug)]
pub struct Simulator {
    transcript: Transcript,
    cursor: usize,
    elapsed: Duration,
    attempts: Vec<String>,
}

impl Simulator {
    /// Create a simulator at the start of a transcript.
    #[must_use]
    pub const fn new(transcript: Transcript) -> Self {
        Self {
            transcript,
            cursor: 0,
            elapsed: Duration::ZERO,
            attempts: Vec::new(),
        }
    }

    /// Parse a fixture and create a simulator.
    ///
    /// # Errors
    ///
    /// Returns a line-addressed transcript validation error.
    pub fn from_fixture(input: &str) -> Result<Self, TranscriptError> {
        Transcript::parse(input).map(Self::new)
    }

    /// Transcript profile.
    #[must_use]
    pub const fn profile(&self) -> Profile {
        self.transcript.profile
    }

    /// Cumulative deterministic time.
    #[must_use]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Commands for which radio I/O was attempted.
    #[must_use]
    pub fn attempts(&self) -> &[String] {
        &self.attempts
    }

    /// Number of replay records consumed.
    #[must_use]
    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    /// Whether every transcript record was consumed.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.cursor == self.transcript.steps.len()
    }

    /// Reset replay position, time, and attempt log deterministically.
    pub fn reset(&mut self) {
        self.cursor = 0;
        self.elapsed = Duration::ZERO;
        self.attempts.clear();
    }
}

impl RadioIo for Simulator {
    fn exchange(&mut self, command: &str, _window: ResponseWindow) -> Result<Exchange, IoError> {
        self.attempts.push(command.to_string());
        let Some(step) = self.transcript.steps.get(self.cursor) else {
            return Err(IoError::new("transcript_exhausted"));
        };
        if step.command != command {
            return Err(IoError::new(format!(
                "transcript_expected:{}",
                step.command
            )));
        }
        self.cursor += 1;
        self.elapsed += step.delay;
        match &step.outcome {
            StepOutcome::Response(chunks) => Ok(Exchange::Complete {
                chunks: chunks.clone(),
                elapsed: step.delay,
            }),
            StepOutcome::NoResponse => Ok(Exchange::Complete {
                chunks: Vec::new(),
                elapsed: step.delay,
            }),
            StepOutcome::Timeout(chunks) => Ok(Exchange::Timeout {
                chunks: chunks.clone(),
                elapsed: step.delay,
            }),
        }
    }
}

/// Repository document-derived KX2 fixture.
pub const KX2_DOCUMENT_FIXTURE: &str = include_str!("../fixtures/document-derived/kx2.cat");

/// Repository document-derived KX3 fixture.
pub const KX3_DOCUMENT_FIXTURE: &str = include_str!("../fixtures/document-derived/kx3.cat");
