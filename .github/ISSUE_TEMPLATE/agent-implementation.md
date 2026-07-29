---
name: Agent-ready implementation
about: Define a bounded implementation slice that can be handed directly to an agent
title: ""
labels: "agent-ready, enhancement"
assignees: ""
---

Use this template only when every blocking dependency has landed and the issue can
be handed to an agent immediately. Use the planned implementation template when
preserving future work whose dependencies remain open.

## Outcome

Describe the concrete result this issue should deliver.

## Context

Explain why it matters and identify the current source of truth.

## Implementation contract

The implementation must preserve accepted architecture and transmit-safety
invariants, update maintained documentation, and avoid new electrical assumptions.

## Scope

- Required capability
- Required integration points
- Required tests, fixtures, simulator evidence, or bench evidence
- Required documentation changes

## Non-goals

- Explicitly deferred behavior
- Adjacent systems that must not change

## Acceptance criteria

- [ ] Required observable behavior exists.
- [ ] Important failure and disconnect cases are covered.
- [ ] Interoperability evidence is reproducible.
- [ ] Maintained documentation is accurate.
- [ ] `mise run ci` passes.
- [ ] The working-copy diff contains no unrelated changes.

## Dependencies

- Depends on: None
- Blocks: None

## Implementation discretion

The agent may choose private module organization and test decomposition consistent
with repository conventions. Stop for material public API, hardware architecture,
electrical-limit, safety, or external-authority expansion.

## Completion evidence

Record delivered behavior, Jujutsu change or commit, verification commands and
results, documentation changes, and follow-up issues. Close the issue and update its
tracking issue after the work lands.
