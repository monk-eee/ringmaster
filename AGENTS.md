# Agent guidance

Keep changes small, grounded, and validated. Repository decisions live in
[`docs/adr.d/`](docs/adr.d/README.md); current proof lives separately in
[`docs/evidence.d/`](docs/evidence.d/).

## Before implementation

For every change to source, tests, configuration, infrastructure, or pipelines,
agents must identify and read an accepted governing ADR before implementation.
Search by affected component, invariant, trigger, and rejected alternative. A
title match alone is not coverage.

Reuse an accepted ADR only while its decision and scope govern the work. If none
applies, draft a bounded ADR and exact-name evidence record, then obtain explicit
acceptance from the named decider before changing implementation. Purely
editorial corrections that do not alter behavior, constraints, interfaces, or
operating rules are exempt.

Never rewrite an accepted ADR. Record a changed direction in a new ADR that
amends or supersedes it.

## Evidence

Agents must run `node scripts/check-evidence.mjs` before reporting an ADR as
Proven, Broken, Stale, Asserted, or Deadheaded. State is derived, never inferred
from file inspection or written into an ADR or evidence record.

Evidence must prove the implementation or invariant, not merely match the ADR's
claim. Evidence records are declarative data and must never contain executable
shell commands. Report checker failures honestly; do not weaken a check to make
it pass.

## Validation

Run the narrowest relevant project checks after each implementation change and
the full repository gate before completion. Until product-specific commands are
established, the documentation gate is:

```bash
node scripts/check-evidence.mjs
git diff --check
```

Follow [the contributor guide](docs/CONTRIBUTING.md) for review requirements.