# Implement an Elecraft CAT core and transcript fixtures

## Outcome

Create a transport-independent Elecraft CAT core for the M1 command subset, with transcript fixtures and a simulator that let firmware and iOS work proceed without a live radio.

## Context

The CAT core should encode commands, parse incremental responses, model timeout/error behavior, and replay source-backed KX2/KX3 transcripts. It must follow the accepted architecture rather than becoming an unbounded universal rig-control library.

## Scope

- Create the minimal Rust workspace or crate structure accepted by the architecture.
- Implement encoding/parsing for the exact M1 command subset.
- Handle partial reads, concatenated responses, unexpected commands, timeouts, and malformed data.
- Add transcript fixtures for KX2 and KX3 documented or human-captured behavior.
- Provide a deterministic simulator/loopback usable by firmware and iOS tests.
- Document unsupported commands and profile differences.

## Non-goals

- Full Elecraft command coverage.
- Serial electrical interfacing or live-radio validation.
- Other radio protocols.
- Embedding iOS or BLE concerns in the CAT core.

## Acceptance criteria

- [ ] All required M1 commands round-trip through fixtures.
- [ ] Streaming and malformed-input cases are tested.
- [ ] KX2/KX3 differences are explicit rather than hidden by guesses.
- [ ] The simulator is documented and reusable by downstream issues.
- [ ] `mise run ci` passes.

{{RELATIONSHIPS}}

## Completion evidence

Record the delivered artifact or decision, source and experiment evidence, change or commit, verification commands and results, documentation updates, and follow-up issues. Update the parent tracking issue after the work lands.
