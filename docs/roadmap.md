# Roadmap

The roadmap describes product outcomes, not a task list. GitHub milestones and focused
issues own executable work.

## Current — M0: feasibility and architecture

Establish a source-backed product baseline:

- iPhone control transport and distribution constraints;
- KX2/KX3 audio, CAT, and PTT requirements;
- transmit-safety invariants;
- feasibility architecture and component boundaries; and
- versioned v0 control protocol.

Exit requires explicit owner approval of the baseline and authorization to build the
bench proof.

## Next — M1: end-to-end bench proof

Demonstrate useful audio, CAT, and fail-safe PTT from an iPhone through development
hardware to a KX2 on a dummy load, then validate the same contract with a KX3.

The proof should be intentionally ugly, instrumentable, and replaceable. It is not a
production PCB.

## Later — M2: Rev A open-hardware reference design

After M1 evidence supports proceeding:

- choose production-oriented components;
- create KiCad schematic and PCB sources;
- design the detachable Elecraft harness and enclosure;
- assemble and validate prototypes; and
- publish complete manufacturing source and known limitations.

Detailed M2 child issues should be written only after the M1 exit review.

## Later — M3: developer preview

Stabilize the host protocol and Swift package, add configuration/update tooling, build a
small reference iOS application, and publish a documented developer preview.

Support for additional radios and applications follows evidence from the first preview,
not speculation during M0.
