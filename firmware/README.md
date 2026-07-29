# Firmware

Embedded firmware will own host transport, capability negotiation, radio profiles,
CAT transport, PTT leases, watchdog behavior, and observable fault handling.

No MCU or framework has been selected. Rust is preferred when the accepted platform
supports the required USB audio and Bluetooth behavior without compromising the proof.

Software in this directory is licensed under Apache-2.0.

The reusable, transport-independent M1 radio-control implementation lives in the
[`rigtether-elecraft-cat`](../crates/elecraft-cat/README.md) crate. Hardware-facing
firmware supplies the serial scheduler/electrical adapter, frequency policy, radio
service fault routing, and separate PTT safety service; none of those boundaries are
implemented by the CAT crate.
