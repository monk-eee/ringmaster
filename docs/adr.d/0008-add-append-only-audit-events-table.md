# ADR-0008: Add an append-only audit_events table for security-relevant actions

- **Status:** Accepted
- **Date:** 2026-08-13
- **Decider:** monk-eee
- **Approval:** Continuation of accepted [docs/PRODUCT-SPEC.md](../PRODUCT-SPEC.md) §10 and Epic E1 under a general "keep working" instruction, 2026-08-13
- **Depends on:** [ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md), [ADR-0007](0007-generalize-obligation-and-require-pgvector.md)
- **Tags:** security, audit, architecture

## Context

[docs/PRODUCT-SPEC.md § 10](../PRODUCT-SPEC.md#10-security-privacy-and-responsible-use)
requires complete audit history for "extraction, validation, correction,
linking, action proposal and execution." §9.2 names an `audit_events` table
(actor, action, previous/new state, source, policy outcome) as distinct from
the domain event log. §16 Epic E1 ("Foundation") names an "audit skeleton"
as part of the initial deliverable, alongside the Rust workspace and
Postgres migrations already built under [ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md)
and [ADR-0007](0007-generalize-obligation-and-require-pgvector.md).

`audit_events` is a different concern from `obligation_events`: it is a
system-level record of who did what, through what policy outcome, not a
domain event driving a projection. It still needs the same tamper-evidence
guarantee an audit trail requires — a log a bug or a bad actor can edit is
not evidence.

## Decision

- A new `audit_events` table records `actor`, `action`, `previous_state`,
  `new_state`, `source`, and `policy_outcome` for security-relevant actions.
- Existing rows must not be mutated or deleted, enforced at the database
  level, the same way [ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md)
  enforces it for `obligation_events`.
- A minimal Rust `audit` module provides `record(...)` to append one row.
- This ADR provides the skeleton only. It does not decide which application
  actions must call `record()` — extraction, validation, correction, and the
  other actions §10 names do not exist as features yet. Wiring real call
  sites in is future, ADR-governed work as those features land.

## Scope

**In scope:** the `audit_events` schema, its immutability enforcement, and a
minimal Rust `record()` function.

**Out of scope:** deciding which application actions are audited, a query/read
API over audit history, and retention/redaction policy for audit data.

## Options considered

- **Append-only table with a Rust skeleton (chosen):** matches the spec's
  named table and Epic E1 deliverable, and reuses the already-proven
  immutability pattern instead of inventing a new one.
- **Defer audit entirely until a feature needs it:** simpler today, but
  §10 already names audit as a requirement, not an aspiration; building the
  skeleton now avoids retrofitting it under time pressure later.
- **Mutable audit rows with a separate history table:** more flexible for
  corrections, but undermines the basic property an audit trail needs —
  that its own history cannot be quietly edited.

## Consequences

- **Positive:** the repository has a real, tested place to record
  security-relevant actions before any feature needs one; it uses the same,
  already-verified immutability mechanism as `obligation_events`.
- **Negative / trade-off:** an unused table and function until real features
  call it; if none do soon, it should be revisited rather than left as
  unproven aspiration.
- **Risk:** a skeleton with no real call sites can look more complete than
  it is. This ADR states plainly that no action is audited yet.

## Exit criteria and evidence

Evidence: [EV-0008](../evidence.d/0008-add-append-only-audit-events-table.md)

| Exit criterion | Evidence |
|---|---|
| `audit_events` exists and rejects mutation or deletion of existing rows | `audit-events-table-exists`, `audit-events-are-immutable` |
| A Rust function can append one audit row | `audit-record-function-exists` |
