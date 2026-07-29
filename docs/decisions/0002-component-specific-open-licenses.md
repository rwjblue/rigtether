# 0002 — Apply component-specific open licenses

- Status: Accepted
- Date: 2026-07-29

## Context

Editable hardware source and software have different reuse and reciprocity needs. The
project also wants documentation to be broadly republishable with attribution.

## Decision

- License hardware design source under CERN-OHL-S-2.0.
- License software, protocol implementations, tooling, and bootstrap automation under
  Apache-2.0.
- License maintained prose documentation under CC-BY-4.0.
- Record the path mapping and full texts in the repository root and `LICENSES/`.

## Consequences

Contributors must determine which component they are changing and preserve the
corresponding notices. Released hardware packages must include complete preferred-form
source, not only fabrication exports.
