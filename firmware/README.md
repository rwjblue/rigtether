# Firmware

The M1 firmware prototype targets the Nordic nRF5340 DK (PCA10095) application core
with the supported Bluetooth controller on the network core over HCI IPC.

Two deliberately separate implementations live here:

- [`nrf5340/`](nrf5340/) is the nRF Connect SDK v3.3.0 C/Zephyr hardware boundary. It
  advertises the exact v0 BLE service, discovers the ATT value limit per connection,
  validates the 16-byte fragment envelope, dispatches completed logical messages,
  frames operation responses, renders status from the safety snapshot, exports the
  canonical UAC1 descriptor, retains all negotiated operation bytes in the selected
  external-QSPI cache partition, serializes queued indications, implements the
  radio-disconnected typed-command fixture, provides mono16/stereo24 conversion,
  starts receive-safe, and gives one safety service sole ownership of release,
  monotonic timing, and watchdog feed.
- [`rigtether-firmware-core`](../crates/firmware-core/README.md) is the host-neutral,
  radio-disconnected Rust simulator. It consumes every shared v0 scenario, runtime
  framing vector, strict-JSON rejection vector, and the typed CAT core/simulator from
  issue #9.

The checked-in board configuration keeps `CONFIG_RIGTETHER_PTT_OUTPUT_ENABLED=n`.
Issue #13 must define and validate the normally-open output, physical inhibit,
downstream `PTT OUT` sense, pins, protection, and electrical limits before a local
radio-disconnected safety-fixture overlay can enable an output. There is no live-radio
test path in this repository.

Build, flash, trace, diagnostics, simulator, logging, fault-injection, and safe
recovery procedures are maintained in the
[M1 firmware runbook](../docs/m1-firmware.md).

Software in this directory is licensed under Apache-2.0.
