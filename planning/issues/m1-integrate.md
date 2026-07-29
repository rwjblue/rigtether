# Integrate the end-to-end KX2 feasibility prototype

## Outcome

Connect the iOS probe, prototype firmware, bench interface, and KX2 into one reproducible dummy-load demonstration that meets the M1 product contract.

## Context

This issue integrates already validated components. It must not absorb missing architecture, platform, or electrical design work; discoveries become focused follow-up issues.

## Scope

- Document exact phone, OS, development boards, firmware, app, harness, KX2 firmware/settings, instruments, load, and power configuration.
- Demonstrate receive audio and capture objective level/quality observations.
- Inject bounded transmit test audio and verify radio input/ALC behavior without overdrive.
- Identify the KX2, read frequency, set frequency, and report command errors.
- Acquire/release PTT and exercise every practical fault path from the safety matrix.
- Produce a repeatable demonstration script, logs, measurements, and issue list.

## Non-goals

- KX3 validation, which has its own human issue.
- On-air QSOs or digital-mode automation.
- Rev A board design.
- Papering over failures with manual reset steps.

## Acceptance criteria

- [ ] Every initial success criterion in `README.md` is demonstrated or explicitly blocked.
- [ ] No tested host/link/firmware fault leaves PTT asserted beyond the accepted bound.
- [ ] Audio levels remain within documented KX2 limits.
- [ ] The full setup can be reproduced from repository instructions.
- [ ] Unexpected behavior has focused follow-up issues rather than undocumented workarounds.
- [ ] `mise run ci` passes.

{{RELATIONSHIPS}}

## Completion evidence

Record the delivered artifact or decision, source and experiment evidence, change or commit, verification commands and results, documentation updates, and follow-up issues. Update the parent tracking issue after the work lands.
