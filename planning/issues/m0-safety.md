# Decision: define transmit-safety invariants and failure behavior

## Outcome

Define and record the transmit-safety invariants, ownership model, timeouts, interlocks, and failure responses that every feasibility implementation must satisfy.

## Context

PTT spans app, transport, firmware, transistor or optocoupler, cable, and radio. A crash, disconnect, reconnect, or stale packet must not leave the transmitter keyed. These constraints must be settled before architecture and protocol design.

## Scope

- Create a hazard and failure-mode table covering boot, brownout, reset, watchdog, host detach, BLE/control loss, app suspension/termination, stale messages, duplicate messages, profile changes, firmware update, CAT failure, audio failure, and hardware faults.
- Define receive-safe states, PTT ownership, maximum lease duration, renewal behavior, and recovery requirements.
- Decide which guarantees must be enforced in hardware, firmware, host software, or more than one layer.
- Evaluate a physical TX-inhibit control and actual-output TX indication for the bench and Rev A stages.
- Define testable acceptance checks for each invariant without requiring on-air transmission.
- Update hardware-safety and architecture docs and create an ADR if the result is durable.

## Non-goals

- Selecting a specific PTT transistor, relay, or optocoupler.
- Final regulatory certification or RF-exposure analysis.
- An unattended transmit mode.
- Replacing the radio manufacturer operating manual.

## Acceptance criteria

- [ ] Every identified failure has an expected receive-safe response or an explicit unresolved risk.
- [ ] PTT lease and timeout semantics are precise enough for the protocol issue.
- [ ] Hardware-enforced and software-enforced guarantees are clearly separated.
- [ ] The M1 bench validation checklist can verify each invariant.
- [ ] Evergreen docs and downstream issue contracts reflect the decision.
- [ ] `mise run ci` passes.

## Implementation discretion

The agent may reason from documented interfaces and established fail-safe design practices. Do not claim a physical result, component qualification, or regulatory conclusion without evidence.

{{RELATIONSHIPS}}

## Completion evidence

Record the delivered artifact or decision, source and experiment evidence, change or commit, verification commands and results, documentation updates, and follow-up issues. Update the parent tracking issue after the work lands.
