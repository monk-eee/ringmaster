# ADR-0081: Add an Actions lens to Graph Explorer's neighbourhood view

- **Status:** Accepted
- **Date:** 2026-08-18
- **Decider:** monk-eee
- **Approval:** Explicitly accepted by monk-eee on 2026-08-18 ("accept and build")
- **Depends on:** [ADR-0025](0025-node-edge-write-api-and-traversal.md), [ADR-0032](0032-temporal-edge-validity-supersede-on-write.md), [ADR-0033](0033-progressive-graph-traversal-trail.md)
- **Amends:** none — additive to ADR-0033, which named lenses as a real, deliberately deferred follow-up, not something ruled out
- **Tags:** architecture, frontend, ux, graph, traversal

## Context

[Relationship Memory and Progressive Graph Design](../RELATIONSHIP-GRAPH-DESIGN.md)
describes traversal "lenses" — opinionated policies that reduce a
neighbourhood to the question actually being asked, rather than an
undifferentiated list of everything connected to a node.
[ADR-0033](0033-progressive-graph-traversal-trail.md) built the trail
mechanism but explicitly left lenses, multi-hop depth, and ranked
path-finding as separate, future decisions, not included in that slice.

monk-eee's stated goal is an "intuitive scroll" through connected memory
specifically **to comprehend what needs doing** — not every relationship
type with equal weight. Graph Explorer's one-hop neighbourhood today
renders every neighbour (people, meetings, decisions, expectations,
outcomes, services, Obligations) the same way, which is honest but not
focused for that specific, named question.

The data this needs already exists in the same `GET /api/nodes/:id`
response [ADR-0025](0025-node-edge-write-api-and-traversal.md) already
returns: each neighbour is either a graph node (carrying a `node_type`)
or the polymorphic Obligation shape ADR-0025/ADR-0028 already resolve.
"What needs doing" is answerable today as a pure client-side filter over
data already fetched — no new backend route or query required.

## Decision

- Graph Explorer's node-detail panel gains a two-option lens control,
  **All** (default, current behavior unchanged) and **Actions**.
- When **Actions** is selected, the radial neighbourhood diagram, the
  relationship-count line, and the SVG rendering include only neighbours
  that are:
  - the polymorphic Obligation neighbour shape (`"type" in neighbor &&
    neighbor.type === "obligation"`), or
  - a graph-node neighbour whose `node_type === "risk"`.
- Neighbours the lens excludes are never fetched separately or hidden
  silently: the existing relationship-count line states both numbers
  honestly, e.g. `"3 shown, 9 filtered by Actions lens"` rather than
  just `"3 relationships."`
- The traversal trail, the "Why here" line, the node create/enrich/link
  forms, and the underlying one-hop `GET /api/nodes/:id` call are
  entirely unchanged. Pivoting into a neighbour (`visitNeighbor`) works
  identically whether or not a lens is active, including pivoting into a
  neighbour the current lens would otherwise filter out (selecting a
  node from the plain node list, or via an already-open trail step,
  still shows that node's own full neighbourhood next). Switching the
  lens never resets or truncates the current trail.
- Switching the lens re-evaluates the filter over the already-fetched
  `detail.neighbors` for the current focus; it does not issue a new
  network request.
- The lens choice is in-memory UI state only, matching ADR-0033's own
  trail-state precedent — no persistence, URL, or backend change.

## Scope

**In scope:** the two-option lens control; the client-side filter
predicate over `detail.neighbors`; the honest shown/filtered count line;
focused frontend/Playwright coverage proving the filter includes
Obligation and risk neighbours, excludes others, and that switching
lenses preserves trail position.

**Out of scope, named honestly:** the design's other lenses (People,
Meetings, Documents, Why); a backend query parameter or any change to
`GET /api/nodes/:id`'s response shape; multi-hop depth or ranked path
search (still [ADR-0033](0033-progressive-graph-traversal-trail.md)'s
named deferral, unchanged here); persisting the selected lens across
sessions or trail steps; applying a lens anywhere outside Graph Explorer
(Obligation Detail's and People's linked-node lists remain a separate,
not-yet-decided surface).

## Options considered

- **Client-side filter over the existing one-hop response (chosen):**
  zero new backend surface, provable and reversible inside one
  component, matches ADR-0033's own precedent for keeping this kind of
  decision small until real usage justifies more.
- **A `lens=actions` backend query parameter on `GET /api/nodes/:id`:**
  would let the database do the filtering, but repeats the query-
  language question ADR-0025 and ADR-0033 both deliberately deferred,
  for a filter a client can already perform correctly over data it
  already has.
- **A fixed node-type checkbox list instead of a named lens:** more
  flexible, but reintroduces the generic-database-browser feel ADR-0039
  moved away from; one named "Actions" toggle answers the one question
  actually asked.
- **Also add a "People" lens in the same record:** the design names six
  lenses; bundling more than the one actually requested repeats the
  scope-creep ADR-0033 explicitly warned against ("combining all of
  those into one implementation decision would be too broad to prove or
  safely reverse").

## Consequences

- **Positive:** directly answers monk-eee's stated need — pivoting
  through the graph specifically to see what's owed, at risk, or
  blocked — with a small, honest, additive change.
- **Positive:** no backend risk; the entire change is contained to
  `GraphExplorer.tsx` and its tests.
- **Neutral:** "Actions" is deliberately narrow (Obligation + risk node
  type only). If usage shows requests/decisions/blockers also belong,
  that is a follow-up ADR widening the predicate, not a silent edit to
  this one.
- **Negative:** a manager exploring a person's full context must
  remember to switch back to "All" to see meetings/decisions/
  expectations again — an honest usability trade-off of any opinionated
  default.
