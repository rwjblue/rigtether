# Development workflow

## Prerequisites

The repository's current checks require:

- Python 3;
- [mise](https://mise.jdx.dev/) for task discovery; and
- GitHub CLI (`gh`) only when publishing the bootstrap plan.

No firmware, iOS, Rust, or KiCad toolchain is selected by the initial scaffold.

## Verification

```sh
mise run check
mise run ci
```

Both currently validate repository structure, issue metadata, dependencies, and text
hygiene. Toolchain-specific checks should be added with the first implementation that
needs them.

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
