//! Transport-independent Elecraft KX2/KX3 CAT support for `RigTether` M1.
//!
//! The crate deliberately exposes no method that writes arbitrary CAT. Callers submit
//! typed [`Operation`] values through [`CatSession`]. [`Request::RawCat`] and
//! [`Request::Unsupported`] exist only at an untrusted-input boundary and are always
//! rejected before [`RadioIo`] is called.
//!
//! `IF` and `TQ` results are represented by [`TxObservation`]. They describe radio
//! state only; they cannot grant or sustain transmit authority and are not a
//! substitute for sensed `PTT OUT`.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod parser;
pub mod simulator;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use core::time::Duration;

pub use parser::{DecodeError, FrameDecoder};

/// Maximum value encodable by the documented eleven-digit `FA` field.
pub const MAX_FREQUENCY_HZ: u64 = 99_999_999_999;

/// Response windows derived from the Elecraft programmer reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponseWindow {
    /// A representative value inside the documented under-10 ms typical response.
    Typical,
    /// The documented approximately 100 ms delayed-response case.
    Delayed,
    /// The documented up-to-500 ms band-change case.
    BandChange,
}

impl ResponseWindow {
    /// Returns the document-modeled representative delay.
    #[must_use]
    pub const fn documented_delay(self) -> Duration {
        match self {
            Self::Typical => Duration::from_millis(9),
            Self::Delayed => Duration::from_millis(100),
            Self::BandChange => Duration::from_millis(500),
        }
    }

    /// Returns the provisional scheduler deadline.
    ///
    /// These conservative margins are software scheduling choices, not measured radio
    /// limits. They leave room for complete response serialization and adapter
    /// overhead without assuming undocumented serial word framing. Issue #13 may
    /// replace them with human-captured timing evidence.
    #[must_use]
    pub const fn deadline(self) -> Duration {
        match self {
            Self::Typical => Duration::from_millis(50),
            Self::Delayed => Duration::from_millis(250),
            Self::BandChange => Duration::from_millis(750),
        }
    }
}

/// A selected, model-specific radio profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Profile {
    /// Elecraft KX2.
    Kx2,
    /// Elecraft KX3.
    Kx3,
}

impl Profile {
    /// Returns the product code required in the `OM` response.
    #[must_use]
    pub const fn product_code(self) -> u8 {
        match self {
            Self::Kx2 => 1,
            Self::Kx3 => 2,
        }
    }

    /// Returns the stable profile name used by the v0 protocol.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Kx2 => "kx2",
            Self::Kx3 => "kx3",
        }
    }

    fn supports_mode(self, mode: Mode) -> bool {
        self == Self::Kx3 || mode != Mode::Fm
    }
}

/// One documented option position in a KX2/KX3 `OM` response.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptionFlag {
    /// Internal automatic antenna tuner.
    A,
    /// External 100 W power amplifier.
    P,
    /// Roofing filter.
    F,
    /// External 100 W antenna tuner.
    T,
    /// Internal battery charger/real-time clock.
    B,
    /// Transverter module.
    X,
    /// RTC I/O module.
    I,
}

/// A parsed KX2/KX3 identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Identity {
    /// Profile proven by the product code.
    pub profile: Profile,
    /// Product code from the fixed `OM` field.
    pub product_code: u8,
    /// Installed options in their documented field order.
    pub option_flags: Vec<OptionFlag>,
}

/// Firmware revisions returned by `RVM` and optional `RVD`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Firmware {
    /// Main MCU firmware revision.
    pub main: String,
    /// Main DSP firmware revision when the caller requested it.
    pub dsp: Option<String>,
}

/// An operating mode from `MD` or the fixed-width `IF` response.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    /// Lower sideband.
    Lsb,
    /// Upper sideband.
    Usb,
    /// CW.
    Cw,
    /// FM. The KX2 profile rejects this product-specific value.
    Fm,
    /// AM.
    Am,
    /// DATA.
    Data,
    /// CW reverse.
    CwReverse,
    /// DATA reverse.
    DataReverse,
}

/// Observed CAT transmit state.
///
/// This value is diagnostic only. It is not PTT authority and must never substitute
/// for the separately sensed `PTT OUT` input owned by the safety service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TxObservation {
    /// The CAT response reports receive.
    Receive,
    /// The CAT response reports transmit or pseudo-transmit.
    TransmitOrPseudoTransmit,
}

/// The allowlisted fields exposed from the complete fixed-width `IF` response.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperatingState {
    /// VFO A frequency in hertz.
    pub frequency_hz: u64,
    /// Observed transmit state, never PTT authority.
    pub tx_state: TxObservation,
}

/// Result of a completed typed operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationResult {
    /// `AI0`, `K20`, and `K30` were each query-verified.
    SessionNormalized,
    /// Product identity and option positions.
    Identity(Identity),
    /// Main and optional DSP firmware revisions.
    Firmware(Firmware),
    /// Query-verified VFO A frequency.
    Frequency(u64),
    /// Full-width parsed `IF` observation.
    OperatingState(OperatingState),
    /// Read-only operating mode.
    Mode(Mode),
    /// Read-only transmit observation.
    TxState(TxObservation),
}

/// Exact typed M1 operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Operation {
    /// Send and verify `AI0`, `K20`, and `K30`.
    NormalizeSession,
    /// Read and validate `OM`.
    Identify,
    /// Read `RVM` and optionally `RVD`.
    ReadFirmware {
        /// Whether the optional DSP revision should be queried.
        include_dsp: bool,
    },
    /// Read VFO A using `FA`.
    ReadFrequency,
    /// Set VFO A and complete only after an `FA` query verifies the result.
    SetFrequency {
        /// Requested frequency in hertz.
        frequency_hz: u64,
    },
    /// Read and fully validate `IF`.
    ReadOperatingState,
    /// Read `MD`.
    ReadMode,
    /// Read `TQ`.
    ReadTxState,
}

/// Input at the boundary between untrusted host requests and typed CAT.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Request<'a> {
    /// An allowlisted typed operation.
    Typed(Operation),
    /// Raw CAT, which is always rejected before radio I/O.
    RawCat(&'a str),
    /// Any other operation name, which is always rejected before radio I/O.
    Unsupported(&'a str),
}

/// Caller-owned frequency policy.
///
/// The core does not invent legal bands, transverter configuration, or
/// operator-facing limits. The radio service supplies those accepted constraints.
pub trait FrequencyPolicy {
    /// Validate a requested frequency before any radio I/O.
    ///
    /// # Errors
    ///
    /// Returns a stable detail when the selected profile or operator policy denies
    /// the frequency.
    fn validate(&self, profile: Profile, requested_hz: u64) -> Result<(), &'static str>;

    /// Confirm that an `FA` query is an allowed result of the requested set.
    ///
    /// # Errors
    ///
    /// Returns a stable detail when the queried value does not confirm the set.
    fn verify(
        &self,
        profile: Profile,
        requested_hz: u64,
        confirmed_hz: u64,
    ) -> Result<(), &'static str>;
}

/// A policy useful for document-derived tests.
///
/// It accepts all encodable values and recognizes either an exact result or the
/// programmer-reference behavior where the 1 Hz digit is ignored outside FINE mode.
/// Production integration must wrap this with operator/profile frequency limits.
#[derive(Clone, Copy, Debug, Default)]
pub struct DocumentedFrequencyPolicy;

impl FrequencyPolicy for DocumentedFrequencyPolicy {
    fn validate(&self, _profile: Profile, requested_hz: u64) -> Result<(), &'static str> {
        if requested_hz <= MAX_FREQUENCY_HZ {
            Ok(())
        } else {
            Err("frequency does not fit the documented eleven-digit FA field")
        }
    }

    fn verify(
        &self,
        _profile: Profile,
        requested_hz: u64,
        confirmed_hz: u64,
    ) -> Result<(), &'static str> {
        if confirmed_hz == requested_hz || confirmed_hz == requested_hz / 10 * 10 {
            Ok(())
        } else {
            Err("query did not confirm the requested or documented 10 Hz-normalized value")
        }
    }
}

/// Result of one transport exchange.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Exchange {
    /// The transport completed with zero or more response chunks.
    Complete {
        /// Incremental byte chunks in arrival order.
        chunks: Vec<Vec<u8>>,
        /// Time from command submission through the final response chunk.
        elapsed: Duration,
    },
    /// The response window expired, optionally after partial bytes arrived.
    Timeout {
        /// Partial byte chunks received before timeout.
        chunks: Vec<Vec<u8>>,
        /// Time from command submission until the adapter declared timeout.
        elapsed: Duration,
    },
}

/// A transport adapter used by [`CatSession`].
///
/// Implementations own serial scheduling and electrical I/O. They receive only
/// allowlisted bytes emitted by this crate.
pub trait RadioIo {
    /// Send one command and collect incremental bytes for its response window.
    ///
    /// # Errors
    ///
    /// Returns a transport category after an exchange was attempted.
    fn exchange(&mut self, command: &str, window: ResponseWindow) -> Result<Exchange, IoError>;
}

/// Transport-layer failure without electrical assumptions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IoError {
    /// Stable adapter-defined category.
    pub kind: String,
}

impl IoError {
    /// Create an adapter error category.
    #[must_use]
    pub fn new(kind: impl Into<String>) -> Self {
        Self { kind: kind.into() }
    }
}

/// Stable safety routing for a CAT error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SafetyDirective {
    /// Reject the request without touching the radio.
    None,
    /// Release immediately and let the safety service latch first-cause lockout.
    ReleaseAndLockout,
}

/// Stable v0 error code for radio and safety service integration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorCode {
    /// Raw, keying-capable, or otherwise non-allowlisted operation.
    UnsupportedRadioOperation,
    /// Typed argument rejected by the selected profile/operator policy.
    InvalidArgument,
    /// CAT I/O, timing, parsing, identity, or consistency uncertainty.
    RadioControlFault,
}

impl ErrorCode {
    /// Exact v0 protocol spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedRadioOperation => "unsupported_radio_operation",
            Self::InvalidArgument => "invalid_argument",
            Self::RadioControlFault => "radio_control_fault",
        }
    }
}

/// Typed CAT failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    /// Raw or non-allowlisted operation rejected before I/O.
    UnsupportedOperation {
        /// Rejected input name or raw bytes.
        operation: String,
    },
    /// Invalid typed argument rejected before I/O.
    InvalidArgument {
        /// Stable argument detail for diagnostics.
        detail: &'static str,
    },
    /// The transport adapter failed after an exchange was attempted.
    Io(IoError),
    /// The response window elapsed.
    Timeout {
        /// Command whose response was pending.
        command: &'static str,
        /// Elapsed time reported by the adapter.
        elapsed: Duration,
        /// Whether partial response bytes had arrived.
        partial_response: bool,
    },
    /// A semicolon-terminated response was structurally malformed.
    MalformedResponse {
        /// Stable parse detail.
        detail: String,
    },
    /// The radio returned `?;`.
    BusyOrInvalid,
    /// A complete response used a different command prefix.
    UnexpectedResponse {
        /// Expected response prefix.
        expected: &'static str,
        /// Actual complete response.
        actual: String,
    },
    /// Extra complete response data was received.
    UnsolicitedResponse {
        /// Unsolicited complete response.
        actual: String,
    },
    /// `OM` reported a different KX product.
    ModelMismatch {
        /// Selected profile.
        expected: Profile,
        /// Product code observed in `OM`.
        actual_product_code: u8,
    },
    /// A product-specific response value is not valid for the selected profile.
    ProfileMismatch {
        /// Selected profile.
        profile: Profile,
        /// Invalid value.
        detail: &'static str,
    },
    /// Query-after-set did not confirm the allowed result.
    VerificationMismatch {
        /// Requested frequency.
        requested_hz: u64,
        /// Returned frequency.
        confirmed_hz: u64,
    },
    /// `IF` and `TQ` observations disagreed.
    InconsistentState {
        /// Observation from `IF`.
        if_state: TxObservation,
        /// Observation from `TQ`.
        tq_state: TxObservation,
    },
}

impl Error {
    /// Stable v0 error code.
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        match self {
            Self::UnsupportedOperation { .. } => ErrorCode::UnsupportedRadioOperation,
            Self::InvalidArgument { .. } => ErrorCode::InvalidArgument,
            _ => ErrorCode::RadioControlFault,
        }
    }

    /// Whether the request reached [`RadioIo::exchange`].
    #[must_use]
    pub const fn radio_io_attempted(&self) -> bool {
        !matches!(
            self,
            Self::UnsupportedOperation { .. } | Self::InvalidArgument { .. }
        )
    }

    /// Safety-service action associated with this error.
    ///
    /// The caller performs this action immediately; CAT recovery is never awaited.
    #[must_use]
    pub const fn safety_directive(&self) -> SafetyDirective {
        if self.radio_io_attempted() {
            SafetyDirective::ReleaseAndLockout
        } else {
            SafetyDirective::None
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

/// Stateful typed CAT executor for one selected profile.
pub struct CatSession<I> {
    profile: Profile,
    io: I,
}

impl<I: RadioIo> CatSession<I> {
    /// Create a session for one explicit KX2 or KX3 profile.
    #[must_use]
    pub const fn new(profile: Profile, io: I) -> Self {
        Self { profile, io }
    }

    /// Return the selected profile.
    #[must_use]
    pub const fn profile(&self) -> Profile {
        self.profile
    }

    /// Borrow the transport adapter, including its attempt log when provided.
    #[must_use]
    pub const fn io(&self) -> &I {
        &self.io
    }

    /// Consume the session and return its transport adapter.
    #[must_use]
    pub fn into_io(self) -> I {
        self.io
    }

    /// Execute a request after allowlist and argument validation.
    ///
    /// # Errors
    ///
    /// Returns a pre-I/O rejection or a typed radio-control fault.
    pub fn execute(
        &mut self,
        request: Request<'_>,
        frequency_policy: &impl FrequencyPolicy,
    ) -> Result<OperationResult, Error> {
        let operation = match request {
            Request::Typed(operation) => operation,
            Request::RawCat(raw) => {
                return Err(Error::UnsupportedOperation {
                    operation: format!("raw_cat:{raw}"),
                });
            }
            Request::Unsupported(name) => {
                return Err(Error::UnsupportedOperation {
                    operation: name.to_string(),
                });
            }
        };

        if let Operation::SetFrequency { frequency_hz } = operation {
            if frequency_hz > MAX_FREQUENCY_HZ {
                return Err(Error::InvalidArgument {
                    detail: "frequency does not fit the documented eleven-digit FA field",
                });
            }
            frequency_policy
                .validate(self.profile, frequency_hz)
                .map_err(|detail| Error::InvalidArgument { detail })?;
        }

        match operation {
            Operation::NormalizeSession => {
                self.set_then_verify("AI0;", "AI;", "AI0;")?;
                self.set_then_verify("K20;", "K2;", "K20;")?;
                self.set_then_verify("K30;", "K3;", "K30;")?;
                Ok(OperationResult::SessionNormalized)
            }
            Operation::Identify => {
                let response = self.query("OM;", "OM", ResponseWindow::Delayed)?;
                Ok(OperationResult::Identity(parse_identity(
                    self.profile,
                    &response,
                )?))
            }
            Operation::ReadFirmware { include_dsp } => {
                let main =
                    parse_revision("RVM", &self.query("RVM;", "RVM", ResponseWindow::Delayed)?)?;
                let dsp = if include_dsp {
                    Some(parse_revision(
                        "RVD",
                        &self.query("RVD;", "RVD", ResponseWindow::Delayed)?,
                    )?)
                } else {
                    None
                };
                Ok(OperationResult::Firmware(Firmware { main, dsp }))
            }
            Operation::ReadFrequency => {
                let response = self.query("FA;", "FA", ResponseWindow::Delayed)?;
                Ok(OperationResult::Frequency(parse_frequency(&response)?))
            }
            Operation::SetFrequency { frequency_hz } => {
                let command = format!("FA{frequency_hz:011};");
                self.send_set(&command, ResponseWindow::BandChange)?;
                let response = self.query("FA;", "FA", ResponseWindow::BandChange)?;
                let confirmed_hz = parse_frequency(&response)?;
                frequency_policy
                    .verify(self.profile, frequency_hz, confirmed_hz)
                    .map_err(|_| Error::VerificationMismatch {
                        requested_hz: frequency_hz,
                        confirmed_hz,
                    })?;
                Ok(OperationResult::Frequency(confirmed_hz))
            }
            Operation::ReadOperatingState => {
                let response = self.query("IF;", "IF", ResponseWindow::Delayed)?;
                Ok(OperationResult::OperatingState(parse_if(
                    self.profile,
                    &response,
                )?))
            }
            Operation::ReadMode => {
                let response = self.query("MD;", "MD", ResponseWindow::Delayed)?;
                Ok(OperationResult::Mode(parse_mode_response(
                    self.profile,
                    &response,
                )?))
            }
            Operation::ReadTxState => {
                let response = self.query("TQ;", "TQ", ResponseWindow::Delayed)?;
                Ok(OperationResult::TxState(parse_tq(&response)?))
            }
        }
    }

    /// Query `IF` and `TQ` and require their observational transmit fields to agree.
    ///
    /// The returned value remains observation only and carries no PTT authority.
    ///
    /// # Errors
    ///
    /// Returns a typed radio-control fault, including inconsistent observations.
    pub fn read_consistent_operating_state(&mut self) -> Result<OperatingState, Error> {
        let if_response = self.query("IF;", "IF", ResponseWindow::Delayed)?;
        let state = parse_if(self.profile, &if_response)?;
        let tq_response = self.query("TQ;", "TQ", ResponseWindow::Delayed)?;
        let tq_state = parse_tq(&tq_response)?;
        if state.tx_state != tq_state {
            return Err(Error::InconsistentState {
                if_state: state.tx_state,
                tq_state,
            });
        }
        Ok(state)
    }

    fn set_then_verify(
        &mut self,
        set: &str,
        query: &'static str,
        expected: &'static str,
    ) -> Result<(), Error> {
        self.send_set(set, ResponseWindow::Delayed)?;
        let response = self.query(query, &expected[..2], ResponseWindow::Delayed)?;
        if response != expected {
            return Err(Error::MalformedResponse {
                detail: format!("normalization query {query} returned {response}"),
            });
        }
        Ok(())
    }

    fn send_set(&mut self, command: &str, window: ResponseWindow) -> Result<(), Error> {
        let exchange = self.io.exchange(command, window).map_err(Error::Io)?;
        match exchange {
            Exchange::Complete { chunks, elapsed } => {
                Self::ensure_within_window(command, window, elapsed, !chunks.is_empty())?;
                let frames = decode_chunks(&chunks)?;
                if let Some(actual) = frames.first() {
                    if actual == "?;" {
                        return Err(Error::BusyOrInvalid);
                    }
                    return Err(Error::UnsolicitedResponse {
                        actual: actual.clone(),
                    });
                }
                Ok(())
            }
            Exchange::Timeout { chunks, elapsed } => Err(Error::Timeout {
                command: command_name(command),
                elapsed,
                partial_response: !chunks.is_empty(),
            }),
        }
    }

    fn query(
        &mut self,
        command: &'static str,
        expected: &'static str,
        window: ResponseWindow,
    ) -> Result<String, Error> {
        let exchange = self.io.exchange(command, window).map_err(Error::Io)?;
        let (chunks, elapsed) = match exchange {
            Exchange::Complete { chunks, elapsed } => (chunks, elapsed),
            Exchange::Timeout { chunks, elapsed } => {
                return Err(Error::Timeout {
                    command: command_name(command),
                    elapsed,
                    partial_response: !chunks.is_empty(),
                });
            }
        };
        Self::ensure_within_window(command, window, elapsed, !chunks.is_empty())?;
        let frames = decode_chunks(&chunks)?;
        let Some(response) = frames.first() else {
            return Err(Error::Timeout {
                command: command_name(command),
                elapsed,
                partial_response: false,
            });
        };
        if response == "?;" {
            return Err(Error::BusyOrInvalid);
        }
        if frames.len() > 1 {
            let unsolicited = if response.starts_with(expected) {
                &frames[1]
            } else {
                response
            };
            return Err(Error::UnsolicitedResponse {
                actual: unsolicited.clone(),
            });
        }
        if !response.starts_with(expected) {
            return Err(Error::UnexpectedResponse {
                expected,
                actual: response.clone(),
            });
        }
        Ok(response.clone())
    }

    fn ensure_within_window(
        command: &str,
        window: ResponseWindow,
        elapsed: Duration,
        partial_response: bool,
    ) -> Result<(), Error> {
        if elapsed > window.deadline() {
            Err(Error::Timeout {
                command: command_name(command),
                elapsed,
                partial_response,
            })
        } else {
            Ok(())
        }
    }
}

fn command_name(command: &str) -> &'static str {
    match command.as_bytes() {
        [b'A', b'I', ..] => "AI",
        [b'K', b'2', ..] => "K2",
        [b'K', b'3', ..] => "K3",
        [b'O', b'M', ..] => "OM",
        [b'R', b'V', b'M', ..] => "RVM",
        [b'R', b'V', b'D', ..] => "RVD",
        [b'F', b'A', ..] => "FA",
        [b'I', b'F', ..] => "IF",
        [b'M', b'D', ..] => "MD",
        [b'T', b'Q', ..] => "TQ",
        _ => "unknown",
    }
}

fn decode_chunks(chunks: &[Vec<u8>]) -> Result<Vec<String>, Error> {
    let mut decoder = FrameDecoder::new();
    let mut frames = Vec::new();
    for chunk in chunks {
        frames.extend(
            decoder
                .push(chunk)
                .map_err(|error| Error::MalformedResponse {
                    detail: error.to_string(),
                })?,
        );
    }
    decoder.finish().map_err(|error| Error::MalformedResponse {
        detail: error.to_string(),
    })?;
    Ok(frames)
}

fn parse_identity(profile: Profile, response: &str) -> Result<Identity, Error> {
    let bytes = response.as_bytes();
    if bytes.len() != 16 || &bytes[..3] != b"OM " || bytes[13] != b'0' || bytes[15] != b';' {
        return Err(malformed("OM must match the complete fixed KX2/KX3 field"));
    }
    let expected = [
        (3, b'A', OptionFlag::A),
        (4, b'P', OptionFlag::P),
        (5, b'F', OptionFlag::F),
        (9, b'T', OptionFlag::T),
        (10, b'B', OptionFlag::B),
        (11, b'X', OptionFlag::X),
        (12, b'I', OptionFlag::I),
    ];
    for index in [6, 7, 8] {
        if bytes[index] != b'-' {
            return Err(malformed("OM reserved positions must be dashes"));
        }
    }
    let mut option_flags = Vec::new();
    for (index, marker, flag) in expected {
        match bytes[index] {
            b'-' => {}
            value if value == marker => option_flags.push(flag),
            _ => return Err(malformed("OM option marker is in the wrong fixed position")),
        }
    }
    if !bytes[14].is_ascii_digit() {
        return Err(malformed("OM product code is not decimal"));
    }
    let product_code = bytes[14] - b'0';
    if product_code != profile.product_code() {
        return Err(Error::ModelMismatch {
            expected: profile,
            actual_product_code: product_code,
        });
    }
    Ok(Identity {
        profile,
        product_code,
        option_flags,
    })
}

fn parse_revision(expected: &'static str, response: &str) -> Result<String, Error> {
    let bytes = response.as_bytes();
    if bytes.len() != 9
        || &bytes[..3] != expected.as_bytes()
        || bytes[5] != b'.'
        || bytes[8] != b';'
        || !bytes[3..5].iter().all(u8::is_ascii_digit)
        || !bytes[6..8].iter().all(u8::is_ascii_digit)
    {
        return Err(malformed("firmware response must be RVxNN.NN;"));
    }
    Ok(response[3..8].to_string())
}

fn parse_frequency(response: &str) -> Result<u64, Error> {
    let bytes = response.as_bytes();
    if bytes.len() != 14
        || &bytes[..2] != b"FA"
        || bytes[13] != b';'
        || !bytes[2..13].iter().all(u8::is_ascii_digit)
    {
        return Err(malformed(
            "FA response must contain exactly eleven decimal digits",
        ));
    }
    response[2..13]
        .parse()
        .map_err(|_| malformed("FA frequency is not an integer"))
}

fn parse_mode_response(profile: Profile, response: &str) -> Result<Mode, Error> {
    let bytes = response.as_bytes();
    if bytes.len() != 4 || &bytes[..2] != b"MD" || bytes[3] != b';' {
        return Err(malformed("MD response must be MDn;"));
    }
    parse_mode(profile, bytes[2])
}

fn parse_mode(profile: Profile, value: u8) -> Result<Mode, Error> {
    let mode = match value {
        b'1' => Mode::Lsb,
        b'2' => Mode::Usb,
        b'3' => Mode::Cw,
        b'4' => Mode::Fm,
        b'5' => Mode::Am,
        b'6' => Mode::Data,
        b'7' => Mode::CwReverse,
        b'9' => Mode::DataReverse,
        _ => return Err(malformed("mode is not in the documented M1 value set")),
    };
    if !profile.supports_mode(mode) {
        return Err(Error::ProfileMismatch {
            profile,
            detail: "FM is not supported by the KX2 profile",
        });
    }
    Ok(mode)
}

fn parse_if(profile: Profile, response: &str) -> Result<OperatingState, Error> {
    let bytes = response.as_bytes();
    if bytes.len() != 38
        || &bytes[..2] != b"IF"
        || bytes[37] != b';'
        || !bytes[2..13].iter().all(u8::is_ascii_digit)
        || bytes[13..18] != [b' '; 5]
        || !matches!(bytes[18], b'+' | b'-')
        || !bytes[19..25].iter().all(u8::is_ascii_digit)
        || !matches!(bytes[23], b'0' | b'1')
        || !matches!(bytes[24], b'0' | b'1')
        || bytes[25] != b' '
        || bytes[26..28] != *b"00"
        || !matches!(bytes[28], b'0' | b'1')
        || !matches!(bytes[30], b'0' | b'1')
        || !matches!(bytes[31], b'0' | b'1')
        || !matches!(bytes[32], b'0' | b'1')
        || bytes[33] != b'0'
        // K30 normalization selects the basic IF form; only K31 may report
        // data-submode values 1-3 in this field.
        || bytes[34] != b'0'
        || bytes[35] != b'1'
        || bytes[36] != b' '
    {
        return Err(malformed(
            "IF response violates its complete basic fixed-width form",
        ));
    }
    let _mode = parse_mode(profile, bytes[29])?;
    let frequency_hz = response[2..13]
        .parse()
        .map_err(|_| malformed("IF frequency is not an integer"))?;
    Ok(OperatingState {
        frequency_hz,
        tx_state: parse_tx_digit(bytes[28])?,
    })
}

fn parse_tq(response: &str) -> Result<TxObservation, Error> {
    let bytes = response.as_bytes();
    if bytes.len() != 4 || &bytes[..2] != b"TQ" || bytes[3] != b';' {
        return Err(malformed("TQ response must be TQ0; or TQ1;"));
    }
    parse_tx_digit(bytes[2])
}

fn parse_tx_digit(value: u8) -> Result<TxObservation, Error> {
    match value {
        b'0' => Ok(TxObservation::Receive),
        b'1' => Ok(TxObservation::TransmitOrPseudoTransmit),
        _ => Err(malformed("transmit observation must be zero or one")),
    }
}

fn malformed(detail: impl Into<String>) -> Error {
    Error::MalformedResponse {
        detail: detail.into(),
    }
}
