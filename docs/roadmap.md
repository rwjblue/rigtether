# Roadmap

The roadmap describes product outcomes, not a task list. GitHub milestones and focused
issues own executable work.

## Completed — M0: feasibility and architecture

The owner approved the source-backed baseline in issue #7 on 2026-07-29:

- iPhone control transport and distribution constraints — accepted in ADR 0003;
- cross-host constraints that preserve a future Android implementation — allocated to
  platform adapters and host-neutral contracts in ADR 0005;
- KX2/KX3 audio, CAT, and PTT requirements — source-backed with physical unknowns
  reserved for M1 measurement;
- transmit-safety invariants — accepted in ADR 0004;
- feasibility architecture and component boundaries — accepted in ADR 0005; and
- versioned v0 control protocol — accepted in ADR 0006 with executable conformance
  vectors.

That approval authorizes the M1 bench proof without claiming physical, electrical, RF,
current-device, or KX2/KX3 validation.

## Current — M1: end-to-end bench proof

Demonstrate useful audio, CAT, and fail-safe PTT from an iPhone through development
hardware to a KX2 on a dummy load, then validate the same contract with a KX3.

The proof should be intentionally ugly, instrumentable, and replaceable. It is not a
production PCB. Work proceeds through independent but observable USB Audio, BLE
control, audio conversion, radio-profile/CAT, and PTT-safety boundaries. Simulators,
transcript fixtures, loopback, and a radio-disconnected PTT fixture precede
human-supervised KX2/KX3 measurements.

M1 chooses development hardware and exact USB descriptors and PCM formats. Those
choices should stay inside Android's documented USB Audio Class 1 host-mode subset
when compatible with the iPhone proof. Any exception needs contradictory evidence, an
explicit owner decision, and a credible Android alternate path.

## Later — M2: Rev A open-hardware reference design

After M1 evidence supports proceeding:

- choose production-oriented components;
- create KiCad schematic and PCB sources;
- design the detachable Elecraft harness and enclosure;
- assemble and validate prototypes; and
- publish complete manufacturing source and known limitations.

Before Rev A freezes USB descriptors, power behavior, or BLE semantics, run a bounded
interoperability check on representative USB-C Android hardware or record an explicit
owner decision accepting the incompatibility and a credible alternate path.

Detailed M2 child issues should be written only after the M1 exit review.

## Later — M3: developer preview

Stabilize the host protocol and Swift package, add configuration/update tooling, build a
small reference iOS application, and publish a documented developer preview.

## Future — Android host support

After the iOS-first transport, hardware, safety, and protocol contracts have evidence,
implement an Android host adapter and reference application. Validate USB Audio and BLE
control together on a documented device/OS matrix, including power, routing, lifecycle,
reconnect, and receive-safe fault behavior.

Support for additional radios and applications follows evidence from the first preview,
not speculation during M0.
