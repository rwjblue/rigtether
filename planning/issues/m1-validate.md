# Human validation: verify KX2 fault recovery and KX3 interoperability

## Outcome

Supply real KX2 fault-recovery evidence and repeat the supported M1 contract on KX3 hardware, then record the M1 go/no-go for Rev A design.

## Context

An unattended agent cannot truthfully provide physical radio, phone, cable, instrument, dummy-load, or fault evidence. Agents may prepare the procedure and reporting template; the owner performs or directly supervises the tests.

## Scope

- Review and execute the KX2 fault matrix using the integrated prototype.
- Repeat audio, CAT, PTT, and fault checks on a documented KX3 configuration.
- Record KX2/KX3 differences and update the radio profile or harness requirements.
- Document failures, heat, level, current, timing, and recovery observations.
- Decide whether evidence supports beginning M2 and which risks remain.
- Update interoperability docs, tracking issues, and M2 readiness.

## Non-goals

- Invented, simulated, or inferred physical readings.
- On-air unattended transmission.
- Starting Rev A layout before the go/no-go is recorded.

## Acceptance criteria

- [ ] Actual KX2 and KX3 setup and revisions are recorded.
- [ ] Every safety fault test has an observed result and bound.
- [ ] Audio, CAT, and PTT compatibility claims are evidence-backed.
- [ ] Failures have focused issues with severity and workaround status.
- [ ] The owner records a Rev A go/no-go and rationale.
- [ ] M1 tracking and M2 readiness are accurate.

{{RELATIONSHIPS}}

## Completion evidence

Record the delivered artifact or decision, source and experiment evidence, change or commit, verification commands and results, documentation updates, and follow-up issues. Update the parent tracking issue after the work lands.
