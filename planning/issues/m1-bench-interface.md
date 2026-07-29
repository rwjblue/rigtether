# Build and document the bench audio and PTT interface

## Outcome

Build and document an instrumentable, protected bench interface for KX2/KX3 receive audio, transmit audio, CAT electrical signaling, and fail-open PTT.

## Context

This is the deliberately non-compact proof circuit that translates source-backed Elecraft requirements into safe, measurable paths. It may combine modules and breadboards; it must not masquerade as Rev A hardware.

## Scope

- Create an editable schematic and wiring diagram for every path.
- Implement bounded receive and transmit audio levels with DC blocking, adjustment, protection, and measurement points.
- Implement the documented CAT electrical interface without assuming RS-232 or raw MCU logic levels.
- Implement normally open hardware PTT, receive-safe reset bias, actual-output indication, and TX inhibit.
- Create a BOM, assembly notes, checkout steps, and expected measurement ranges.
- Perform human-supervised measurements using dummy-load precautions and record actual values.

## Non-goals

- A compact PCB or final connector system.
- On-air testing.
- Unverified galvanic-isolation claims.
- Hiding adjustment or protection inside an undocumented commercial cable.

## Acceptance criteria

- [ ] The source schematic matches the assembled bench wiring.
- [ ] RX/TX audio and CAT/PTT paths meet documented limits or raise focused blockers.
- [ ] Removing host power, resetting firmware, and opening the control link release PTT electrically.
- [ ] The TX inhibit prevents assertion independent of software.
- [ ] Measurements include setup, instruments, uncertainty, and revisions.
- [ ] `mise run ci` passes.

{{RELATIONSHIPS}}

## Completion evidence

Record the delivered artifact or decision, source and experiment evidence, change or commit, verification commands and results, documentation updates, and follow-up issues. Update the parent tracking issue after the work lands.
