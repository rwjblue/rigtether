# 0001 — Use a monorepo during feasibility

- Status: Accepted
- Date: 2026-07-29

## Context

Hardware, firmware, host transport, protocol, iOS experiments, and validation fixtures
will change together while basic feasibility is unresolved. Premature repository splits
would introduce version coordination without an independent release boundary.

## Decision

Keep product documentation, hardware source, firmware, protocol, iOS code, tools, and
bootstrap planning in `rwjblue/rigtether` through at least the M1 bench proof.

## Consequences

- Cross-layer changes can land atomically.
- One issue tracker and CI gate describe the actual proof.
- Component-specific licenses require an explicit path mapping.
- A component may move to its own repository later only when it has a stable independent
  release, ownership, or reuse boundary.
