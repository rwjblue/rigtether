# Agent kickoff

Use an explicit issue handoff. Do not ask an agent to "start building RigTether"; the
project still contains unresolved architecture and safety choices.

## Recommended first handoff

Choose one of the M0 issues carrying `agent-ready`. The iPhone transport decision is a
good first choice because it constrains the host architecture without requiring physical
radio access.

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

The transport, Elecraft interface, and safety issues are independent enough for separate
agents. Do not hand off architecture or protocol issues until all of their listed
dependencies have landed.

## Human boundaries

An agent may prepare an iOS probe, firmware, test instructions, and data-capture forms.
Only a human with the actual phone, radio, cables, instruments, and dummy load may
supply physical validation evidence.
