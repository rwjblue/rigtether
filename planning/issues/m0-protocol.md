# Define the RigTether v0 control protocol contract

## Outcome

Define the smallest versioned control contract needed for discovery, capabilities, radio state, CAT operations, safe PTT, status, and errors in the M1 proof.

## Context

The protocol is shared by firmware and iOS and must reflect the accepted transport and transmit-safety decisions. It should avoid freezing production APIs while still enabling independent implementation and transcript tests.

## Scope

- Define framing or characteristic boundaries for the selected transport.
- Define version negotiation, device identity, radio profile/capabilities, and feature discovery.
- Choose and document the relationship between typed operations and raw CAT passthrough.
- Specify PTT lease acquisition, renewal, release, timeout, denial, and actual-output status.
- Specify errors, reconnect behavior, idempotency, ordering, and observability.
- Provide machine-readable or textual test vectors for success and failure cases.
- Update `protocol/README.md`, architecture docs, and downstream implementation issues.

## Non-goals

- A complete universal radio object model.
- Remote internet operation, authentication accounts, or cloud services.
- Final firmware update protocol unless required for M1.
- Backward compatibility before a first implementation exists.

## Acceptance criteria

- [ ] Two independent implementations can interoperate from the written contract and vectors.
- [ ] PTT semantics satisfy the accepted safety decision.
- [ ] Unknown and intentionally deferred fields are explicit.
- [ ] The contract has a clear version and compatibility rule.
- [ ] M1 firmware and iOS issues reference the same source of truth.
- [ ] `mise run ci` passes.

{{RELATIONSHIPS}}

## Completion evidence

Record the delivered artifact or decision, source and experiment evidence, change or commit, verification commands and results, documentation updates, and follow-up issues. Update the parent tracking issue after the work lands.
