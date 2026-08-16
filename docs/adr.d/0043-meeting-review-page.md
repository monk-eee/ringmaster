# ADR-0043: Meeting Review page — transcript fragments with inline extracted candidates

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Direct instruction ("yes"), confirming the largest named gap in this session's own gap analysis, 2026-08-17
- **Depends on:** [ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md), [ADR-0024](0024-candidate-accept-reject-buttons.md), [ADR-0025](0025-node-edge-write-api-and-traversal.md), [ADR-0027](0027-promote-accepted-candidate-to-obligation.md), [ADR-0036](0036-meeting-detail-read.md), [ADR-0037](0037-meeting-scoped-candidate-listing.md), [ADR-0039](0039-product-re-steer-primary-navigation.md)
- **Tags:** architecture, frontend, meeting, extraction, ux

## Context

[ADR-0034](0034-http-meeting-transcript-ingestion.md),
[ADR-0036](0036-meeting-detail-read.md), and
[ADR-0037](0037-meeting-scoped-candidate-listing.md) built a complete
backend path for one meeting: ingest it, read it back with its ordered
transcript fragments, and list those fragments with whatever candidates
were already extracted from each, plus fragment-level extraction progress.
Nothing in the frontend calls any of it. A meeting can be loaded through
the API, CLI, or MCP tool
([ADR-0040](0040-dated-source-ingestion.md)), but cannot be viewed,
reviewed, or acted on anywhere in the browser — named as this session's own
largest gap between what's built and
[MEETING-REVIEW-DESIGN.md](../MEETING-REVIEW-DESIGN.md)'s described
experience.

[MEETING-REVIEW-DESIGN.md](../MEETING-REVIEW-DESIGN.md) describes a fuller
review experience (claim bundles, correction, merge, split, defer, an
accepted-memory preview) than one record can responsibly build and prove at
once — the same reasoning
[ADR-0033](0033-progressive-graph-traversal-trail.md) already used to split
that document's graph-traversal half into a foundational first slice. This
ADR is the equivalent foundational slice for meeting review: put the
transcript and its extracted candidates in front of a person, using data
`GET /api/meetings/:id/candidates` already returns pre-grouped by fragment,
and reuse every validation control that already exists.

## Decision

- A new **Meetings** tab joins the existing secondary/developer group
  (Obligations, Search, Graph) established by
  [ADR-0039](0039-product-re-steer-primary-navigation.md) — same visual
  treatment, same reasoning: a real, useful, but diagnostic-shaped surface,
  not one of the four primary destinations.
- **Meeting list:** `GET /api/nodes?node_type=meeting`
  ([ADR-0025](0025-node-edge-write-api-and-traversal.md), already used
  elsewhere for People). Each entry shows its title and, when present, the
  `occurred_at`/`date` already stored in its attributes
  ([ADR-0040](0040-dated-source-ingestion.md)/[ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md)).
- **Meeting detail:** selecting a meeting calls
  `GET /api/meetings/:id` ([ADR-0036](0036-meeting-detail-read.md)) for its
  title/attributes, and `GET /api/meetings/:id/candidates`
  ([ADR-0037](0037-meeting-scoped-candidate-listing.md)) for its ordered
  fragments, each fragment's already-extracted candidates, and the
  extraction-progress summary. Both routes are read-only and already
  proven; this ADR adds no backend route.
- **Evidence beside interpretation, without a synchronized two-pane
  build:** each fragment renders in transcript order with its speaker and
  text, and directly beneath it, the candidates
  `GET /api/meetings/:id/candidates` already nests under that fragment.
  This achieves the design document's "evidence and interpretation
  together" goal for free, because the API already groups candidates by
  their originating fragment — no separate scroll-synchronization
  mechanism is designed or built here.
- **Reuses existing validation controls exactly:** each rendered candidate
  gets the same accept/reject/promote actions
  ([ADR-0024](0024-candidate-accept-reject-buttons.md)/[ADR-0027](0027-promote-accepted-candidate-to-obligation.md))
  already used by the Inbox/Candidates tab, calling the identical
  `POST /api/candidates/:id/accept|reject|promote` routes. No new
  validation state, transition, or control is invented.
- **Wires up the extraction trigger for the first time:** a fragment with
  no candidates yet shows an honest "No candidates extracted from this
  passage yet" state plus an **Extract** button calling the existing
  `POST /api/source-fragments/:id/extract`
  ([ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md)),
  which has had no frontend caller until now. Its existing `201`/`204`/`503`
  responses render as, respectively: the new candidate (after refetching),
  a plain "nothing worth extracting" message, or "No model configured" —
  never a fabricated result.
- After any accept/reject/promote/extract action, the view refetches
  `GET /api/meetings/:id/candidates`, the same refetch-on-change pattern
  `CandidatesTable.tsx` already uses, so validation state and extraction
  progress stay current.

## Scope

**In scope:** the Meetings tab; the meeting list; the meeting detail view
composing two already-proven read routes; per-fragment candidate
rendering with the three existing validation actions; wiring the existing
per-fragment extraction trigger into a frontend surface for the first
time; refetch-on-change.

**Out of scope, named honestly (real gaps, deferred to
[MEETING-REVIEW-DESIGN.md](../MEETING-REVIEW-DESIGN.md)'s later slices):**
correction, merge, split, or defer controls on a candidate (slice 6);
an accepted-memory preview showing where a validated item now surfaces
(slice 7); creating or ingesting a meeting from this page — ingestion
stays an API/CLI/MCP action
([ADR-0034](0034-http-meeting-transcript-ingestion.md)/[ADR-0040](0040-dated-source-ingestion.md));
batch/bulk extraction across every pending fragment in one action; a
synchronized-scroll two-pane transcript viewer; pagination for very large
transcripts; any new backend route, schema change, or dependency.

## Options considered

- **Inline per-fragment candidates over two existing read routes
  (chosen):** delivers the actual review need (see evidence beside the
  claims it produced) with zero new backend surface, since
  `GET /api/meetings/:id/candidates` already returns exactly this
  grouping; reuses every existing validation action and status
  presentation verbatim.
- **Build a true split-pane transcript/proposal view with synchronized
  highlighting**, per the fuller mockup in
  [MEETING-REVIEW-DESIGN.md](../MEETING-REVIEW-DESIGN.md): closer to that
  document's original sketch, but adds real scroll-sync/selection-state
  design and implementation risk this session's own "smallest real slice"
  precedent argues against taking in one step; the inline layout satisfies
  the same underlying need (evidence next to interpretation) with data
  already shaped for it.
- **Add the Meetings tab as a fifth primary destination instead of
  secondary:** rejected — [ADR-0039](0039-product-re-steer-primary-navigation.md)
  deliberately kept exactly four primary, question-shaped destinations;
  a meeting-by-meeting review queue is a diagnostic/working surface, the
  same category as the existing Obligations/Search/Graph group.
- **Leave the extraction trigger unwired, list only already-extracted
  candidates:** rejected — a fragment with nothing extracted yet would be
  a dead end with no visible way to change that, even though the route to
  do so already exists and is already proven.

## Consequences

- **Positive:** closes this session's own largest named gap — a meeting
  can now be ingested, viewed, and its candidates validated end to end
  through the browser, not only through the API/CLI/MCP surfaces.
- **Positive:** zero new backend route, schema, or dependency; every
  action this page performs already existed and was already tested.
- **Negative / trade-off:** this is not yet the fuller claim-bundle review
  experience [MEETING-REVIEW-DESIGN.md](../MEETING-REVIEW-DESIGN.md)
  describes — correction, merge, split, defer, and the accepted-memory
  preview remain real, separate, named follow-ups.
- **Risk:** low. Purely additive frontend code reusing already-proven
  routes and components; no change to any existing route's behavior.

## Exit criteria and evidence

Evidence: [EV-0043](../evidence.d/0043-meeting-review-page.md)

| Exit criterion | Evidence |
|---|---|
| A Meetings tab exists in the secondary/developer group | `meetings-tab-exists` |
| Selecting a meeting renders its fragments with any already-extracted candidates inline | `meeting-detail-renders-fragments-with-candidates` |
| Accept/reject/promote on a candidate in this view call the existing routes and refetch | `meeting-review-reuses-existing-validation-actions` |
| A fragment with no candidates offers an Extract action calling the existing per-fragment trigger | `meeting-review-wires-up-extraction-trigger` |
| Focused browser coverage proves viewing a meeting and triggering extraction on a fragment | `playwright-proves-meeting-review-flow` |
