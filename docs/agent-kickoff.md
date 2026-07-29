# Agent kickoff

Use an explicit issue handoff. Do not ask an agent to "start building RigTether"; the
owner-approved M1 work remains decomposed into dependency-ordered, bounded contracts.

## Recommended first handoff

Choose an open issue carrying `agent-ready` and confirm its dependencies are closed.
The M0 baseline has landed and was approved in issue #7; do not reopen it without new
contradictory evidence. The first independent M1 handoffs are issue #9, the typed
Elecraft CAT core and fixtures, and issue #10, the bench-prototype platform decision.
The GitHub queue remains authoritative for current readiness.

Example prompt:

```text
Work the explicitly handed-off GitHub issue in rwjblue/rigtether.

Follow AGENTS.md and the issue contract. Replace agent-ready with in-progress before
substantive work. Use current primary platform or manufacturer sources, record access
dates, and distinguish documentation from experiments and inference. Do not transmit,
spend money, change repository settings, or expand the product scope. Promote a settled
durable result to an ADR, update affected evergreen docs and downstream issues, run
mise run ci, land the work, post completion evidence, close the issue, and reassess any
issues it unblocks.
```

## Parallel work

The CAT core in #9 and platform decision in #10 may proceed independently. Do not hand
off firmware, the iOS probe, transport characterization, bench hardware, or integration
until every listed dependency has landed. A readiness label is not authorization;
each issue still needs an explicit handoff.

## Human boundaries

An agent may prepare an iOS probe, firmware, test instructions, and data-capture forms.
Only a human with the actual phone, radio, cables, instruments, and dummy load may
supply physical validation evidence.
