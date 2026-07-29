---
name: Agent-ready technical decision
about: Hand a bounded research or architecture choice directly to an agent
title: "Decision: "
labels: "agent-ready, decision"
assignees: ""
---

## Decision needed

State the question to resolve.

## Context

Explain why it matters and which downstream work it affects.

## Options and evidence

Compare viable options using primary sources and bounded experiments.

## Decision criteria

- Platform compatibility and distribution constraints
- Electrical and RF safety
- Complexity and maintenance cost
- Offline, disconnect, reset, and failure behavior
- Testability and observability
- Open-hardware reproducibility

## Acceptance criteria

- [ ] Current primary sources and repository constraints are documented.
- [ ] Viable options and tradeoffs are compared.
- [ ] A recommendation and rationale are recorded.
- [ ] A durable choice is promoted to an ADR when warranted.
- [ ] Follow-up issues are created or updated.

## Implementation discretion

The agent may perform read-only research and local experiments. Do not ship product
behavior, transmit on air, spend money, or make external infrastructure changes unless
this issue explicitly authorizes them.

## Result

Record the decision, rationale, consequences, ADR, and follow-up work.
