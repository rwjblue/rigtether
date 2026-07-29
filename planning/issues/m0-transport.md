# Decision: choose the iPhone control transport for the feasibility prototype

## Outcome

Select and document the control transport an ordinarily distributed iOS application will use for the feasibility prototype, including the relationship between audio and control.

## Context

The leading hypothesis is USB Audio Class for bidirectional audio and BLE for CAT/PTT, but generic USB serial, composite USB, Accessory Access, External Accessory/MFi, and other viable paths must be evaluated against current Apple behavior rather than memory.

## Scope

- Use current primary Apple documentation to identify public APIs, entitlements, MFi requirements, OS/device constraints, background behavior, and App Store implications.
- Compare BLE control, a composite USB device, supported USB accessory APIs, and any other viable normally distributable option.
- Separate audio-path requirements from control-path requirements and explain whether one physical tether is still practical.
- Perform only small local software experiments needed to resolve documentation ambiguity; record code and results when useful.
- Update ADR 0003 to Accepted, Rejected, or Superseded and update architecture/interoperability docs.
- Rewrite affected M1 issue contracts and readiness labels to match the result.

## Non-goals

- Selecting the production MCU or audio codec.
- Building the reference app or firmware.
- Private APIs, jailbreak-only behavior, or an enterprise-only distribution assumption.
- Purchasing hardware or applying for commercial programs or entitlements.

## Acceptance criteria

- [ ] A dated source table covers every material platform claim.
- [ ] A comparison matrix includes distribution, entitlement, latency, reliability, power, complexity, and single-tether implications.
- [ ] The recommendation names a fallback and the evidence that would trigger it.
- [ ] ADR 0003 and evergreen docs reflect the durable result.
- [ ] Downstream issues accurately describe the selected transport and blockers.
- [ ] `mise run ci` passes.

## Implementation discretion

The agent may create throwaway or checked-in probe code when it materially resolves uncertainty. Stop before external enrollment, paid hardware, or a transport choice that requires owner acceptance beyond the criteria in this issue.

{{RELATIONSHIPS}}

## Completion evidence

Record the delivered artifact or decision, source and experiment evidence, change or commit, verification commands and results, documentation updates, and follow-up issues. Update the parent tracking issue after the work lands.
