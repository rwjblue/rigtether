# Decision: define the feasibility architecture and component boundaries

## Outcome

Accept a feasibility architecture that maps the selected iPhone transport, Elecraft interfaces, safety invariants, prototype hardware, firmware, protocol, and iOS responsibilities into explicit boundaries.

## Context

The architecture must synthesize the three independent M0 decisions. It should authorize an instrumentable bench proof without prematurely selecting a production PCB or broad radio abstraction.

## Scope

- Define the phone, transport, audio, control core, CAT adapter, PTT, watchdog, harness, and radio boundaries.
- Choose what belongs in hardware, firmware, the Swift package, and the probe application.
- Describe startup, discovery, capability negotiation, normal RX/TX, disconnect, reset, and update state flows.
- Identify development-board and audio-interface requirements without selecting parts unless necessary.
- Record observability requirements: logs, test points, loopback, simulator, and transcript fixtures.
- Write an accepted ADR and update the architecture, roadmap, and M1 contracts.

## Non-goals

- Production component selection, PCB layout, or enclosure design.
- A stable public SDK.
- Additional radio vendors.
- Detailed digital-mode application behavior.

## Acceptance criteria

- [ ] The architecture diagram and state flows cover every M1 success criterion and safety invariant.
- [ ] Each cross-layer responsibility has one clear owner and a test strategy.
- [ ] Deferred production decisions are explicit.
- [ ] An ADR records the selected architecture and rejected alternatives.
- [ ] M1 issues are still coherent and have accurate dependencies.
- [ ] `mise run ci` passes.

{{RELATIONSHIPS}}

## Completion evidence

Record the delivered artifact or decision, source and experiment evidence, change or commit, verification commands and results, documentation updates, and follow-up issues. Update the parent tracking issue after the work lands.
