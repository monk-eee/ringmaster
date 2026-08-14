# ADR-0027: Promote an accepted candidate into an Obligation

- **Status:** Proposed
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Depends on:** [ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md), [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md), [ADR-0023](0023-evidence-backed-daily-brief-reasons.md), [ADR-0024](0024-candidate-accept-reject-buttons.md)
- **Tags:** architecture, api, frontend, data-model, validation-ui

## Context

[ADR-0024](0024-candidate-accept-reject-buttons.md) let a candidate
transition from `candidate` to `accepted`/`rejected`, but explicitly named
"the Candidate → Obligation promotion workflow itself is separate, larger,
undecided work" as out of scope. That gap is still open: an `accepted`
candidate and an Obligation remain two separate, unlinked lifecycles.
Accepting a candidate today changes a label; it creates nothing a person
can actually track, and it never appears on the Daily Brief
([ADR-0022](0022-daily-brief-endpoint.md)) no matter how urgent it is.

`extraction::transition_candidate` already accepts an arbitrary event
type and payload with no code change required. `obligation::append_event`
already accepts a `source_fragment_id` field on a `created` event
([ADR-0023](0023-evidence-backed-daily-brief-reasons.md)). Both halves of
this promotion already exist; nothing currently calls them together.

## Decision

- `POST /api/candidates/:id/promote`:
  - `404` when the candidate id is unknown.
  - `409` when `validation_state` is not exactly `accepted` (covers a
    still-`candidate` row, an already-`rejected` row, and a row already
    `promoted` — promotion is one-way, matching accept/reject's own
    one-way 409 semantics).
  - On success: creates a new Obligation (`status: "open"`,
    `source_fragment_id` copied from the candidate's own
    `source_fragment_id`, `hard_due_at`/`soft_due_at` left null — nothing
    about a candidate implies a due date), appends a `"promoted"` event on
    the candidate (payload: `{"obligation_id": <new id>}`), rebuilds both
    projections, and returns `201` with the created Obligation row in the
    same shape `GET /api/obligations` already uses.
- `candidate_projection` gains a nullable `promoted_obligation_id` column,
  populated by `rebuild_candidate_projection` from the `"promoted"`
  event's payload — the same carry-forward treatment `source_fragment_id`
  already got in [ADR-0015](0015-expose-source-fragment-traceability-on-candidates.md).
  `GET /api/candidates` returns it so the frontend never has to re-derive
  it from raw event history.
- `CandidatesTable.tsx`: a candidate with `validation_state === "accepted"`
  renders a "Promote to Obligation" button alongside its existing state
  text. A candidate with `validation_state === "promoted"` renders its
  linked `promoted_obligation_id` (truncated, with a title tooltip,
  matching the convention already used in `ObligationsTable.tsx` and
  `DailyBrief.tsx`) instead of any action.
- No new Obligation field is added to carry a title or description. A
  promoted Obligation is identified only by its status and its linked
  `source_fragment_id`'s quoted text — already surfaced today in both
  `/api/obligations` and the Daily Brief's evidence clause
  ([ADR-0023](0023-evidence-backed-daily-brief-reasons.md)). This is a
  real, named limitation, not an oversight: Obligation has no
  title/statement field at all today, and adding one is separate,
  undecided work with its own knock-on effects on `ObligationsTable.tsx`
  and the Daily Brief reason string.

## Scope

**In scope:** the promote route; the `promoted_obligation_id` column and
its rebuild population; the button/link rendering in
`CandidatesTable.tsx`.

**Out of scope:** giving Obligation a title/description field; un-
promoting or re-linking a promotion; promoting a `candidate`-state row
directly (must go through `accepted` first — no shortcut); bulk
promotion; any change to `accept_candidate`/`reject_candidate` themselves.

## Options considered

- **A dedicated `promote` route plus a carried-forward provenance column
  (chosen):** smallest change that makes an accepted candidate become a
  real, trackable Obligation, reusing `transition_candidate` and
  `append_event` exactly as they already exist.
- **Fold promotion into `accept_candidate` itself, so accepting always
  creates an Obligation:** rejected — collapses two genuinely different
  decisions (this is worth tracking vs. this is worth tracking *as an
  Obligation right now*) into one irreversible click, and would make
  "accepted but not yet promoted" unrepresentable.
- **Derive `promoted_obligation_id` at read time from event history
  instead of a projection column:** rejected for the same reason
  `source_fragment_id` isn't derived that way either — the projection is
  rebuilt from the full log already, so recording it there costs nothing
  and keeps every read route's query shape consistent.

## Consequences

- **Positive:** closes the gap ADR-0024 named explicitly; an accepted
  candidate can become something the Daily Brief actually surfaces.
- **Negative / trade-off:** a promoted Obligation carries no descriptive
  text beyond its evidence quote, which may read as sparse until a future
  ADR addresses Obligation titles.
- **Risk:** low — no new storage engine, no auth change, reuses two
  already-accepted, already-tested code paths (`transition_candidate`,
  `append_event`) exactly as they exist today.

## Exit criteria and evidence

Evidence: [EV-0027](../evidence.d/0027-promote-accepted-candidate-to-obligation.md)

| Exit criterion | Evidence |
|---|---|
| A route promotes an `accepted` candidate into a new Obligation | `promote-route-exists` |
| `candidate_projection` carries the linked Obligation id forward | `promoted-obligation-id-column-exists` |
| The Candidates table offers a promote action and shows the link once promoted | `candidates-table-has-promote-control` |
