# Product contract

## Problem

Portable radio applications commonly need three separate capabilities:

1. receive audio from the radio;
2. transmit audio to the radio; and
3. control frequency, mode, and PTT.

Existing computer-oriented interfaces often assume desktop USB serial drivers,
multiple cables, or host behavior that is awkward on iPhone. RigTether should make
those capabilities available through one compact, documented, reproducible interface
without locking applications to one radio vendor.

## Initial user story

A portable operator connects an iPhone to a small interface, connects a short
replaceable harness to a KX2 or KX3, launches an application, and can exchange audio
and control the radio without a powered hub or a pile of adapters.

## M0 product goals

- Determine a distribution-compatible iPhone transport using current primary sources.
- Establish source-backed KX2 and KX3 electrical and CAT requirements.
- Define transmit-safety invariants before implementation.
- Select a feasibility architecture with explicit component boundaries.
- Define the smallest versioned host/device control contract needed for the bench proof.

## M1 proof goals

Through an iPhone, bench prototype, KX2, and dummy load:

- obtain usable receive audio;
- inject bounded transmit audio without overdriving the microphone path;
- identify the radio and read/set frequency;
- key and unkey using deterministic, fail-safe behavior;
- recover to receive after host, link, firmware, or power faults; and
- repeat the interoperability proof on KX3 hardware.

## Non-goals for the first proof

- A polished FT8, JS8, CW, or logging application
- Remote operation over the internet
- Support for arbitrary radios or radio-specific accessory buses
- A final enclosure or manufacturable PCB
- Certification, waterproofing, or a commercial warranty
- On-air unattended transmission
- High-power RF switching or antenna control
- A stable public SDK before the transport and protocol are validated

## Product principles

- **One portable interface.** The phone-side experience should not require a hub and
  several independent adapters.
- **Safe by construction.** Receive is the default and loss of control cannot sustain
  transmit.
- **Open at every layer.** Editable hardware source, firmware, protocol, host library,
  test fixtures, and manufacturing information must be available.
- **Radio-specific at the edge.** Harnesses and adapters own connector details; host
  applications target a versioned capability model.
- **Measured, not assumed.** Electrical claims come from official specifications or
  reproducible bench evidence.
