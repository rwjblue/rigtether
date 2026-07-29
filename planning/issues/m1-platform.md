# Decision: select the bench-prototype MCU and audio strategy

## Outcome

Select readily obtainable development hardware and an audio strategy for the instrumentable M1 bench proof.

## Context

The choice must satisfy the accepted host transport, USB audio requirements, BLE/control behavior, firmware language/toolchain, test-point access, and safety state machine. It is not a production BOM decision.

## Scope

- Derive hard requirements from the accepted architecture and protocol.
- Compare viable MCU/development boards, integrated versus external audio codec paths, USB device capabilities, BLE support, debugging, Rust support, availability, and cost.
- Prefer off-the-shelf modules that expose signals and avoid an early custom PCB.
- Define the exact M1 bench BOM and substitutes.
- Record setup risks, unavailable capabilities, and any fallback architecture.
- Promote the result to an ADR and update downstream issues.

## Non-goals

- Selecting Rev A production components.
- Ordering parts without owner approval.
- Optimizing enclosure size or battery life.
- Hiding a transport limitation behind proprietary firmware.

## Acceptance criteria

- [ ] The selected platform satisfies every hard M1 requirement or documents an accepted compromise.
- [ ] At least one credible alternative is compared.
- [ ] The BOM is specific enough for owner review and purchasing.
- [ ] A fallback exists for the highest-risk capability.
- [ ] An ADR and downstream contracts reflect the result.
- [ ] `mise run ci` passes.

{{RELATIONSHIPS}}

## Completion evidence

Record the delivered artifact or decision, source and experiment evidence, change or commit, verification commands and results, documentation updates, and follow-up issues. Update the parent tracking issue after the work lands.
