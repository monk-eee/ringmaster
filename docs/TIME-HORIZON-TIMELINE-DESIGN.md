# Ringmaster -- Time Horizon Timeline Design

**Status:** Working product-design intent, 2026-08-14. This document is not
an ADR and does not govern implementation. It extends
[VISION.md § Timeline, not graph, not table, not kanban](VISION.md#timeline-not-graph-not-table-not-kanban----the-future-risk-horizon).
The presentation described here is a possible future evolution of the
bucketed list [ADR-0029](adr.d/0029-time-horizon-view.md) already ships. It
does not amend that accepted decision; any real implementation needs its own
bounded ADR.

## Purpose

monk-eee shared a reference horizontal-timeline component: a zoomable axis,
colored period bands, typed event markers, a "Now" jump, a stacked-count
badge, pan arrows, a legend toggle, and an evidence popover. VISION.md
already names a timeline as the preferred view for the Future Risk Horizon;
[ADR-0029](adr.d/0029-time-horizon-view.md) shipped that intent as a
simpler, honestly-scoped bucketed list first. This document maps the
reference component's affordances onto Ringmaster's own data so a later,
bounded ADR can implement a genuine timeline without re-deriving the
interaction model from scratch.

## What changes and what doesn't

- The underlying data is unchanged. `GET /api/time-horizon`
  ([ADR-0029](adr.d/0029-time-horizon-view.md)) already returns every
  non-closed Obligation with its effective due date and evidence-backed
  `reason`. A first timeline version needs no new backend route.
- The existing bucket boundaries (Overdue, Next 7, Next 30, Next 90 days,
  Beyond) remain the meaningful reference points. A timeline renders them
  as background bands instead of stacked list sections, rather than
  inventing a new time model.
- Severity color and cross-obligation grouping stay out of scope until the
  Risk Engine and Congruence Engine — both still vision, per ADR-0029's own
  deferral note — exist to compute them honestly.

## Reference mapping

| Reference affordance | Ringmaster meaning |
|---|---|
| Timeline name | The current horizon's scope: "Future Risk Horizon", or a filtered view such as "Roopa's Horizon" or "Delivery Horizon". |
| Colored horizontal bands | The existing Overdue / Next 7 / Next 30 / Next 90 / Beyond windows, rendered as background segments instead of stacked sections. |
| Typed event markers (star, pin, square, pill) | Obligation/candidate type glyphs, reusing [ADR-0030](adr.d/0030-human-readable-titles-and-type-iconography.md)'s existing `typeIcon()` vocabulary rather than a second icon system. |
| Event details popover | The existing `reason` string and source-fragment evidence ([ADR-0023](adr.d/0023-evidence-backed-daily-brief-reasons.md)), shown on click instead of inline in a list row. |
| "Now" control | Recenters the visible window on today; distinguishes Overdue from what's genuinely upcoming. |
| Zoom in / zoom out | Changes visible granularity: zoomed in shows day-level placement for Overdue/Next 7 Days; zoomed out shows week/month placement for Next 90 Days/Beyond. |
| Stacked count badge | Multiple Obligations landing on the same point collapse into one marker with a count, expandable into the individual items. |
| Legend | Explains marker glyph and band meanings; grows honestly as severity/congruence signals are added later. |
| Pan arrows | Move the visible window earlier or later without changing zoom, e.g. reviewing recently-passed Overdue items or looking beyond 90 days. |
| Close | Collapses back to the existing bucketed list. The timeline is an alternative, richer presentation of the same data, not a forced replacement. |

## Interaction sketch

```text
Future Risk Horizon                                    [Now] [-] [+] [Legend]
◀  ── Overdue ──│──── Next 7 Days ────│─────── Next 30 Days ──────▶
        ⚠                  📋  📋⁽²⁾                  🗓
   Ownership risk     Transition plan   Connect cycle
   [click for evidence]
```

Clicking a marker opens the same evidence-backed detail already available
in the Daily Brief and bucketed Time Horizon, positioned near the marker
rather than requiring a page navigation.

## Honest gaps

- **No severity color today.** PRODUCT-SPEC.md's 🔴🟠🟡🟢 markers require
  the Risk Engine's actual signal, not a color chosen for the sake of the
  reference image.
- **No congruence-based banding today.** Grouping by shared person,
  service, or meeting is the Congruence Engine's job, not this view's.
- **No persistence of zoom/pan state is assumed.** This is presentation
  state, the same posture as the progressive graph trail
  ([RELATIONSHIP-GRAPH-DESIGN.md](RELATIONSHIP-GRAPH-DESIGN.md)).

## Non-goals

- Replacing the existing bucketed list — both can coexist.
- Inventing a new marker-glyph vocabulary parallel to ADR-0030's icons.
- Building the Risk Engine or Congruence Engine as part of this surface.
- A general-purpose timeline/Gantt component for non-Obligation data.

## Open design questions

1. Does zoom/pan state reset per session, or should it be sticky like a
   view preference?
2. Should the stacked-count badge expand inline or open the same evidence
   panel used for a single marker?
3. Which bucket boundaries deserve a visible band edge once real due dates
   cluster unevenly (e.g., very few overdue items, many at exactly 30 days)?
4. Should a person-scoped horizon (e.g., "Roopa's Horizon") reuse this same
   component, or is that better served by the Relationship page's own
   Past/Now/Next projection?
