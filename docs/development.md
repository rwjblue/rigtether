# Development workflow

## Prerequisites

The repository's current checks require:

- Python 3;
- [mise](https://mise.jdx.dev/) for task discovery; and
- GitHub CLI (`gh`) only when publishing the bootstrap plan.

The initial M1 firmware probe is pinned to nRF Connect SDK v3.3.0 for reproducible
build evidence. Its exact development board, external audio module, USB descriptors,
resource ceilings, staged fixtures, and toolchain risks are defined in the
[M1 development-platform specification](m1-development-platform.md). The repository
does not install that toolchain. The checked-in nRF5340 application, simulator, build
task, and evidence boundaries are documented in the
[M1 firmware runbook](m1-firmware.md).

No production firmware, iOS, or KiCad toolchain is selected. The M1 C/Zephyr
hardware-facing choice and Rust simulator are bounded prototype decisions, not a
production-language decision.

## Verification

```sh
mise run check
mise run ci
```

Both currently validate repository structure, issue metadata, dependencies, and text
hygiene, the protocol and firmware shared vectors, typed CAT integration, exact BLE
UUIDs, and canonical UAC1 descriptor parity. The optional NCS build and
radio-disconnected flash tasks are `firmware:build` and `firmware:flash`.

## Publishing the initial repository

After reviewing the files and authenticating `gh` for the `rwjblue` account:

```sh
mise run publish
```

The publish task creates `rwjblue/rigtether` if necessary, pushes `main`, configures
repository metadata, and creates or updates labels, milestones, and initial issues.
The script is idempotent by exact issue title.

To publish only labels, milestones, and issues into an existing repository:

```sh
mise run bootstrap-github
```

After publication, edit work in GitHub rather than changing `planning/issues/*.md` as
a shadow tracker. Keep the bootstrap files for reproducibility and provenance.
