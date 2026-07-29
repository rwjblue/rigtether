use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

const MAX_RESPONSE_BYTES: usize = 128;

/// Incremental semicolon-terminated response decoder.
///
/// A decoder accepts partial reads and returns every complete response in order. It
/// intentionally does not decide whether a response was solicited; [`crate::CatSession`]
/// applies that transaction-level rule.
#[derive(Clone, Debug, Default)]
pub struct FrameDecoder {
    pending: Vec<u8>,
}

impl FrameDecoder {
    /// Create an empty decoder.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    /// Add an incremental byte chunk and return all newly completed responses.
    ///
    /// # Errors
    ///
    /// Returns an error for non-ASCII/control bytes or an overlong response.
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<String>, DecodeError> {
        let mut completed = Vec::new();
        for byte in bytes {
            if !byte.is_ascii() || byte.is_ascii_control() {
                return Err(DecodeError::NonAscii);
            }
            self.pending.push(*byte);
            if self.pending.len() > MAX_RESPONSE_BYTES {
                return Err(DecodeError::ResponseTooLong);
            }
            if *byte == b';' {
                let response =
                    core::str::from_utf8(&self.pending).map_err(|_| DecodeError::NonAscii)?;
                completed.push(response.to_string());
                self.pending.clear();
            }
        }
        Ok(completed)
    }

    /// Require that no unterminated response bytes remain.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError::Unterminated`] when partial bytes remain.
    pub fn finish(&self) -> Result<(), DecodeError> {
        if self.pending.is_empty() {
            Ok(())
        } else {
            Err(DecodeError::Unterminated)
        }
    }

    /// Number of bytes waiting for a semicolon.
    #[must_use]
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }
}

/// Incremental decoder failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError {
    /// A byte was non-ASCII or an ASCII control character.
    NonAscii,
    /// One response exceeded the bounded parser capacity.
    ResponseTooLong,
    /// Input ended with an incomplete response.
    Unterminated,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonAscii => formatter.write_str("response contains non-ASCII/control bytes"),
            Self::ResponseTooLong => formatter.write_str("response exceeds 128 bytes"),
            Self::Unterminated => {
                formatter.write_str("response is missing its terminating semicolon")
            }
        }
    }
}
