# ADR-0096: Source review generalized beyond "meeting" — every ingested source type gets a browsable review UI

- **Status:** Accepted
- **Date:** 2026-08-19
- **Decider:** monk-eee
- **Approval:** Direct instruction ("look for gaps"), following a live,
  evidence-based audit of the running app and database that found the gap
  below — the same standing pattern this session has used for every prior
  self-initiated fix (ADR-0091/0093/0095).
- **Depends on:** [ADR-0036](0036-meeting-detail-read.md)/[ADR-0037](0037-meeting-scoped-candidate-listing.md)
  (the read routes this ADR generalizes), [ADR-0040](0040-dated-source-ingestion.md)
  (the any-source-type ingestion route that created this gap), [ADR-0043](0043-meeting-review-page.md)
  (the UI this ADR broadens)
- **Tags:** backend, frontend, product

## Context

A live audit of the running (non-test) database found: **zero `meeting`-type
nodes exist**, while 46 real, dated source documents exist across five
*other* source types — `1on1` (18), `note` (10), `comms` (8), `perspective`
(5), `connect` (5) — all ingested via `POST /api/sources/ingest`
([ADR-0040](0040-dated-source-ingestion.md)'s explicitly "any source type"
route).

But the only source-browsing UI in the entire app — the Meetings tab
(`MeetingReview.tsx`, [ADR-0043](0043-meeting-review-page.md)) — fetches
`GET /api/nodes?node_type=meeting` and its two supporting routes
(`GET /api/meetings/:id`, `GET /api/meetings/:id/candidates`,
[ADR-0036](0036-meeting-detail-read.md)/[ADR-0037](0037-meeting-scoped-candidate-listing.md))
explicitly 404 for any node whose `node_type != "meeting"` — a deliberate
choice at the time ("this route's contract is specifically a meeting, not
any node type"), made before ADR-0040 introduced free-text source types.

The result: the one place designed to let a manager read a source
document's transcript alongside its extracted candidates side by side
— exactly the workflow this repo exists to support — has zero real rows to
show, while 46 real documents are reachable only via the flat, ungrouped
Inbox candidate list or a lucky Search hit. This is the UI-side twin of the
concurrent session's own diagnosis in
[ADR-0094](0094-candidate-synthesis-pass.md) ("the extractions are not
granular enough... the linkages are not obvious") — both point at the same
underlying weakness: the review experience was built around one source
type that the real data has since moved past.

## Decision

- **Backend, `graph::list_nodes_filtered`**: add a `has_source_fragments:
  bool` filter — an `EXISTS` subquery against `source_fragments.source_id`
  — so "list every node that is a real ingested source" no longer requires
  naming a specific `node_type` string, and stays correct for any future
  source type ADR-0040 introduces without needing another ADR. `list_nodes`
  (the simpler, widely-used wrapper) is unchanged in signature — it always
  passes `false`, so every existing caller (the MCP `list_entities` tool,
  every other route, every existing test) is unaffected.
- **`GET /api/nodes?has_source_fragments=true`**: new, additive query
  param on the existing route, calling `list_nodes_filtered` directly.
  Omitting it preserves the route's exact prior behavior.
- **`GET /api/meetings/:id` / `GET /api/meetings/:id/candidates`**: the
  `node.node_type != "meeting"` gate is replaced with an existence check —
  "does at least one `source_fragments` row reference this id" — a 404
  still means "not a real ingested source," just no longer means
  "specifically a meeting." Route paths, request/response shapes, and
  every other behavior are unchanged.
- **Frontend, `MeetingReview.tsx`**: fetches `has_source_fragments=true`
  instead of `node_type="meeting"`. The tab label and empty-state/heading
  copy change from "Meetings"/"No meetings ingested yet" to
  "Sources"/"No sources ingested yet" — continuing to call a `connect`
  self-assessment or a `1on1` note a "meeting" would be a fabrication this
  repo's conventions refuse. Internal names (`fetchMeetingDetail`,
  `.meeting-review*` CSS classes, the `"meetings"` tab id) are **not**
  renamed — this is a user-facing copy fix, not a refactor, and renaming
  every internal identifier is unrequested churn with no user-visible
  benefit.

## Scope

**In scope:** `backend/src/graph/node.rs` (`list_nodes_filtered`'s new
filter), `backend/src/api/nodes.rs` (`?has_source_fragments=` query param),
`backend/src/api/ingestion.rs` (the two routes' relaxed existence check),
`frontend/src/api.ts` (`fetchNodes`'s new optional param),
`frontend/src/components/MeetingReview.tsx` (fetch call + user-facing
copy), `frontend/src/App.tsx` (the "Meetings" tab label), and the
Playwright assertions on that exact tab-label text.

**Out of scope, named honestly:**

- **Renaming internal identifiers** (`MeetingReview.tsx`,
  `fetchMeetingDetail`/`fetchMeetingCandidates`, `.meeting-review*` CSS
  classes, the `"meetings"` tab id in `Tab` type, the
  `GET /api/meetings/:id` route *path* itself). Only the text a user
  reads changes; a full rename is unrequested scope for what's fundamentally
  a data-coverage bug fix, not a rebrand.
- **The candidate-synthesis gap** ([ADR-0094](0094-candidate-synthesis-pass.md),
  a concurrent session's own, separate, already-accepted work) — this ADR
  does not touch `synthesis.rs`, the new migration, or that ADR's scope.
- **A generalized "source type" taxonomy/validation.** `node_type` for a
  dated source remains free text (ADR-0040's own decision, unchanged);
  this ADR does not introduce an enum or restrict what values are valid.
- **Deleting or migrating the now-empty `meeting` type.** Nothing about
  `node_type='meeting'` itself changes; a future real meeting ingestion
  would show up in the same generalized list exactly like today.

## Options considered

- **Existence-based filter over a fixed type allowlist (chosen):** an
  `EXISTS (SELECT 1 FROM source_fragments ...)` check is exactly the
  honest signal ("this is a real ingested source") and never needs
  updating when ADR-0040's free-text `source_type` gains a sixth, seventh,
  etc. value — a hardcoded `node_type IN ('meeting','1on1','note',...)`
  list would silently miss the next new type until someone remembered to
  add it here too.
- **A full rename of every internal identifier and route path:**
  rejected as disproportionate — the bug is that real data can't be
  browsed, not that a function name is imprecise; renaming
  `fetchMeetingDetail`/`/api/meetings/:id`/CSS classes changes nothing a
  user sees and multiplies the diff for no functional benefit.
- **Leave "Meetings" as the label and just broaden the type filter:**
  rejected — calling a `connect` self-assessment or a `1on1` note a
  "meeting" in the UI a user reads daily is the same kind of dishonest
  labeling this repo's conventions (e.g. ADR-0039's Candidates→Inbox
  rename) already reject.

## Consequences

- **Positive:** all 46 of the real, currently-unbrowsable source documents
  become reachable through the one UI built for exactly this — read the
  transcript, see its extracted candidates, trigger extraction, accept/
  reject/promote — without waiting on the synthesis pass or any other work.
- **Positive:** the fix is forward-compatible with any future `source_type`
  ADR-0040 permits, with zero further ADRs needed just to add a type to an
  allowlist.
- **Negative / trade-off:** the tab and route still say "Meetings"/
  `/api/meetings/` internally — a future contributor reading the route
  name without this ADR could reasonably assume it's meeting-specific;
  mitigated by this ADR's own doc comments at each changed call site.
- **Risk:** low. Additive backend filter (existing callers unaffected by
  construction), one relaxed existence check replacing a stricter one
  (strictly more permissive, never rejects what it used to accept), and a
  copy-only frontend change validated by the existing Playwright suite
  (with its tab-label assertion updated to match the intentional copy
  change).

## Exit criteria and evidence

Evidence: [EV-0096](../evidence.d/0096-generalize-source-review-beyond-meeting.md)

| Exit criterion | Evidence |
|---|---|
| `list_nodes_filtered` supports a `has_source_fragments` existence filter | `has-source-fragments-filter-exists` |
| `GET /api/nodes?has_source_fragments=true` is wired | `nodes-route-supports-filter` |
| The meeting detail/candidates routes accept any source-bearing node, not only `node_type='meeting'` | `meeting-routes-accept-any-source-type` |
| The Sources tab fetches by `has_source_fragments`, not `node_type=meeting` | `frontend-fetches-by-has-source-fragments` |
| The tab label and empty state read "Sources", not "Meetings" | `honest-sources-label` |
| The full Playwright suite passes with the updated tab-label assertion | `playwright-suite-passes-generalized-sources` |
