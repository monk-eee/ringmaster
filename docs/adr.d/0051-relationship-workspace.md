# ADR-0051: Relationship workspace — People shows who needs something from you, not every person node

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Continuation of this session's established build pattern ("accept everything continue"), 2026-08-17
- **Depends on:** [ADR-0028](0028-person-relationship-view.md), [ADR-0039](0039-product-re-steer-primary-navigation.md), [ADR-0040](0040-dated-source-ingestion.md), [ADR-0042](0042-occurred-at-retrieval-and-recall-sources-mcp-tool.md)
- **Tags:** frontend, api, architecture

## Context

[docs/current-status.md](../current-status.md)'s audit found the People
tab fetches and renders every `person` node with no filtering or
pagination — 1,007 of them in the current dev database, almost all test
fixtures. An independent product review of that audit put the underlying
problem plainly: *"You never wanted `People → list every person`. You
wanted `People → People who need attention from me`."* [ADR-0028](0028-person-relationship-view.md)
already resolves one person's linked Obligations into `at_risk`/`open`
groups — the per-person detail page is a real relationship view, just not
yet surfaced as *the reason to open a person* from the list itself, and
missing two pieces of "external memory" the review named: when a
commitment/request last had real evidence, and what's actually owed.

## Decision

- **`GET /api/nodes?node_type=person` gains an optional `?needs_attention=true`
  filter**: only person nodes with at least one linked `open` or `at_risk`
  Obligation (the same join [ADR-0028](0028-person-relationship-view.md)'s
  detail read already does, applied as an existence filter here). The
  People tab uses this by default, with an explicit "Show everyone" toggle
  back to today's unfiltered list — never a silent, permanent hide.
- **The person detail read (`GET /api/nodes/:id`,
  [ADR-0028](0028-person-relationship-view.md)) gains two more real
  fields, both derived from data that already exists:**
  - `last_interaction_at`: the most recent `source_fragments.occurred_at`
    (via its parent node, [ADR-0042](0042-occurred-at-retrieval-and-recall-sources-mcp-tool.md))
    among fragments whose `speaker` string-matches this person's
    `canonical_text`. Best-effort name matching, not a resolved identity
    edge — [ADR-0040](0040-dated-source-ingestion.md) already deferred
    participant-to-person-node resolution, and this ADR does not revisit
    that. `null` when no fragment matches, rendered as an honest "No
    recorded interaction," never guessed.
  - `risk_signals` attached to each grouped Obligation in `relationship`,
    the same computed signals Daily Brief/Time Horizon already show
    ([ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md)/[ADR-0046](0046-unowned-obligation-risk-signal.md)) —
    reused, not recomputed differently.
- **The People list card leads with what's owed**: person name, the count
  of `at_risk`/`open` Obligations, and `last_interaction_at`'s human phrase
  — not just a name, matching Today's existing "management meaning, not a
  raw list" posture ([ADR-0044](0044-today-attention-items-management-meaning.md)).

## Scope

**In scope:** the `needs_attention` filter on the person list route; two
new derived fields on person detail (`last_interaction_at`,
`risk_signals` on relationship Obligations); the People list card's
content.

**Out of scope, named honestly:**

- **"Upcoming conversation."** The review's mockup named a next scheduled
  meeting/date. Nothing in this schema models a *future* meeting or
  calendar entry — only past, already-ingested ones. Fabricating a next
  date would violate this repo's own never-fabricate posture throughout;
  this stays unbuilt until a real calendar source exists.
- **Resolving `speaker` strings to actual Person node ids.** Still the
  same deferred work [ADR-0040](0040-dated-source-ingestion.md) named;
  `last_interaction_at` is a best-effort string match, explicitly not a
  guaranteed identity link.
- **"Recent requests" as a distinct, separately-labeled section.** A
  request-type candidate promoted to an Obligation already appears in
  `relationship`'s existing grouping; a separate, differently-shaped
  section for the same underlying data is not added here.
- **Changing `GET /api/nodes?node_type=person`'s response shape for
  existing callers.** `needs_attention` is opt-in via a new query param;
  omitting it preserves today's exact behavior.

## Options considered

- **Filter by existence of a linked open/at-risk Obligation, plus two real
  derived detail fields (chosen):** reuses
  [ADR-0028](0028-person-relationship-view.md)'s existing join and
  [ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md)'s
  existing signals; adds exactly the two pieces of real, honestly-derivable
  memory the review named, and nothing fabricated.
- **Rank people by a computed "relationship health" score:** rejected —
  this repo's own [VISION.md](../VISION.md) DO-NOT list (quoted in
  [ADR-0039](0039-product-re-steer-primary-navigation.md)'s own drafting)
  explicitly rejects invented scores; a plain existence filter plus real
  counts stays honest.
- **Build a next-scheduled-conversation feature now:** rejected — no
  calendar source exists yet; this would be fabricated, not derived.

## Consequences

- **Positive:** People becomes "who needs something from me," the
  review's own framing, using only data that already exists — no new
  ingestion, no new schema.
- **Positive:** a real, if best-effort, answer to "when did I last
  actually hear from this person," reusing `occurred_at`
  ([ADR-0042](0042-occurred-at-retrieval-and-recall-sources-mcp-tool.md))
  for the first time in a user-facing view.
- **Negative / trade-off:** `last_interaction_at` can be wrong or absent
  when a name doesn't match a fragment's `speaker` string exactly (a
  known, named limitation, not silently hidden).
- **Risk:** low. Additive query param and detail fields; no schema change;
  no existing behavior changes when the new param/fields are unused.

## Exit criteria and evidence

Evidence: [EV-0051](../evidence.d/0051-relationship-workspace.md)

| Exit criterion | Evidence |
|---|---|
| `GET /api/nodes?node_type=person&needs_attention=true` returns only people with a linked open/at-risk Obligation | `person-list-filters-by-needs-attention` |
| Omitting `needs_attention` preserves today's exact response | `person-list-unchanged-without-filter` |
| Person detail includes `last_interaction_at`, derived from matching fragments' `occurred_at` | `person-detail-includes-last-interaction-at` |
| Person detail's `relationship` Obligations include `risk_signals` | `person-detail-relationship-includes-risk-signals` |
| The People tab defaults to the filtered view with a "Show everyone" toggle | `people-tab-defaults-to-needing-attention` |
