# Contributing

## Before implementation

Every change to source, tests, configuration, infrastructure, or pipelines must
be covered by an accepted ADR in [`docs/adr.d/`](adr.d/README.md) before
implementation. Reuse an existing record only when its decision and scope apply.
Otherwise, use the repository ADR authoring skill to draft a bounded decision
and matching evidence record, then obtain explicit acceptance from its named
decider.

Purely editorial corrections that do not change behavior, constraints,
interfaces, or operating rules may use the `N/A - editorial` exemption.

## Pull requests

Use the GitHub pull request template and link the governing ADR or state the
editorial exemption. Keep a pull request focused on one reviewable outcome and
include the validation evidence needed to assess it.

Before requesting review, run:

```bash
node scripts/check-evidence.mjs
git diff --check
```

Add project-specific build, format, lint, and test commands here when the Rust
and Node project structures are established under accepted ADRs.