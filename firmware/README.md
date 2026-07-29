# Firmware

Embedded firmware will own host transport, capability negotiation, radio profiles,
CAT transport, PTT leases, watchdog behavior, and observable fault handling.

No MCU or framework has been selected. Rust is preferred when the accepted platform
supports the required USB audio and Bluetooth behavior without compromising the proof.

Software in this directory is licensed under Apache-2.0.
