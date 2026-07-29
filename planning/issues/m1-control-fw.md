# Implement prototype control firmware on the selected development hardware

## Outcome

Implement the accepted v0 control service, Elecraft CAT integration, PTT lease state machine, watchdog behavior, and diagnostics on the selected development hardware.

## Context

The firmware must be testable without a radio and must make the actual PTT output state observable. Audio implementation belongs here only where the selected platform requires firmware participation.

## Scope

- Implement transport discovery, version/capability negotiation, state, commands, and errors.
- Integrate the transcript-tested CAT core and simulator mode.
- Implement PTT ownership, lease renewal/release, timeout, watchdog, and receive-safe reset behavior.
- Expose actual PTT output state and fault/reset reason.
- Provide deterministic host-side integration tests or a hardware-in-loop test harness.
- Document build, flash, logging, and recovery procedures.

## Non-goals

- Production bootloader or polished update UX.
- Additional radio profiles.
- Unbounded raw CAT access that bypasses accepted safety policy.
- Assuming physical electrical behavior not supplied by the bench-interface issue.

## Acceptance criteria

- [ ] Simulator tests cover normal control, malformed messages, disconnect, reset, watchdog, stale lease, and reconnect.
- [ ] Every safety invariant has an automated or explicitly human-required check.
- [ ] Firmware never asserts PTT during boot, reset, update entry, or profile change.
- [ ] Diagnostics are sufficient to explain an M1 failure.
- [ ] `mise run ci` passes.

{{RELATIONSHIPS}}

## Completion evidence

Record the delivered artifact or decision, source and experiment evidence, change or commit, verification commands and results, documentation updates, and follow-up issues. Update the parent tracking issue after the work lands.
