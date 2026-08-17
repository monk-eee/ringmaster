# ADR-0049: Audit trail read API — a chronological activity feed

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Continuation of this session's established build pattern ("fix em all"), following the gap-check's proposal, 2026-08-17
- **Depends on:** [ADR-0008](0008-add-append-only-audit-events-table.md), [ADR-0038](0038-wire-up-audit-events-for-candidate-validation.md)
- **Tags:** architecture, api, frontend

## Context

[ADR-0008](0008-add-append-only-audit-events-table.md) added `audit_events`
and explicitly deferred "an audit read API." [ADR-0038](0038-wire-up-audit-events-for-candidate-validation.md)
started writing real rows to it (every candidate accept/reject/correct/
promote). [ADR-0044](0044-today-attention-items-management-meaning.md)
independently named the same gap again while scoping the Today page's
"What changed" section: *"needs a read surface over `audit_events`... ADR-0008
explicitly deferred an audit read API."* [PRODUCT-SPEC.md §10](../PRODUCT-SPEC.md#10-security-privacy-and-responsible-use)
requires *"complete audit history for extraction, validation, correction,
linking, action proposal and execution"* — data that has been accumulating
since ADR-0038 with nothing anywhere to read it back. `backend/src/audit.rs`
has exactly one function today: `record`.

`audit_events` has no `obligation_id`/`candidate_id` column — `previous_state`/
`new_state` are opaque JSONB, and different actions carry different shapes
(`candidate_accepted` carries `{"validation_state": ...}`;
`candidate_promoted` carries `{"obligation_id": ...}`). Correlating a row to
"this specific Today item" would mean either adding a new column (a real,
separate schema decision) or fragile per-action JSONB parsing this ADR does
not attempt. The honest, buildable-today slice is a flat, global,
reverse-chronological feed — not yet wired into Today's own "what changed
since I looked" framing, which stays exactly as deferred as ADR-0044 left it.

## Decision

- A new function, `audit::recent(pool, limit)`, returning up to `limit`
  rows from `audit_events` ordered by `recorded_at` descending. `limit` is
  clamped to `[1, 200]` (default 50) rather than rejected outside that
  range — this is a read-only diagnostic feed, not a validated write, and
  every other bound in this codebase (Time Horizon's day windows, Risk
  Engine's thresholds) is a disclosed clamp, not a 400.
- `GET /api/audit-events?limit=N`: returns the array as-is (`id`, `actor`,
  `action`, `previous_state`, `new_state`, `source`, `policy_outcome`,
  `recorded_at`) — no reshaping, no fabricated correlation to any
  Obligation or candidate.
- **Frontend:** a new "Activity" tab, grouped with the existing
  developer/secondary tabs (Obligations, Search, Graph, Meetings) rather
  than the primary four — this is a diagnostic/audit surface, not one of
  VISION's four primary management questions. Fetches its own data on
  mount (matching `People.tsx`/`MeetingReview.tsx`'s self-contained
  pattern rather than `App.tsx`'s combined Today/Timeline `load()`).
  Each row shows actor, action, a relative timestamp, and the raw
  previous/new state — no attempt to render a fabricated human sentence
  across action types that carry genuinely different payload shapes.

## Scope

**In scope:** `audit::recent`; `GET /api/audit-events`; the Activity tab.

**Out of scope, named honestly (deferred, larger/separate work):**
correlating a row to a specific Obligation/candidate (needs a new column
or fragile parsing, a real schema decision); Today's own "What changed"
section using this feed (ADR-0044's own deferred scope, unchanged);
pagination beyond a single capped page (200 rows is the whole feed for
now, matching this repo's dev-scale data); filtering by actor/action/date
range; any change to what `audit::record` writes.

## Options considered

- **A flat, capped, reverse-chronological feed (chosen):** zero schema
  change, exposes real data that has existed since ADR-0038 with nothing
  ever reading it, matches PRODUCT-SPEC §10's "complete audit history"
  requirement in its smallest honest form.
- **Add an `entity_id` column now so rows can be correlated to a specific
  Obligation/candidate:** would directly unblock Today's "What changed"
  section, but is a real, separate schema decision (what counts as the
  entity for every past and future action type) this ADR does not make —
  rejected as over-scoped for what was asked.
- **Parse `previous_state`/`new_state` JSONB to infer a correlated entity
  per action type:** rejected — fragile, action-type-specific special-
  casing that would silently break for any new action added later,
  exactly the kind of implicit coupling this repo's event-sourced design
  has avoided everywhere else.

## Consequences

- **Positive:** closes a gap named independently by ADR-0008 and
  ADR-0044; zero schema change; real data, not fabricated; a genuine
  first step toward PRODUCT-SPEC §10's audit-history requirement.
- **Negative / trade-off:** not yet correlated to any specific item, so it
  doesn't yet deliver Today's "What changed" framing — only the raw
  material for a future ADR to build that on top of.
- **Risk:** low. Purely additive read-only route; no writes; no schema
  migration.
