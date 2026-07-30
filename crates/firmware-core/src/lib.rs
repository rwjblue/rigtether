//! Host-neutral `RigTether` M1 firmware core and deterministic simulator.
//!
//! This crate is executable software evidence for the v0 protocol, transport framing,
//! audio conversion, typed CAT boundary, and dedicated safety state machine. The
//! nRF5340 hardware-facing implementation lives under `firmware/nrf5340`; this crate
//! does not claim physical PTT, watchdog-release, USB, BLE, electrical, or radio
//! behavior.

#![forbid(unsafe_code)]
#![allow(
    clippy::assigning_clones,
    clippy::manual_let_else,
    clippy::needless_pass_by_value,
    clippy::single_match_else,
    clippy::too_many_lines,
    clippy::unnecessary_wraps
)]

pub mod audio;
pub mod framing;
pub mod model;
pub mod radio;
pub mod strict_json;

/// Maximum accepted v0 PTT lease.
pub const T_LEASE_MAX_MS: u64 = 500;
/// Maximum release interval after expiry or a detected safety event.
pub const T_RELEASE_MAX_MS: u64 = 100;
/// Non-extendable continuous-authority cap.
pub const T_CONTINUOUS_MAX_MS: u64 = 60_000;
/// Continuously receive-safe interval required after the continuous cap.
pub const T_REARM_MIN_MS: u64 = 1_000;
/// Minimum retained operations in a v0 session.
pub const MAX_SESSION_OPERATIONS: u64 = 512;
