# ADR-0033: Progressive graph traversal trail over one-hop neighborhoods

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Explicitly accepted by monk-eee on 2026-08-14 ("accept")
- **Depends on:** [ADR-0025](0025-node-edge-write-api-and-traversal.md), [ADR-0026](0026-graph-explorer-frontend.md), [ADR-0028](0028-person-relationship-view.md), [ADR-0032](0032-temporal-edge-validity-supersede-on-write.md)
- **Amends:** [ADR-0026](0026-graph-explorer-frontend.md)'s click-to-recenter interaction so the visited path remains visible; it does not amend [ADR-0025](0025-node-edge-write-api-and-traversal.md)'s one-hop API boundary
- **Tags:** architecture, frontend, graph, traversal, ux

## Context

[ADR-0025](0025-node-edge-write-api-and-traversal.md) deliberately chose a
one-hop node-detail API and deferred arbitrary multi-hop traversal.
[ADR-0026](0026-graph-explorer-frontend.md) built the corresponding radial
view: clicking a neighbour fetches its one-hop detail and re-centres the
diagram. That proves traversal is possible, but each click replaces the
previous context. The manager cannot see how they travelled from Roopa to a
1:1, from that meeting to a Product Docs Archive, and from the archive to a
decision or Obligation.

[Relationship Memory and Progressive Graph Design](../RELATIONSHIP-GRAPH-DESIGN.md)
now records the intended interaction: explore one meaningful increment at a
time, keep the route visible and reversible, and enrich the newly focused
node without dumping the entire organizational graph onto the screen. The
full design also describes configurable depth, ranked path finding, meeting
ingestion, and node-type-specific enrichment. Combining all of those into
one implementation decision would be too broad to prove or safely reverse.

This ADR chooses the foundational frontend slice: compose repeated calls to
the already-accepted one-hop endpoint into a durable in-view traversal trail.

## Decision

- The existing Graph Explorer keeps a client-side **traversal trail** for the
  current exploration. Each step contains the visited node summary and, for
  every step after the first, the edge used to arrive there: edge id, type,
  direction, confidence, and temporal validity already exposed by
  [ADR-0032](0032-temporal-edge-validity-supersede-on-write.md).
- Selecting a node from the node list starts a new trail with that node as
  its root. Clicking a neighbour in the existing radial diagram appends that
  neighbour and connecting edge, then loads it through the unchanged
  `GET /api/nodes/:id` one-hop endpoint and makes it the current focus.
- The trail renders as an accessible, horizontally scrollable path above the
  radial neighbourhood. It uses human-readable node labels and relationship
  verbs, for example:

  ```text
  Roopa Venkat > attended > Weekly 1:1 > discussed > Product Docs Archive
  ```

  Node steps are buttons. Relationship steps are labels, not controls.
- Selecting an earlier node in the trail truncates later steps and restores
  that node as the current focus. A dedicated Back control performs the same
  operation by one step and is disabled at the root. This gives the
  exploration browser-like, reversible navigation without introducing a
  separate routing system.
- If a neighbour already exists earlier in the trail, selecting it returns
  to that occurrence and truncates the later branch instead of creating an
  endlessly repeating cycle. The underlying graph remains cyclic; this rule
  governs only the navigation history.
- The current detail panel gains a short deterministic **Why here** line for
  non-root steps, derived only from the previous node and traversed edge. It
  does not invoke a model or claim context not present in the graph.
- The existing one-hop radial diagram remains the only neighbourhood rendered
  around the current focus. Previously visited nodes remain visible through
  the trail, not by accumulating every fetched neighbour into an unbounded
  canvas.
- Suggested or historical edges retain the trust treatment established by
  their source decision: the trail marks non-null `valid_to` edges as
  historical and confidence-bearing/suggested edges with text as well as
  visual treatment. Temporal or epistemic state is never communicated by
  colour alone.
- Trail state is in-memory for this slice. Refreshing the page, leaving the
  Graph tab, or starting from another list item may begin a new exploration;
  no URL, database, or cross-session persistence is introduced.
- No backend route, schema, package, or graph-visualization dependency is
  added. The frontend composes the existing one-hop primitive instead of
  weakening ADR-0025's explicit rejection of a premature generic graph-query
  language.

## Scope

**In scope:** client-side traversal state; starting, appending, truncating,
and stepping back through a trail; a visible path with relationship labels;
the deterministic "Why here" line; preserving existing temporal and trust
states in the trail; focused frontend and Playwright coverage.

**Out of scope:** a backend multi-hop endpoint or graph-query language;
automatic radius expansion to 2, 3, or 10 hops; ranked path search between
distant nodes; retaining all fetched neighbourhoods on one canvas; pan,
zoom, force-directed layout, or a visualization dependency; persisted or
shareable trails; meeting batch ingestion through MCP/CLI; identity
resolution; automatic links from extracted candidates to Person/Meeting
nodes; generated node summaries; specialized Meeting, Document, or Archive
detail layouts. Those are follow-up decisions described by the design, not
silently included here.

## Options considered

- **Client-composed trail over repeated one-hop reads (chosen):** directly
  creates the one-increment-at-a-time interaction requested by monk-eee,
  preserves the existing API and SVG, has bounded state, and can be proven
  with frontend tests before real graph scale justifies a deeper query
  design.
- **Add a configurable backend multi-hop endpoint now:** would support
  depth 3 or 10 in one request, but immediately requires decisions about
  cycle handling, branch limits, ranking, authorization across paths, payload
  size, and query cost. The product does not yet have evidence for those
  choices; defer until the progressive trail reveals real path-finding needs.
- **Accumulate every visited neighbourhood into one growing SVG:** keeps more
  context spatially visible, but the hand-rolled radial layout in ADR-0026
  has no collision, pan/zoom, or large-graph strategy. It would turn a
  predictable one-hop diagram into an unstable canvas before those problems
  are deliberately designed.
- **Adopt a graph visualization library:** could solve layout and interaction
  at larger scale, but repeats an option ADR-0026 rejected and adds dependency
  and design surface before this smaller interaction has been validated.
- **Keep destructive click-to-recenter navigation unchanged:** technically
  traversable, but loses the route and does not satisfy the product intent of
  moving through organizational memory without losing context.

## Consequences

- **Positive:** a manager can move from Roopa to a meeting to a document or
  decision while always seeing how the current focus relates to the starting
  person.
- **Positive:** the implementation exercises real multi-step use without
  committing the backend to an arbitrary multi-hop query contract.
- **Positive:** "Why here" is deterministic and evidence-shaped; it cannot
  hallucinate a reason beyond the traversed edge.
- **Negative / trade-off:** the trail preserves the chosen route, not every
  branch inspected or every neighbour fetched. It is navigation history, not
  a complete multi-hop graph rendering.
- **Negative / trade-off:** refresh and cross-tab navigation may clear the
  trail. Persistence remains a separate design decision.
- **Risk:** a long trail can overflow horizontally; the path must scroll and
  keep the current step reachable rather than resizing the page or truncating
  labels into ambiguity.
- **Risk:** free-text edge types may read awkwardly as relationship verbs.
  This ADR renders the stored value honestly and does not invent the edge
  vocabulary decision that ADR-0009 deliberately deferred.

## Exit criteria and evidence

Evidence: [EV-0033](../evidence.d/0033-progressive-graph-traversal-trail.md)

| Exit criterion | Evidence |
|---|---|
| Selecting a list node starts a trail and clicking a neighbour appends its node and connecting relationship | `trail-starts-and-appends` |
| The trail visibly renders human-readable nodes and relationship labels | `trail-renders-readable-path` |
| Back and earlier-step selection truncate the trail and restore that node as current focus | `trail-navigation-is-reversible` |
| A non-root focus explains its immediate path through a deterministic "Why here" line | `focused-node-explains-why-here` |
| Traversal continues to use the existing one-hop node-detail API with no backend or dependency change | `one-hop-api-remains-boundary` |
| Focused browser coverage proves a user can traverse at least two edges and return to the root | `playwright-proves-multi-step-traversal` |
