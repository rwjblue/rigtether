# Track M0: feasibility and architecture

## Outcome

Produce an owner-approved, source-backed feasibility baseline that is safe and specific enough to authorize an end-to-end bench prototype.

## Child work

{{CHILDREN}}

## Dependency order

The transport, Elecraft-interface, and transmit-safety decisions may proceed in parallel. All three must land before architecture. Architecture and the settled transport/safety constraints unblock the v0 protocol. Owner review closes the milestone.

## Exit criteria

- [ ] Current iPhone control paths and distribution constraints are documented.
- [ ] KX2 and KX3 electrical, audio, CAT, and PTT requirements are documented with unknowns.
- [ ] Transmit-safety invariants and fault behavior are accepted.
- [ ] A feasibility architecture and component boundaries are accepted in an ADR.
- [ ] The v0 control contract is documented with test vectors.
- [ ] The owner records a go/no-go and any scope changes for M1.

{{RELATIONSHIPS}}

## Completion evidence

Keep this tracker current as children land. Close it only when every exit criterion has real evidence and downstream readiness has been reassessed.
