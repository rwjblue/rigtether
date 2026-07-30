//! Exact M1 USB Audio fixture data and radio-disconnected conversion model.

/// Canonical M1 UAC1 configuration descriptor from ADR 0007.
pub const CONFIGURATION_DESCRIPTOR: [u8; 192] = [
    0x09, 0x02, 0xC0, 0x00, 0x03, 0x01, 0x00, 0x80, 0xFA, 0x09, 0x04, 0x00, 0x00, 0x00, 0x01, 0x01,
    0x00, 0x00, 0x0A, 0x24, 0x01, 0x00, 0x01, 0x46, 0x00, 0x02, 0x01, 0x02, 0x0C, 0x24, 0x02, 0x01,
    0x01, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x09, 0x24, 0x06, 0x02, 0x01, 0x01, 0x01, 0x00,
    0x00, 0x09, 0x24, 0x03, 0x03, 0x03, 0x06, 0x00, 0x02, 0x00, 0x0C, 0x24, 0x02, 0x04, 0x03, 0x06,
    0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x09, 0x24, 0x06, 0x05, 0x04, 0x01, 0x01, 0x00, 0x00, 0x09,
    0x24, 0x03, 0x06, 0x01, 0x01, 0x00, 0x05, 0x00, 0x09, 0x04, 0x01, 0x00, 0x00, 0x01, 0x02, 0x00,
    0x00, 0x09, 0x04, 0x01, 0x01, 0x01, 0x01, 0x02, 0x00, 0x00, 0x07, 0x24, 0x01, 0x01, 0x08, 0x01,
    0x00, 0x0B, 0x24, 0x02, 0x01, 0x01, 0x02, 0x10, 0x01, 0x80, 0xBB, 0x00, 0x09, 0x05, 0x08, 0x0D,
    0x60, 0x00, 0x01, 0x00, 0x00, 0x07, 0x25, 0x01, 0x00, 0x00, 0x00, 0x00, 0x09, 0x04, 0x02, 0x00,
    0x00, 0x01, 0x02, 0x00, 0x00, 0x09, 0x04, 0x02, 0x01, 0x01, 0x01, 0x02, 0x00, 0x00, 0x07, 0x24,
    0x01, 0x06, 0x08, 0x01, 0x00, 0x0B, 0x24, 0x02, 0x01, 0x01, 0x02, 0x10, 0x01, 0x80, 0xBB, 0x00,
    0x09, 0x05, 0x88, 0x0D, 0x60, 0x00, 0x01, 0x00, 0x00, 0x07, 0x25, 0x01, 0x00, 0x00, 0x00, 0x00,
];

/// Samples in one mono USB full-speed frame at 48 kHz.
pub const USB_SAMPLES_PER_FRAME: usize = 48;
/// Bytes in one mono signed-16-bit USB frame.
pub const USB_BYTES_PER_FRAME: usize = 96;
/// Designed ring depth and advertised `bDelay`, in USB frames.
pub const RING_FRAMES: usize = 16;
/// Designed target delay, in USB frames.
pub const TARGET_FILL_FRAMES: usize = 8;

/// Independent audio health causes observed by the firmware service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct AudioHealth {
    /// USB configuration is active.
    pub configured: bool,
    /// Host-to-device transmit stream is active.
    pub tx_stream_active: bool,
    /// USB-disciplined sample clock is healthy.
    pub clock_healthy: bool,
    /// Playback/capture rings are within their safe bounds.
    pub buffers_healthy: bool,
    /// Required I²S clocks, DMA, and digital self-test are healthy.
    pub converter_healthy: bool,
}

impl AudioHealth {
    /// Aggregate required health without collapsing the underlying causes.
    #[must_use]
    pub const fn aggregate_healthy(self) -> bool {
        self.configured
            && self.tx_stream_active
            && self.clock_healthy
            && self.buffers_healthy
            && self.converter_healthy
    }
}

/// Convert one USB mono signed-16 sample to duplicated stereo 24-bit-in-32-bit words.
#[must_use]
pub const fn playback_mono16_to_stereo24(sample: i16) -> [i32; 2] {
    let widened = (sample as i32) << 8;
    [widened, widened]
}

/// Round and saturate one signed 24-bit ADC word to USB signed-16.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub const fn capture_left24_to_mono16(sample: i32) -> i16 {
    let clamped = if sample < -8_388_608 {
        -8_388_608
    } else if sample > 8_388_607 {
        8_388_607
    } else {
        sample
    };
    let rounded = if clamped >= 0 {
        (clamped + 128) >> 8
    } else {
        (clamped - 128) >> 8
    };
    if rounded < i16::MIN as i32 {
        i16::MIN
    } else if rounded > i16::MAX as i32 {
        i16::MAX
    } else {
        rounded as i16
    }
}

/// Ring and clock observations that never become transmit authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AudioTelemetry {
    /// Playback ring fill in USB frames.
    pub playback_fill: usize,
    /// Capture ring fill in USB frames.
    pub capture_fill: usize,
    /// Total underruns.
    pub underruns: u64,
    /// Total overruns.
    pub overruns: u64,
    /// Missed DMA completions.
    pub missed_dma: u64,
    /// Diagnostic sample insertions or drops.
    pub sample_corrections: u64,
    /// Whether the I²S frame clock is currently observed.
    pub i2s_clock_running: bool,
}

impl AudioTelemetry {
    /// Evaluate the accepted initial unsafe thresholds.
    #[must_use]
    pub const fn healthy(self) -> bool {
        self.playback_fill >= 2
            && self.playback_fill <= 14
            && self.capture_fill >= 2
            && self.capture_fill <= 14
            && self.underruns == 0
            && self.overruns == 0
            && self.missed_dma == 0
            && self.sample_corrections == 0
            && self.i2s_clock_running
    }
}
