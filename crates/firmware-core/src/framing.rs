//! Runtime-sized v0 BLE fragment envelope.

use core::fmt;

const HEADER_BYTES: usize = 16;
const START: u8 = 0x01;
const END: u8 = 0x02;

/// A stable framing failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FramingError {
    /// Selected ATT value is too small for the envelope plus four payload bytes.
    TransportLimitTooSmall,
    /// Logical message is empty.
    EmptyMessage,
    /// Logical message exceeds the negotiated maximum.
    MessageTooLarge,
    /// No fragment was supplied.
    MissingStart,
    /// Fragment is shorter than the fixed envelope.
    ShortHeader,
    /// Framing version is not v0.
    UnsupportedVersion,
    /// Reserved flag bits were set.
    UnknownFlags,
    /// Reserved envelope field was nonzero.
    NonzeroReserved,
    /// A later fragment incorrectly carried `START`.
    UnexpectedStart,
    /// Transfer identity changed before completion.
    InterleavedTransfer,
    /// Total length changed before completion.
    ChangedTotalLength,
    /// Fragment offset introduced a gap or overlap.
    GapOrOverlap,
    /// Payload extends beyond the declared total.
    PayloadPastTotal,
    /// `END` appeared before the declared total.
    EarlyEnd,
    /// No fragment carried `END`.
    MissingEnd,
    /// Bytes followed the completed transfer.
    BytesAfterEnd,
}

impl FramingError {
    /// Stable vector spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TransportLimitTooSmall => "transport_limit_too_small",
            Self::EmptyMessage => "malformed_empty_message",
            Self::MessageTooLarge => "message_too_large",
            Self::MissingStart => "missing_start",
            Self::ShortHeader => "short_header",
            Self::UnsupportedVersion => "unsupported_framing_version",
            Self::UnknownFlags => "unknown_flags",
            Self::NonzeroReserved => "nonzero_reserved",
            Self::UnexpectedStart => "unexpected_start",
            Self::InterleavedTransfer => "interleaved_transfer",
            Self::ChangedTotalLength => "changed_total_length",
            Self::GapOrOverlap => "gap_or_overlap",
            Self::PayloadPastTotal => "payload_past_total",
            Self::EarlyEnd => "early_end",
            Self::MissingEnd => "missing_end",
            Self::BytesAfterEnd => "bytes_after_end",
        }
    }
}

impl fmt::Display for FramingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Fragment one nonempty logical message using a runtime ATT value limit.
///
/// # Errors
///
/// Returns a stable framing error when the negotiated limits are invalid.
pub fn fragment(
    message: &[u8],
    frame_limit: usize,
    transfer_id: u32,
    max_message_bytes: usize,
) -> Result<Vec<Vec<u8>>, FramingError> {
    if frame_limit < 20 {
        return Err(FramingError::TransportLimitTooSmall);
    }
    if message.is_empty() {
        return Err(FramingError::EmptyMessage);
    }
    if message.len() > max_message_bytes || message.len() > u32::MAX as usize {
        return Err(FramingError::MessageTooLarge);
    }
    let capacity = frame_limit - HEADER_BYTES;
    let total = u32::try_from(message.len()).map_err(|_| FramingError::MessageTooLarge)?;
    let mut frames = Vec::new();
    let mut offset = 0;
    while offset < message.len() {
        let end = message.len().min(offset + capacity);
        let mut flags = if offset == 0 { START } else { 0 };
        if end == message.len() {
            flags |= END;
        }
        let mut frame = Vec::with_capacity(HEADER_BYTES + end - offset);
        frame.extend_from_slice(&[0, flags, 0, 0]);
        frame.extend_from_slice(&transfer_id.to_be_bytes());
        frame.extend_from_slice(&total.to_be_bytes());
        frame.extend_from_slice(
            &u32::try_from(offset)
                .map_err(|_| FramingError::MessageTooLarge)?
                .to_be_bytes(),
        );
        frame.extend_from_slice(&message[offset..end]);
        frames.push(frame);
        offset = end;
    }
    Ok(frames)
}

/// Reassemble one contiguous, ordered transfer.
///
/// # Errors
///
/// Returns the first stable envelope violation.
pub fn reassemble(frames: &[Vec<u8>], max_message_bytes: usize) -> Result<Vec<u8>, FramingError> {
    if frames.is_empty() {
        return Err(FramingError::MissingStart);
    }
    let mut expected_offset = 0usize;
    let mut transfer_id = None;
    let mut total_length = None;
    let mut data = Vec::new();
    let mut ended = false;

    for (index, frame) in frames.iter().enumerate() {
        if ended {
            return Err(FramingError::BytesAfterEnd);
        }
        if frame.len() < HEADER_BYTES {
            return Err(FramingError::ShortHeader);
        }
        let version = frame[0];
        let flags = frame[1];
        let reserved = u16::from_be_bytes([frame[2], frame[3]]);
        let current_transfer = u32::from_be_bytes([frame[4], frame[5], frame[6], frame[7]]);
        let current_total = u32::from_be_bytes([frame[8], frame[9], frame[10], frame[11]]) as usize;
        let offset = u32::from_be_bytes([frame[12], frame[13], frame[14], frame[15]]) as usize;

        if version != 0 {
            return Err(FramingError::UnsupportedVersion);
        }
        if flags & !0x03 != 0 {
            return Err(FramingError::UnknownFlags);
        }
        if reserved != 0 {
            return Err(FramingError::NonzeroReserved);
        }
        if index == 0 && flags & START == 0 {
            return Err(FramingError::MissingStart);
        }
        if index > 0 && flags & START != 0 {
            return Err(FramingError::UnexpectedStart);
        }
        if let Some(value) = transfer_id {
            if value != current_transfer {
                return Err(FramingError::InterleavedTransfer);
            }
        } else {
            if current_total > max_message_bytes {
                return Err(FramingError::MessageTooLarge);
            }
            transfer_id = Some(current_transfer);
            total_length = Some(current_total);
        }
        if total_length != Some(current_total) {
            return Err(FramingError::ChangedTotalLength);
        }
        if offset != expected_offset {
            return Err(FramingError::GapOrOverlap);
        }
        let payload = &frame[HEADER_BYTES..];
        if expected_offset + payload.len() > current_total {
            return Err(FramingError::PayloadPastTotal);
        }
        data.extend_from_slice(payload);
        expected_offset += payload.len();
        if flags & END != 0 {
            if expected_offset != current_total {
                return Err(FramingError::EarlyEnd);
            }
            ended = true;
        }
    }
    if !ended {
        return Err(FramingError::MissingEnd);
    }
    Ok(data)
}
