# ADR-0038: Wire up audit_events for candidate validation actions

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** monk-eee delegated the accept/implement decision directly ("work autonomously and make good decisions"), 2026-08-17
- **Depends on:** [ADR-0008](0008-add-append-only-audit-events-table.md), [ADR-0024](0024-candidate-accept-reject-buttons.md), [ADR-0027](0027-promote-accepted-candidate-to-obligation.md)
- **Tags:** security, audit, api

## Context

[PRODUCT-SPEC.md § 10](../PRODUCT-SPEC.md#10-security-privacy-and-responsible-use)
requires "complete audit history for extraction, validation, correction,
linking, action proposal and execution."
[ADR-0008](0008-add-append-only-audit-events-table.md) built the
append-only `audit_events` table and a Rust `audit::record(...)` skeleton
specifically to satisfy this, but deliberately stopped there: "it does not
decide which application actions must call `record()` ... wiring real call
sites in is future, ADR-governed work as those features land." That
skeleton has sat with zero call sites since. Three real, shipped actions now
exist that are exactly the "validation" §10 names: `POST
/api/candidates/:id/accept`, `.../reject`
([ADR-0024](0024-candidate-accept-reject-buttons.md)), and `.../promote`
([ADR-0027](0027-promote-accepted-candidate-to-obligation.md)) — each a
one-way state transition on a candidate, exactly the kind of action an
audit trail exists to record.

## Decision

- After each of `accept_candidate`, `reject_candidate`, and
  `promote_candidate` successfully commits its state change, call
  `audit::record(...)` in the **same database transaction** as that state
  change (wrapping the existing single-statement transitions in
  `pool.begin()`/`tx.commit()` where they are not already transactional, as
  `promote_candidate` already is) — so an action is never left un-audited
  by a failure between the two writes, and a failed audit write rolls back
  the action instead of silently proceeding un-recorded.
- Recorded fields, honestly scoped to what the system actually knows today:
  - `actor`: the fixed literal `"local-operator"` — [ADR-0004](0004-defer-multi-user-access-control-single-user-v1.md)'s
    single-user v1 has no session/identity model, so this names what's
    true (one operator) rather than inventing a fake per-request identity.
  - `action`: `"candidate_accepted"` / `"candidate_rejected"` /
    `"candidate_promoted"`.
  - `previous_state` / `new_state`: `{"validation_state": "..."}` (and, for
    promote, the new Obligation id) — the same before/after shape the
    candidate row itself already tracks, not a new schema.
  - `source`: `"http_api"`.
  - `policy_outcome`: `"allowed"` — there is no policy/rules engine yet
    (Congruence Engine, Risk Engine both still vision); this records that
    the existing state-machine check passed, not a richer evaluation that
    doesn't exist.
- A failed `audit::record` call surfaces as the same `500` the route
  already returns for any other database error — no new error type.

## Scope

**In scope:** audit call sites for `accept_candidate`, `reject_candidate`,
`promote_candidate`, each atomic with its own state change.

**Out of scope:** auditing extraction, edge creation/linking, or meeting
ingestion (each a separately-justified follow-up, not bundled here to keep
this record reviewable); a query/read API over `audit_events` (still
explicitly out of scope per [ADR-0008](0008-add-append-only-audit-events-table.md));
a real `actor` identity (blocked on the multi-user work
[ADR-0004](0004-defer-multi-user-access-control-single-user-v1.md)
explicitly defers); retention/redaction policy.

## Options considered

- **Audit the three candidate-validation routes first, atomically (chosen):**
  smallest slice that is unambiguously "validation" per §10's own wording,
  reuses the already-built, already-tested `audit::record` skeleton
  verbatim, and closes ADR-0008's oldest-standing named gap ("no call
  sites yet").
- **Audit every write route at once (extraction, edges, promote, meeting
  ingestion, candidate transitions):** rejected as too broad for one
  reviewable decision; each of those is its own defensible follow-up ADR
  once this first slice proves the pattern.
- **Fire-and-forget audit write (not in the same transaction):** rejected —
  an audit trail that can silently miss the action it's supposed to record
  (if the audit insert fails after the state change already committed)
  defeats the purpose; atomicity is the whole point of pairing them.
- **Wait for multi-user auth before wiring any audit call sites:** rejected
  — `actor` being a known, honestly-labeled placeholder today doesn't
  block recording the other genuinely useful fields (what happened, when,
  what changed); revisit only `actor` once real identity exists, not the
  whole feature.

## Consequences

- **Positive:** closes ADR-0008's explicitly-named gap ("no call sites
  yet") for the one action type (validation) §10 lists that already has
  shipped routes; zero new schema, zero new dependency, reuses an
  already-tested function verbatim.
- **Negative / trade-off:** `actor` is a placeholder, not a real identity,
  until multi-user access control exists — named honestly rather than
  faked.
- **Risk:** low. Purely additive (one more write per existing route, same
  transaction); no change to any route's request/response contract or
  status codes on the success path.

## Exit criteria and evidence

Evidence: [EV-0038](../evidence.d/0038-wire-up-audit-events-for-candidate-validation.md)

| Exit criterion | Evidence |
|---|---|
| Accepting a candidate writes an immutable audit row in the same transaction as the state change | `accept-writes-audit-row` |
| Rejecting a candidate writes an immutable audit row in the same transaction as the state change | `reject-writes-audit-row` |
| Promoting a candidate writes an immutable audit row in the same transaction as the state change | `promote-writes-audit-row` |
| `actor` is the honestly-labeled single-user placeholder, not a fabricated identity | `actor-is-honest-placeholder` |
