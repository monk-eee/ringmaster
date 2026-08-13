---
name: adr-authoring
description: "Find, author, review, accept, amend, or supersede ringmaster Architecture Decision Records. Use when a proposed repository mutation lacks clear governing ADR coverage or when asked to write or review an ADR."
argument-hint: "decision or proposed change"
---

# ADR authoring

The policy lives in [ADR-0001](../../../docs/adr.d/0001-require-governing-adr-coverage-before-implementation.md)
and [ADR-0002](../../../docs/adr.d/0002-keep-current-evidence-separate-from-accepted-decisions.md).

## Procedure

1. Read [the ADR index](../../../docs/adr.d/README.md).
2. Search accepted records by affected component, invariant, trigger, and
   rejected alternative. Reuse one only when its decision and scope apply.
3. If no record applies, choose the next four-digit number and create one
   bounded `docs/adr.d/<number>-<slug>.md` record plus an exact-name evidence
   record under `docs/evidence.d/`.
4. Include context, explicit decision rules, scope, credible options,
   consequences, and observable exit criteria mapped to evidence check IDs.
5. Start at `Proposed`. Do not implement until the named decider explicitly
   accepts it. Never rewrite an accepted ADR; amend or supersede it.
6. Keep pre-implementation evidence honestly `manual` without
   `last_verified`. Replace it with declarative proof as implementation lands.
7. Update the ADR index, then run `node scripts/check-evidence.mjs` and
   `git diff --check`.

Evidence may use `present`, `absent`, `manual`, or `parity` checks. It must not
contain shell commands or a handwritten state. A check must prove the current
invariant, not merely match the ADR that declares it.