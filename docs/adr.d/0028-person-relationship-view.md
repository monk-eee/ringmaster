# ADR-0028: Person relationship view — resolve linked Obligations into a per-person page

- **Status:** Proposed
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Pending — awaiting monk-eee's decision
- **Depends on:** [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md), [ADR-0020](0020-obligation-due-date-fields.md), [ADR-0022](0022-daily-brief-endpoint.md), [ADR-0023](0023-evidence-backed-daily-brief-reasons.md), [ADR-0025](0025-node-edge-write-api-and-traversal.md), [ADR-0026](0026-graph-explorer-frontend.md)
- **Amends:** [ADR-0025](0025-node-edge-write-api-and-traversal.md)'s neighbor-resolution scope for the specific case of an edge whose other end is an Obligation id, not the ADR as a whole
- **Tags:** architecture, api, frontend, data-model, graph

## Context

[VISION.md § Relationship pages as external memory](../VISION.md#relationship-pages-as-external-memory)
names this as monk-eee's own **#2 priority**, directly after the Daily
Brief: *"A manager spends most of their time managing relationships, not
entities... this view will be gold."* The stated reprioritized build order
is Daily Brief (shipped), **Relationship View**, Time Horizon, Congruence
Engine, Risk Engine, Candidate Validation, ADO integration, Automation.

[ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md) designed
`edges.from_id`/`to_id` to be polymorphic on purpose — either a `nodes.id`
or an Obligation's `obligation_id` — specifically so a Person node could
be linked to a real Obligation without a schema change. That capability
has sat unused: nothing creates such a link automatically today (no
owner/counterparty resolution exists in `extraction.rs`), and
[ADR-0025](0025-node-edge-write-api-and-traversal.md)'s own
`GET /api/nodes/:id` deliberately resolves an edge's other end **only**
against `nodes`, returning `neighbor: null` whenever it's actually an
Obligation id — proven by its own accepted test,
`node_detail_includes_neighbor_summary_and_handles_a_non_node_edge_target`.
That was the right, honest scope for ADR-0025 (a generic graph API), but
it means a Person node manually linked to their real Obligations today (via
[ADR-0025](0025-node-edge-write-api-and-traversal.md)'s own
`POST /api/edges`, already possible) shows those links as dead, blank
entries — the opposite of "external memory."

## Decision

- **Obligation-neighbor resolution (amends ADR-0025's scope, backend):**
  `GET /api/nodes/:id` additionally resolves any neighbor id that isn't
  found in `nodes` against `obligation_projection` (read-only
  `LEFT JOIN`, the same evidence-join pattern
  [ADR-0015](0015-expose-source-fragment-traceability-on-candidates.md)/
  [ADR-0023](0023-evidence-backed-daily-brief-reasons.md) already
  established). When it resolves, the neighbor gains `status`,
  `hard_due_at`, `soft_due_at`, and a `reason` string computed by the
  exact same `daily_brief_reason` function
  [ADR-0022](0022-daily-brief-endpoint.md)/[ADR-0023](0023-evidence-backed-daily-brief-reasons.md)
  already built — no new reasoning logic. A neighbor id that resolves
  against neither `nodes` nor `obligation_projection` (a genuinely unknown
  or stale id) still reports `neighbor: null`, unchanged from today.
- **Person grouping (backend):** when the requested node's `node_type` is
  `"person"`, the response additionally includes a `relationship` object
  grouping its resolved-Obligation neighbors into `at_risk` and `open`
  buckets (closed Obligations excluded, matching the Daily Brief's own
  convention), each ordered the same way the Daily Brief orders them
  (at-risk first, then soonest due date). Any node type other than
  `person` omits this field entirely — the plain neighbor list is
  unchanged for every other node type.
- **Frontend (Graph Explorer's detail panel,
  [ADR-0026](0026-graph-explorer-frontend.md)):** when the selected node's
  `node_type` is `"person"`, the detail panel renders a **Relationship
  view** above the existing generic SVG neighborhood: "Open Commitments"
  and "At Risk" groups, each entry showing the linked Obligation's status
  badge and `reason` text (the same presentation the Daily Brief tab
  already uses). Every other node type's detail panel is visually
  unchanged.

## Scope

**In scope:** resolving an Obligation-typed edge target on
`GET /api/nodes/:id`; a person-only `relationship` grouping in that same
response; a person-only "Relationship view" rendering in the Graph
Explorer's existing detail panel, reusing the Daily Brief's own status
badge/reason presentation.

**Out of scope, named honestly (all still vision, not this ADR):**
automatic owner/counterparty extraction linking a new Obligation to a
Person at creation time (extraction.rs is unchanged; a link is still only
created by a manual `POST /api/edges` call); a requests-vs-commitments
distinction (no `obligation_type` field exists on Obligation today, only
`status`); a "recent meetings" or "last interaction" timestamp (no
Meeting-to-Person edges with recency tracking exist); a Decisions section;
risks distinct from the existing `at_risk` status; a dedicated
`/relationships/:person_id` route or page separate from the existing Graph
Explorer tab (this reuses that surface rather than adding a new one).

## Options considered

- **Resolve on `GET /api/nodes/:id`, reuse `daily_brief_reason` (chosen):**
  no new reasoning logic, no new route, additive to an already-accepted
  response shape; the one deliberate behavior change (a previously-null
  neighbor now resolving) is narrowly scoped and named as an amendment.
- **A dedicated `GET /api/people/:id/relationships` route:** would keep
  Obligation-resolution logic out of the generic graph route entirely,
  but duplicates the neighbor-fetching/grouping logic this ADR can instead
  add once, and fragments "view a node" into two different routes
  depending on its type — rejected as unnecessary surface area for what's
  still a read-only enrichment of one existing response.
- **Wait for automatic owner/counterparty extraction first:** would make
  the view populate itself, but ties a UI-visible, user-requested #2
  priority to a materially larger, riskier change (a new field on the
  model-extraction schema) with no committed timeline — rejected; manual
  linking via the already-accepted `POST /api/edges` is a real, if
  unautomated, path to a populated relationship view today.

## Consequences

- **Positive:** closes the gap between ADR-0009's polymorphic edge design
  and an actual usable view of it; directly serves monk-eee's stated #2
  priority; reuses three already-accepted, already-proven building blocks
  (the evidence LEFT JOIN pattern, `daily_brief_reason`, the Graph
  Explorer's detail panel) rather than inventing new ones.
- **Negative / trade-off:** a Person's relationship view stays empty
  until something (today, only a manual `POST /api/edges` call) actually
  links their Obligations — this ADR makes the view work, not the linking
  automatic.
- **Breaking-ish change, named explicitly:** amends
  [ADR-0025](0025-node-edge-write-api-and-traversal.md)'s own accepted
  test expectation that an Obligation-typed edge target always reports
  `neighbor: null`. That test
  (`node_detail_includes_neighbor_summary_and_handles_a_non_node_edge_target`)
  must be updated, not just extended, as part of implementing this ADR —
  its still-valid assertion (a *truly* unknown id reports null) is
  preserved; only the Obligation-resolves-to-null part changes.
- **Risk:** low. Purely additive to the JSON shape for every non-person
  node; read-only for the person case (no new writes, no new mutation
  path); reuses existing, already-tested query and reasoning logic instead
  of adding new logic surface.

## Exit criteria and evidence

Evidence: [EV-0028](../evidence.d/0028-person-relationship-view.md)

| Exit criterion | Evidence |
|---|---|
| An edge whose target is a real Obligation resolves with its status/dates/reason, not null | `obligation-neighbor-resolves` |
| A genuinely unknown neighbor id still reports null (ADR-0025's original guarantee, preserved) | `unknown-neighbor-still-null` |
| A person node's response includes an `at_risk`/`open` grouped `relationship` object; other node types omit it | `person-relationship-grouping` |
| The Graph Explorer's detail panel renders a Relationship view for person nodes | `relationship-view-component-exists` |
