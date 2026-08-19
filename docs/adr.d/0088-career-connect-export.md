# ADR-0088: Career/Connect export — a person's completed obligation history, with evidence

- **Status:** Accepted
- **Date:** 2026-08-19
- **Decider:** monk-eee
- **Approval:** Continuing this session's established autonomous-work practice ("keep working" / "work autonomously and make good decisions" when unavailable), 2026-08-19 — Priority 4 chosen specifically because Priority 3 (ADO source coverage) is explicitly gated on a data-access-control policy decision `docs/IMPROVEMENT-PLAN.md` itself says should not be drafted speculatively
- **Depends on:** [ADR-0022](0022-daily-brief-endpoint.md), [ADR-0023](0023-evidence-backed-daily-brief-reasons.md), [ADR-0028](0028-person-relationship-view.md), [ADR-0051](0051-relationship-workspace.md), [ADR-0083](0083-meeting-brief-generation.md)
- **Tags:** api, frontend

## Context

`docs/IMPROVEMENT-PLAN.md` Priority 4 names a real Learn.ADOA gap: *"The
People obligation type covers onboarding, career follow-up, and
recognition (PRODUCT-SPEC.md §5.1), but there is no dedicated report/export
view — the artifact a manager would actually paste into a Connect
self-assessment."*

Two honesty constraints, both already established by prior records, shape
this one:

- **No stored "People" obligation category exists.** [ADR-0082](0082-repeated-concern-risk-signal.md)
  and [ADR-0085](0085-focus-blocks-people-filter.md) already confirmed
  `obligation_projection` carries no `kind`/type column distinguishing a
  People-type Obligation from a Delivery/Leadership/Operational one.
  Filtering this export to "just People obligations" would require
  inventing a classification with no real basis — the same fabrication
  this repo's conventions consistently refuse. This export is honestly
  scoped to *every* completed Obligation linked to a person, letting the
  manager themselves pick what's Connect-relevant, exactly as
  [ADR-0085](0085-focus-blocks-people-filter.md) left the
  Delivery/Leadership/Operational split to a future, real classification
  source.
- **No existing read returns a person's *closed* Obligations at all.**
  `get_node_detail`'s `relationship` grouping ([ADR-0028](0028-person-relationship-view.md)/[ADR-0051](0051-relationship-workspace.md))
  and `person_brief`'s `open_commitments` ([ADR-0083](0083-meeting-brief-generation.md))
  both explicitly filter to `status != 'closed'` — the exact opposite data
  a Connect self-assessment needs (finished accomplishments, not open
  work). Confirmed directly: `backend/src/api/nodes.rs`'s own test
  comments state "a closed obligation must never appear in either
  relationship group." No schema change is needed to close this gap —
  `obligation_projection.status = 'closed'` rows already exist and are
  queryable; nothing has ever read them per-person.

## Decision

- **A new, additive read, `person_career_history(pool, person_id)`** in
  `backend/src/api/nodes.rs`, alongside (not replacing) `person_brief`:
  returns every `status = 'closed'` Obligation linked to the person by any
  edge, each with its evidence citation (`source_text`) and a new,
  closed-specific reason string — reusing the existing evidence-clause
  wording `daily_brief_reason` already uses ("Last evidence: ..."/"No
  evidence recorded."), not that function's due-date clause, since "Due in
  N days" reads nonsensically for an already-closed item. Ordered by
  `updated_at` descending (most recently closed first) — the same column
  the rest of the app already treats as "last touched," honest about the
  absence of a dedicated `closed_at` column.
- **Exposed as `GET /api/people/:id/career-export`**, matching the
  existing `/api/people/:id/brief` route shape and convention.
- **A "Career export" section on Person detail** (`frontend/src/components/People.tsx`),
  rendered below the existing Relationship section: an honest empty state
  when there are zero closed Obligations, otherwise a plain-text,
  copy-to-clipboard block listing each completed item with its evidence —
  the literal artifact a manager pastes into a Connect self-assessment,
  not a fabricated narrative summary of it.

## Scope

**In scope:** `person_career_history` in `nodes.rs`; the `GET
/api/people/:id/career-export` route; a Career export section and
copy-to-clipboard action on Person detail.

**Out of scope, named honestly:** any People/Delivery/Leadership/
Operational categorization or filtering (no real classification exists to
ground it, per the Context above); a dedicated `closed_at` timestamp
column (would need a schema/event-payload change; `updated_at` is the
existing, honest proxy every other feature already relies on); PDF/Word
export or any formatted document generation (plain copyable text only);
an MCP tool for this read (no named agent-query in PRODUCT-SPEC.md asks
for it, unlike ADR-0083's `prepare_meeting_brief`; can be added later if
requested); any change to `get_node_detail`'s `relationship` grouping or
`person_brief`'s `open_commitments` (both remain closed-obligation-free,
unchanged).

## Options considered

- **A new closed-obligation read plus a plain-text export block (chosen):**
  closes the exact named gap — real completed work with real evidence,
  formatted for copy-paste — with zero schema change and zero invented
  categorization.
- **Filter to only obligations linked via a heuristic "People-like" node
  type (e.g. only person-to-person edges):** rejected — every Obligation
  in this export is already person-linked by construction (that's how it
  was found); an additional node-type filter on top would be an arbitrary,
  ungrounded second filter, not a real distinction.
- **Generate a narrative Connect-style paragraph via a template or
  model call:** rejected — this repo's own established posture (ADR-0084's
  "presentation change over existing data, not new extraction") argues
  for exposing the real, cited facts plainly; a generated narrative risks
  reading as more confident or complete than the underlying evidence
  actually is.
- **Add a `closed_at` column now, set at the closing event:** the more
  correct long-term data model, but a real migration/event-payload change
  this bounded, additive record doesn't need to take on; `updated_at`
  already serves this purpose honestly elsewhere in the app.

## Exit criteria (evidence-checkable)

| Invariant | Evidence check id |
|---|---|
| `person_career_history` returns only `status = 'closed'` Obligations linked to the person, each with an evidence citation | `career-history-returns-closed-obligations-with-evidence` |
| An open (non-closed) Obligation linked to the same person is excluded | `career-history-excludes-open-obligations` |
| `GET /api/people/:id/career-export` serves the same composition over HTTP | `career-history-http-route-exists` |
| Person detail renders a Career export section with an honest empty state when there is nothing closed | `career-export-honest-empty-state` |
| `get_node_detail`'s `relationship` grouping and `person_brief`'s `open_commitments` remain unchanged (still closed-obligation-free) | `existing-reads-remain-closed-obligation-free` |
