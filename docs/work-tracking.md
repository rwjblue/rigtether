# Work tracking and agent handoffs

GitHub Issues are RigTether's durable record for unfinished work and unresolved
choices. The files under `planning/` reproduce the initial seed only.

## Tracking layers

- The [roadmap](roadmap.md) describes current, next, and later product outcomes.
- A GitHub milestone groups issues that deliver one recognizable outcome.
- A tracking issue maps dependencies and milestone exit criteria.
- A focused decision, implementation, or human-validation issue owns one bounded
  result and its evidence.
- An ADR records a settled durable choice; it does not track unfinished research.

## Lifecycle labels

| Label | Meaning |
| --- | --- |
| `decision` | Research and record a bounded technical choice. |
| `tracking` | Parent map for a milestone; implementation remains in child issues. |
| `agent-ready` | Every dependency has landed and the issue can be handed to an agent now. |
| `in-progress` | An agent or maintainer actively owns the issue. |
| `needs-decision` | Work reached a choice outside its delegated contract. |
| `human-required` | Completion needs physical evidence, credentials, owner judgment, or real hardware. |
| `validation` | Reproducible bench, device, or interoperability evidence is central to completion. |

`agent-ready` is readiness, not authorization. Explicit user handoff starts work.

## Initial dependency order

Three M0 issues may proceed independently:

- iPhone control transport;
- Elecraft interface requirements; and
- transmit-safety invariants.

Their results unblock the system architecture. Architecture then unblocks the v0
protocol. The owner review closes M0 and authorizes M1 implementation.

M1 implementation issues are intentionally prewritten but not agent-ready. Their
contracts must be reassessed against the accepted M0 decisions before handoff.

## Completion evidence

Decision issues record sources, experiments, options, rationale, ADR, and downstream
changes. Implementation issues record delivered behavior, change or commit, tests,
documentation, and follow-ups. Human-validation issues record actual setup and results
without inventing or exposing sensitive data.

After closing an issue, inspect every issue that depends on it and update readiness
labels. A local-only change does not satisfy a GitHub dependency until it lands.
