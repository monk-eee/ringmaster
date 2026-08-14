# ADR-0026: Graph explorer frontend — data entry, drill-down, and relationship visualization

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Explicitly accepted by monk-eee on 2026-08-14
- **Depends on:** [ADR-0014](0014-react-vite-single-page-app.md), [ADR-0021](0021-ratify-search-tab-surfaced-without-its-own-adr.md), [ADR-0025](0025-node-edge-write-api-and-traversal.md)
- **Tags:** architecture, frontend, graph, ux

## Context

[ADR-0025](0025-node-edge-write-api-and-traversal.md) gives the graph
substrate its first API surface, but an API alone does not satisfy
monk-eee's direct ask: enter data, drill into information, see visual
relationships, and traverse the graph, with nodes enrichable. Nothing in
the SPA today shows a node, an edge, or a relationship of any kind. This
ADR is the consuming frontend slice, following the same backend-then-
frontend sequencing already used for search
([ADR-0018](0018-generate-and-store-source-fragment-embeddings.md) →
[ADR-0019](0019-semantic-search-over-source-fragments.md)) and the Daily
Brief ([ADR-0020](0020-obligation-due-date-fields.md) →
[ADR-0022](0022-daily-brief-endpoint.md)).

[ADR-0021](0021-ratify-search-tab-surfaced-without-its-own-adr.md)
already established that a presentational tab consuming an already-
accepted, already-read-only route needs no separate ADR of its own; this
record is broader than that narrow exemption on purpose, because it also
introduces the SPA's first write/data-entry UI and its first visual
(SVG) rendering, neither of which existed before.

## Decision

- A new **Graph** tab is added to the existing tab set (Obligations /
  Candidates / Search / Graph).
- **Node list:** a table of nodes from [ADR-0025](0025-node-edge-write-api-and-traversal.md)'s
  `GET /api/nodes`, with a client-side `node_type` filter (matching the
  existing Obligations tab's client-side status-filter convention).
- **Data entry:** a form to create a node (`node_type` as a free-text
  input seeded with [docs/PRODUCT-SPEC.md §5.2](../PRODUCT-SPEC.md#52-core-node-types)'s
  named types as suggestions, `canonical_text`, and `attributes` entered
  as raw JSON text). No per-node-type structured schema/validation is
  built — every node type shares the same generic attribute bag today,
  matching [ADR-0025](0025-node-edge-write-api-and-traversal.md)'s own
  scope.
- **Drill-down:** selecting a node from the list calls
  `GET /api/nodes/:id` and shows a detail panel: `canonical_text`,
  `lifecycle_state`, `attributes` rendered as key/value pairs, and an
  **enrich** control (edit `canonical_text`/`attributes`, calling
  `PATCH /api/nodes/:id`).
- **Visual relationships and traversal:** the detail panel also renders
  the selected node's one-hop neighborhood (already returned by
  `GET /api/nodes/:id`) as a plain, hand-rolled SVG radial diagram: the
  selected node centered, each neighbor placed on a circle around it at
  an even angular spacing, connected by a line labeled with its
  `edge_type`. Clicking a neighbor re-centers the view on it (a new
  `GET /api/nodes/:id` call), which is the traversal behavior requested.
  No graphing/visualization library is added — plain SVG with basic
  trigonometry for node placement, consistent with this frontend's
  minimal-dependency footprint to date (a real, previously-hit constraint
  in this environment: this repository's own container tooling cannot
  reliably reach the public npm registry, recorded in repo memory).
- **Add relationship:** a minimal control on the detail panel to create
  an edge from the selected node to another node (picked from the node
  list) with a free-text `edge_type`, calling
  [ADR-0025](0025-node-edge-write-api-and-traversal.md)'s
  `POST /api/edges`.

## Scope

**In scope:** the Graph tab; node list with a client-side type filter;
node create and enrich forms; node detail drill-down; a one-hop SVG
relationship visualization with click-to-recenter traversal; edge
creation from the detail panel.

**Out of scope:** any node-type-specific structured form or validation;
multi-hop visualization or graph layout beyond one hop (no force-directed
physics, no pan/zoom); editing or deleting edges; deduplication/entity-
resolution UI; a dedicated visualization library dependency; Playwright
coverage beyond the existing structural-interaction pattern already used
for the other tabs (added here, not a new testing decision).

## Options considered

- **Hand-rolled SVG one-hop radial view (chosen):** directly answers
  "visual relationships" and "traverse the graph" with no new dependency
  and a bounded, easily-reasoned-about layout (a circle of known radius),
  appropriate for the one-hop scope
  [ADR-0025](0025-node-edge-write-api-and-traversal.md) itself returns.
- **Adopt a graph-visualization library (e.g. a force-directed layout
  package):** would scale better to larger, multi-hop graphs, but adds a
  new frontend dependency and a materially larger design surface (layout
  physics, zoom/pan, performance at scale) that nothing today's data
  volume or one-hop API justifies; revisit if/when multi-hop traversal is
  itself decided.
- **Text-only relationship list, no visual rendering:** simplest, but
  does not satisfy the explicit ask for "visual relationships" — this
  ADR's whole reason for existing.

## Consequences

- **Positive:** for the first time, a person can create, browse, enrich,
  and visually traverse the graph substrate
  [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md) built
  without ever exposing; closes the gap between an existing, tested
  schema and an actually usable product surface.
- **Negative / trade-off:** the visualization is deliberately basic (one
  hop, circular layout, no physics) and the data-entry form is generic
  (raw JSON attributes, no per-type schema) — both explicit, bounded
  trade-offs rather than gaps discovered later.
- **Risk:** low — purely additive to the existing SPA; the raw-JSON
  attributes field can accept malformed JSON, which is handled as a
  client-side validation error before any request is sent, never sent
  as-is to the backend.

## Exit criteria and evidence

Evidence: [EV-0026](../evidence.d/0026-graph-explorer-frontend.md)

| Exit criterion | Evidence |
|---|---|
| A Graph tab exists alongside the other three | `graph-tab-exists` |
| The tab can create a node and lists existing nodes | `node-create-and-list-exist` |
| Selecting a node shows its detail, attributes, and one-hop neighbors | `node-detail-component-exists` |
| The neighborhood is rendered as an SVG relationship diagram | `svg-relationship-view-exists` |
| Clicking a neighbor re-centers the view on it (traversal) | `traversal-recenter-exists` |
