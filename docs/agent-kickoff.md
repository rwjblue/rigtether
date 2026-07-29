# Agent kickoff

Use an explicit issue handoff. Do not ask an agent to "start building RigTether"; the
owner-approved M1 work remains decomposed into dependency-ordered, bounded contracts.

## Recommended first handoff

Choose an open issue carrying `agent-ready` and confirm its dependencies are closed.
The M0 baseline has landed and was approved in issue #7; do not reopen it without new
contradictory evidence. Issue #10 has landed as ADR 0007 and the M1
development-platform specification. The current agent-ready M1 handoff is issue #9,
the typed Elecraft CAT core and fixtures. The GitHub queue remains authoritative for
current readiness.

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

## Dependency order

Issue #12 remains blocked on #9. Reassess it for `agent-ready` only after #9 lands and
its bounded firmware contract still matches ADR 0007. Issues #13 and #18 now have
their listed dependencies closed, but both remain `human-required` and must never
receive `agent-ready`; an agent may prepare instructions and artifacts only within an
explicit handoff that preserves the physical-evidence boundary. The iOS probe and
integration remain blocked by their listed dependencies. A readiness label is not
authorization; each issue still needs an explicit handoff.

## Human boundaries

An agent may prepare an iOS probe, firmware, test instructions, and data-capture forms.
Only a human with the actual phone, radio, cables, instruments, and dummy load may
supply physical validation evidence.
