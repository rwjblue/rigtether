# 0007 — Use nRF5340 DK and external I²S converters for M1

- Status: Accepted
- Date: 2026-07-29

## Context

ADRs 0003–0006 fix independent USB Audio and BLE GATT transports, device-enforced PTT
safety, architecture ownership, and the exact v0 control protocol. M1 needs a
replaceable, instrumentable development platform that can test those choices without
prematurely choosing a production MCU, codec, power topology, radio interface, or
circuit.

The proof must preserve a credible Android path, make audio conversion and health
observable, leave sufficient debug and fault-injection access, and keep the sole
normally-open hardware PTT path, physical TX inhibit, and downstream sensed `PTT OUT`
separate from CAT and audio. Current-device and radio-side behavior remain human
evidence.

## Decision

Use a Nordic nRF5340 DK (PCA10095) and external Digilent Pmod I2S2 (SKU 410-379) for
the M1 development probe.

Run the supported BLE controller on the nRF5340 network core. The application core
owns the USB device, external-I²S audio service and conversion, v0 protocol services,
typed CAT adapter, diagnostics, firmware-monotonic lease timing, independent
watchdog, and the single PTT-safety service. This is an ownership split for
instrumentation, not proof that the transports can operate simultaneously.

Pin the initial implementation to nRF Connect SDK v3.3.0 and subject the exact
software stack to an early build/enumeration gate. The upstream UAC1 implementation
is legacy and synchronous-only; a supported new-stack UAC1 implementation or a
platform fallback may be required. Use the supported C/Zephyr interfaces for
hardware-facing M1 integration. Community Rust support exists for the board and USB
peripheral but does not demonstrate the accepted simultaneous topology; keep reusable
protocol, CAT, tooling, and future adapters Rust-compatible without making unproven
Rust drivers part of the safety path.

For #18, propose USB Audio Class 1.0 at full speed with one mono signed-16-bit 48 kHz
PCM Type I stream in each direction. Use the nRF5340's dedicated endpoint number 8:
synchronous isochronous OUT `0x08` and IN `0x88`, each carrying 96 bytes per 1 ms USB
frame. Discipline the external 48 kHz I²S sample clock to USB SOF through measured
firmware clock control and explicit ring health. A sample insertion/drop is an audio
fault, not successful recovery.

Use the external converters at 48 kHz with a 12.288 MHz I²S master clock and 24 valid
bits in 32-bit DMA words. Convert explicitly between USB mono 16-bit and I²S 24-bit;
keep the analog input/output jacks and both digital directions accessible to probes.

The complete descriptors, requirements/comparison matrix, resource constraints,
initial memory and buffer ceilings, health thresholds, power options, development
BOM and substitutes, staged #18 probe, conditional USB MIDI experiment, fallback
triggers, unresolved measurements, and dated primary sources are maintained in the
[M1 development-platform specification](../m1-development-platform.md).

## Consequences

- A dedicated BLE network core and application-core USB/I²S resources make the exact
  M1 topology credible enough to test while preserving independent transport health.
  Simultaneous operation is still unproven.
- External ADC/DAC wiring exposes the service and conversion boundary but creates
  signal-integrity and module-availability risks. The Pmod's CS4344 is discontinued
  and is explicitly excluded from a Rev A production decision.
- The selected UAC1 mono 16-bit 48 kHz format stays inside Android's documented
  host-mode subset. This preserves a future path without claiming Android support.
- The synchronous UAC1 clock servo, deprecated upstream UAC1 stack, external-QSPI v0
  cache, and conditional composite MIDI target become early firmware risks with
  objective fallback triggers.
- The selected DK plus Pmod was readily stocked and had the lowest one-unit
  base-platform cost in the dated comparison. Availability and price are review
  inputs, not purchase authorization, and must be refreshed before any order.
- Hardware-facing M1 firmware begins with C/Zephyr. This is an accepted toolchain
  compromise for bounded USB/BLE evidence, not a rejection of the project's Rust
  preference or a production-language decision.
- A powered USB hub is the baseline phone bench power option. Direct phone power is a
  measured #18 experiment within documented limits, not an assumption.
- Radio audio, CAT levels, PTT switching, sense protection, grounds, ESD, and
  production parts remain unsettled until #13 supplies human evidence.
- USB MIDI remains a conditional, separately gated experiment. It cannot become the
  primary transport or another CAT/PTT authority without a later ADR.

## Alternatives considered

- **nRF5340 Audio DK:** credible board-level substitute with an integrated CS47L63,
  USB-C, and current monitoring. Rejected as the baseline because the external module
  gives more replaceable and separately probed ADC/DAC paths; it retains the same
  UAC1 software risk.
- **STM32WB5MM-DK plus an external codec:** credible dual-core BLE/USB/SAI fallback.
  Rejected as the baseline because it has lower RAM margin and no primary-source
  evidence for the exact simultaneous topology; it remains the MCU fallback for an
  nRF-specific blocker.
- **USB Audio Class 2.0:** upstream nRF5340 examples exist, but UAC2 would leave
  Android's documented UAC1 host-mode baseline without contradictory evidence or an
  owner decision. It is not selected.
- **Integrated or production codec/circuit selection:** rejected because M1 must
  measure analog, clock, ground, and radio requirements before choosing Rev A parts.

## Evidence boundary

This ADR selects only a development probe and objectively testable targets. It does
not claim USB enumeration on an iPhone, simultaneous USB/BLE operation, clock
stability, phone power sufficiency, analog performance, electrical compatibility,
watchdog release timing, PTT behavior, or KX2/KX3 interoperability.

Issues #13 and #18 remain `human-required`. Their measurements may reject this
development platform without weakening ADRs 0003–0006 or the ADR 0004 safety bounds.
