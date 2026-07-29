<p align="center">
  <strong>RigTether</strong>
</p>

<h1 align="center">An open interface between mobile software and portable radios</h1>

<p align="center">
  Bidirectional audio, radio control, and safe transmit control over one portable interface.
</p>

RigTether is an open-hardware and open-source software project exploring a compact
interface between phones, tablets, computers, and amateur-radio transceivers. The
first target is an iPhone connected to an Elecraft KX2 or KX3, with enough capability
for applications to exchange receive and transmit audio, inspect and control the radio,
and key the transmitter safely. Android is a planned future host: the iOS-first proof
must preserve portable hardware, firmware, transport, and protocol boundaries rather
than depend on Apple-only device behavior.

The working product shape is:

```text
phone or tablet
      │
      │ USB-C and/or Bluetooth LE
      ▼
┌──────────────────────────┐
│ RigTether interface      │
│                          │
│ • bidirectional audio    │
│ • radio control          │
│ • fail-safe PTT          │
│ • capability discovery   │
└──────────────────────────┘
      │
      │ replaceable radio harness
      ▼
Elecraft KX2 / KX3 first
```

The accepted feasibility transport is standards-compliant USB Audio Class for
bidirectional audio plus Bluetooth Low Energy GATT for control. The USB-C cable is the
only physical phone tether; BLE adds no second cable. Current-device validation still
has to prove exact audio formats, coexistence, latency, reconnect behavior, and power.

## Project status

> [!WARNING]
> RigTether is in pre-prototype feasibility work. There is no validated schematic,
> firmware, app, cable, or safe-to-build hardware release. Do not order boards or
> connect unpublished circuits to a radio.

The owner-approved M0 baseline fixes the product contract, iPhone transport,
cross-host portability constraints, source-backed Elecraft interface requirements,
transmit-safety invariants, system architecture, and v0 control protocol. Current M1
work is an instrumentable end-to-end bench proof; it is not a production hardware
release. See the [roadmap](docs/roadmap.md) and
[work-tracking guide](docs/work-tracking.md).

## Initial success criterion

The first end-to-end proof should let an iPhone application, through a bench
prototype and into a dummy load:

- receive KX2 audio;
- send bounded test audio to the KX2 microphone path;
- identify the radio and read its frequency;
- change frequency through CAT;
- key and unkey PTT with fail-safe behavior; and
- return to receive automatically after app, link, or firmware failure.

KX3 interoperability is a required validation target, not an assumption.

## Repository layout

- [`docs/`](docs/README.md) — product, architecture, safety, roadmap, and decisions
- [`hardware/`](hardware/README.md) — future KiCad designs, harnesses, and mechanics
- [`firmware/`](firmware/README.md) — future embedded software
- [`ios/`](ios/README.md) — future Swift package and reference application
- [`protocol/`](protocol/README.md) — host/device and radio-control contracts
- [`planning/`](planning/README.md) — one-time GitHub bootstrap manifests
- [`AGENTS.md`](AGENTS.md) — execution contract for coding and research agents

## Starting work

The owner approved the complete M0 baseline in
[issue #7](https://github.com/rwjblue/rigtether/issues/7). The first independently
agent-ready M1 contracts are the transcript-tested Elecraft CAT core and the bounded
bench-platform decision. Physical phone, radio, cable, electrical, and RF evidence
remains human-required.

After the repository is published, the GitHub issue queue is the durable source of
truth. See [Agent kickoff](docs/agent-kickoff.md) for a handoff prompt.

## Licensing

RigTether uses component-specific licenses:

- hardware design sources: CERN-OHL-S-2.0;
- software, protocol implementations, scripts, and tooling: Apache-2.0; and
- project documentation: CC-BY-4.0.

See [`LICENSE`](LICENSE) for the exact mapping. Product and company names belong to
their respective owners; see [`NOTICE`](NOTICE).
