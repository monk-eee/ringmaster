# ADR-0080: Promote Graph Explorer to primary navigation

- **Status:** Accepted
- **Date:** 2026-08-18
- **Decider:** monk-eee
- **Approval:** Explicitly accepted by monk-eee on 2026-08-18 ("accept and build")
- **Depends on:** [ADR-0025](0025-node-edge-write-api-and-traversal.md), [ADR-0026](0026-graph-explorer-frontend.md), [ADR-0033](0033-progressive-graph-traversal-trail.md), [ADR-0039](0039-product-re-steer-primary-navigation.md)
- **Amends:** [ADR-0039](0039-product-re-steer-primary-navigation.md)'s "Secondary/developer navigation" grouping, narrowly for the Graph tab only — Obligations and Search stay demoted, unchanged, for the reasons ADR-0039 already gave
- **Tags:** architecture, frontend, ux, information-architecture, graph

## Context

[ADR-0039](0039-product-re-steer-primary-navigation.md) demoted Graph to a
secondary "Developer" tab because, at the time, it was a raw node/edge
administration console — a database browser, exactly what monk-eee's
re-steer said Ringmaster must not be.

That was true of Graph Explorer as it existed on 2026-08-14. It stopped
being fully true the same day: [ADR-0033](0033-progressive-graph-traversal-trail.md)
turned it into a progressive, pivotable exploration surface with a
persistent trail (`Roopa Venkat > attended > Weekly 1:1 > discussed >
Product Docs Archive`) and a deterministic "Why here" explanation — a
direct answer to one of ADR-0039's own six required questions (how
things are connected, why Ringmaster believes each claim), not a
database browser.

monk-eee has since named the actual cost of leaving it demoted: real
graph traversal — "next neighbour," pivoting from an obligation to its
owner to that owner's other commitments — is not discoverable from
anywhere a manager actually looks (Today, Obligation detail, People). It
sits fenced behind a "Developer" label most users would reasonably skip.
[ADR-0081](0081-graph-explorer-actions-lens.md), decided alongside this
record, adds a lens that filters that same traversal to exactly the
"what needs doing" question ADR-0039 prioritized. Together, the tab now
answers a primary management question; leaving it demoted after that
change would be the real inconsistency.

This ADR narrowly reverses one part of ADR-0039 with named justification
rather than silently re-editing it — Obligations and Search stay demoted:
they remain flat entity tables ADR-0039's original critique still
correctly describes.

## Decision

- `graph` moves from `SECONDARY_TABS` to the end of `PRIMARY_TABS` in
  `frontend/src/App.tsx`: **Today, Timeline, People, Inbox, Graph** —
  Today remains the default landing tab; the first four keep their
  existing relative order unchanged.
- The tab keeps its existing label ("Graph"), route, component
  (`GraphExplorer`), and every existing behavior, control, and test
  unchanged. This record governs tab placement only.
- The "Developer" label and visual de-emphasis (smaller, muted, after the
  divider) continue to apply to Obligations, Search, Meetings, and
  Activity exactly as ADR-0039 and later additions left them.

## Scope

**In scope:** moving one entry between the two arrays that already drive
tab grouping in `App.tsx`; the corresponding frontend/Playwright
navigation assertions that currently expect Graph in the secondary group.

**Out of scope:** any change to Graph Explorer's component, data, routes,
or the node/edge write forms it still exposes; renaming the tab; changing
the default landing tab; any change to Obligations/Search/Meetings/
Activity's grouping or presentation; the Actions lens itself
([ADR-0081](0081-graph-explorer-actions-lens.md), a separate decision).

## Options considered

- **Promote Graph only (chosen):** the narrowest reversal that matches
  the actual, named justification (traversal now answers a primary
  question); leaves ADR-0039's still-correct critique of Obligations/
  Search untouched.
- **Promote Graph, Obligations, and Search together:** would restore
  ADR-0039's original six-tab flat bar wholesale, discarding its
  information-architecture reasoning without a distinct justification
  for Obligations/Search, which did not change.
- **Leave Graph demoted; surface traversal only inside Obligation Detail/
  People instead:** smaller, and was the initial recommendation in
  conversation, but monk-eee's explicit direction was to promote the tab
  itself; that smaller slice remains available later as its own decision
  if the promoted tab still proves insufficient.
- **Rename "Graph" to something narrative (e.g. "Explore," "Connections")
  while promoting it:** a real option, but naming is separable from
  placement and was not requested; deferred rather than bundled in.

## Consequences

- **Positive:** the one surface that already answers "how is this
  connected, and why" becomes discoverable without a label suggesting
  it's for developers only.
- **Positive:** zero implementation risk — no route, schema, or
  component changes; the change is two array entries plus test
  expectations.
- **Negative / tension acknowledged:** this partially reverses monk-eee's
  own 2026-08-14 re-steer after four days, on direct instruction rather
  than discovered usage evidence — recorded honestly, not hidden. Future
  evidence should track whether the promoted tab actually gets used, not
  just that it renders.
- **Negative:** primary navigation grows from four tabs to five;
  `docs/current-status.md`'s existing narrow-screen horizontal-scroll
  behavior must still hold at five primary entries plus the secondary
  group — verified, not assumed, during implementation.
