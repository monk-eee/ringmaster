# ADR-0035: Time Horizon timeline view — an alternative, zoomable presentation of the existing bucketed data

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Direct instruction ("turn into adr and build"), 2026-08-14
- **Depends on:** [ADR-0029](0029-time-horizon-view.md), [ADR-0030](0030-human-readable-titles-and-type-iconography.md)
- **Tags:** architecture, frontend, ux, time-horizon

## Context

[ADR-0029](0029-time-horizon-view.md) shipped the Future Risk Horizon as a
read-only route plus five stacked bucketed sections (Overdue, Next 7/30/90
days, Beyond), an honest first cut that explicitly deferred visual severity
and a true timeline presentation. [VISION.md § Timeline, not graph, not
table, not kanban](../VISION.md#timeline-not-graph-not-table-not-kanban----the-future-risk-horizon)
always named a timeline as the preferred eventual shape for this data.
monk-eee shared a concrete reference component (zoomable axis, colored
period bands, typed markers, a "Now" jump, a stacked-count badge, pan
arrows, a legend, an evidence popover), which
[TIME-HORIZON-TIMELINE-DESIGN.md](../TIME-HORIZON-TIMELINE-DESIGN.md) maps
onto Ringmaster's own Obligation and evidence data, then asked directly to
turn that mapping into an ADR and build it.

`GET /api/time-horizon` already returns every non-closed Obligation's
effective due date, status, and evidence-backed `reason`
([ADR-0029](0029-time-horizon-view.md)). No new backend route, schema
change, or dependency is required to render that same data along an axis
instead of stacked lists.

## Decision

- The existing **Time Horizon** tab gains a client-side view toggle,
  **Buckets** (unchanged, default) and **Timeline** (new). Both render the
  same `GET /api/time-horizon` response already fetched today; switching
  views performs no new network request.
- A new `TimeHorizonTimeline` component renders the same five bucket
  windows as horizontal bands, reusing the exact `accent-overdue` /
  `accent-next-7` / `accent-next-30` / `accent-next-90` / `accent-beyond`
  classes the Buckets view already defines — no new color vocabulary.
- Within a band, Obligations sharing the same effective due date (or
  sharing "no date recorded") collapse into one marker showing
  [ADR-0030](0030-human-readable-titles-and-type-iconography.md)'s
  existing Obligation glyph and a count badge when more than one item
  shares that date. Clicking a marker expands an inline list reusing the
  Daily Brief's own row presentation (status badge, id, evidence `reason`)
  — the same evidence text `GET /api/time-horizon` already returns, not a
  new call or a generated summary.
- **Pan:** prev/next controls move a single "focused" band across the five
  fixed windows; the focused band renders with more visual weight (wider
  flex share). This is a discrete, bounded pan across five known steps, not
  continuous drag.
- **Zoom:** a two-state toggle. Zoomed out (default) shows all five bands
  side by side. Zoomed in shows only the focused band, full width. There is
  no continuous/arbitrary zoom level.
- **Now:** resets focus to the first band (Overdue) and exits zoom, a
  deterministic reset rather than a real "scroll to today" computation.
- **Legend:** a toggle reveals a compact key reusing the same accent chip
  style the Buckets view's ribbon already uses, plus one line naming the
  marker glyph.
- **Close:** switching back to Buckets via the same two-state toggle is the
  close affordance; there is no separate modal or overlay to dismiss.
- No severity color (🔴🟠🟡🟢) is added — that depends on the Risk Engine,
  which does not exist yet, matching ADR-0029's own honest deferral.
- No congruence-based grouping is added — that depends on the Congruence
  Engine, which does not exist yet.
- No new npm dependency is introduced. The band/marker layout is plain
  flexbox and buttons, matching this frontend's established minimal-
  dependency, hand-rolled-visualization precedent
  ([ADR-0026](0026-graph-explorer-frontend.md)).
- View, zoom, pan-focus, and expanded-marker state are local component
  state only; nothing persists across a reload or navigation, the same
  posture [ADR-0033](0033-progressive-graph-traversal-trail.md) already
  established for the graph traversal trail.

## Scope

**In scope:** the Buckets/Timeline view toggle; the `TimeHorizonTimeline`
component; band rendering reusing existing accent classes; same-date
stacking with a count badge and click-to-expand evidence detail; discrete
pan-by-band-focus; two-state zoom; a legend toggle; a "Now" reset; new
CSS entirely additive to the existing stylesheet; focused Playwright
coverage.

**Out of scope:** any backend route or schema change (none needed);
continuous/arbitrary zoom or drag-to-pan; true proportional day-level axis
positioning within a band; severity color from the Risk Engine; congruence
banding from the Congruence Engine; candidate-type markers (this view only
ever shows Obligations, matching `GET /api/time-horizon`'s existing
response shape); persisted view/zoom/pan preferences; a dedicated route or
page separate from the existing Time Horizon tab.

## Options considered

- **Client-side view toggle over the existing route's response (chosen):**
  zero backend change, zero new dependency, reuses every existing accent
  class and the Daily Brief's row presentation; the smallest change that
  satisfies the direct ask.
- **A real, continuously zoomable/pannable date axis with day-level
  marker positioning:** closer to the literal reference component, but
  requires deciding axis math, drag physics, and rendering performance at
  arbitrary zoom — a materially larger, undecided design surface with no
  current data volume to justify it. The discrete five-band
  pan/two-state-zoom model captures the same affordances (Now, zoom,
  pan, legend, stacked count, evidence popover) without that open-ended
  scope.
- **A timeline/Gantt npm library:** would need a new frontend dependency
  in an environment where the public npm registry is unreliable from
  containers (recorded precedent), and repeats an option
  [ADR-0026](0026-graph-explorer-frontend.md) already rejected for the
  Graph Explorer for the same reason.
- **Replace the Buckets view outright:** rejected — the design document
  this ADR implements explicitly frames the timeline as an alternative
  presentation, not a forced replacement, and the Buckets view has no
  known problem that requires removing it.

## Consequences

- **Positive:** delivers the concrete timeline interaction monk-eee asked
  for, directly reusing already-accepted data, icons, and evidence text —
  no new reasoning logic anywhere.
- **Positive:** same-date stacking with an expandable count is a real
  usability improvement a flat bucketed list doesn't need, and becomes
  genuinely visible once several Obligations share a due date or lack one.
- **Negative / trade-off:** panning and zooming are deliberately coarse
  (five fixed bands, two zoom states) rather than a true continuous axis;
  a later ADR can revisit this once real usage shows it's insufficient.
- **Risk:** low. Purely additive frontend code and CSS; no new dependency;
  no backend or schema change; the existing Buckets view and its own
  evidence/tests are untouched.

## Exit criteria and evidence

Evidence: [EV-0035](../evidence.d/0035-time-horizon-timeline-view.md)

| Exit criterion | Evidence |
|---|---|
| A Buckets/Timeline view toggle exists on the Time Horizon tab | `timeline-view-toggle-exists` |
| The timeline renders the same five bands using the existing accent classes, not a new color scheme | `timeline-renders-bands-with-existing-accents` |
| Obligations sharing the same effective due date collapse into one marker with a count | `timeline-stacks-same-day-items-with-count` |
| Clicking a marker reveals its evidence-backed reason inline, reusing the existing row presentation | `timeline-marker-reveals-evidence-reason-on-click` |
| Pan-by-focus, two-state zoom, and a Now reset are all implemented | `timeline-supports-pan-focus-and-zoom-and-now-reset` |
| Focused browser coverage proves switching to Timeline, expanding a stack, and using Now/zoom/pan | `playwright-proves-timeline-interaction` |
