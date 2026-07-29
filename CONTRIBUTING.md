# Contributing to RigTether

RigTether is currently resolving feasibility and safety questions before committing
to production hardware. Contributions are useful, but speculative implementation
should not outrun the milestone and issue contracts.

## Before starting

1. Read [`AGENTS.md`](AGENTS.md), [`docs/product.md`](docs/product.md), and
   [`docs/hardware-safety.md`](docs/hardware-safety.md).
2. Find or open a focused GitHub issue.
3. Confirm that blocking decisions are closed before implementing downstream work.
4. Obtain explicit maintainer handoff before treating an `agent-ready` issue as
   authorized work.

Research contributions should rely on primary sources, distinguish documented facts
from bench observations, and identify unresolved values instead of guessing.

Hardware contributions must include editable design sources. Images, PDFs, Gerbers,
and fabrication exports alone are not sufficient source under the hardware license.

## Pull requests

A pull request should:

- deliver one bounded issue outcome;
- include tests, fixtures, or validation evidence appropriate to the change;
- update maintained documentation when behavior or constraints change;
- avoid unrelated cleanup;
- pass `mise run ci`; and
- link the issue it closes.

Do not include secrets, private Apple entitlements, serial numbers, personal station
information, or proprietary documents.

## Licensing of contributions

By contributing, you agree that your contribution is licensed under the component
license assigned in [`LICENSE`](LICENSE). Do not contribute material you cannot
license on those terms.
