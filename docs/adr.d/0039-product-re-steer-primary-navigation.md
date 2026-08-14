# ADR-0039: Product re-steer — Today/Timeline/People/Inbox as primary navigation; entity-named surfaces demoted to secondary/developer

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Explicit direct instruction ("help me resteer it back to my original idea"), pasting the full re-steer brief this ADR implements, 2026-08-14
- **Depends on:** [ADR-0014](0014-react-vite-single-page-app.md), [ADR-0021](0021-ratify-search-tab-surfaced-without-its-own-adr.md), [ADR-0022](0022-daily-brief-endpoint.md), [ADR-0023](0023-evidence-backed-daily-brief-reasons.md), [ADR-0025](0025-node-edge-write-api-and-traversal.md), [ADR-0026](0026-graph-explorer-frontend.md), [ADR-0028](0028-person-relationship-view.md), [ADR-0029](0029-time-horizon-view.md), [ADR-0031](0031-suggested-focus-blocks.md), [ADR-0035](0035-time-horizon-timeline-view.md)
- **Amends:** [ADR-0014](0014-react-vite-single-page-app.md)'s flat, equally-weighted tab bar; [ADR-0021](0021-ratify-search-tab-surfaced-without-its-own-adr.md)'s and [ADR-0026](0026-graph-explorer-frontend.md)'s Search/Graph tab prominence (demoted, not removed); [ADR-0022](0022-daily-brief-endpoint.md)'s Daily Brief tab (absorbed into Today, not replaced — the route and reasoning are unchanged). Does not amend any backend route, schema, or the read-only/never-fabricate posture every one of those ADRs already established — this record governs navigation and page composition only.
- **Tags:** architecture, frontend, ux, information-architecture

## Context

monk-eee delivered a direct product re-steer: *"Ringmaster is a personal
management attention system, not a graph explorer, task tracker, transcript
browser, analytics dashboard, or generic CRUD app."* Six questions the app
must let a manager answer without visiting another tab: what deserves
attention today, what will become a problem soon, what's likely forgotten,
what's owed to particular people, which obligations belong together, and
why Ringmaster believes each claim. *"The UX is the product. Backend
entities are not the information architecture."*

Today's SPA ([ADR-0014](0014-react-vite-single-page-app.md)) has six flat,
equally-weighted tabs — Daily Brief, Obligations, Candidates, Search,
Graph, Time Horizon — added incrementally, one ADR at a time, each
individually well-scoped but never reconciled into one information
architecture. Several tabs are literally named after backend entities
(Obligations, Candidates) or implementation surfaces (Graph, Search) —
exactly what the re-steer calls out. This ADR reconciles that accumulated
structure into the requested one, spending no new backend capability to do
it: every primary surface below reads from a route that already exists and
is already proven.

## Decision

### Primary navigation

Four primary tabs, always visible, in this order: **Today** (default
landing tab), **Timeline**, **People**, **Inbox**.

### Secondary/developer navigation

The existing **Obligations**, **Graph**, and **Search** tabs continue to
exist, unchanged in behavior, but move to a visually de-emphasized second
group in the tab bar (smaller, muted, after a visual divider, under a
"Developer" label) rather than sharing equal weight with the four primary
tabs. Nothing is deleted; nothing loses a route or a test.

### The Today page (replaces the Daily Brief tab as the default landing view)

Reuses `GET /api/daily-brief` and `GET /api/focus-blocks` verbatim — same
requests already made today, no new endpoint. Renders, in this order:

1. **Greeting + bounded summary** — a short, deterministic sentence
   (`"N item(s) need your attention today."`, matching the Daily Brief's
   own existing summary line almost verbatim) — never a generated or
   model-written greeting.
2. **Ranked attention list, capped** — the existing Daily Brief ranking
   (at-risk first, then soonest due date), client-side capped to a fixed
   number of items (10), with a plain "N more in Timeline" link rather
   than silently truncating. Each row already carries, from data that
   already exists: a plain-language title (the linked evidence quote,
   [ADR-0030](0030-human-readable-titles-and-type-iconography.md)), why
   it matters now (the existing `reason` string), its hard/soft date, and
   evidence status (cited quote or "No evidence recorded" — never
   fabricated). See Scope for what this ADR does **not** yet add per item.
3. **"Do these together"** — the existing Suggested Focus Blocks card
   ([ADR-0031](0031-suggested-focus-blocks.md)), given an explicit section
   heading it does not have today, directly under the ranked list.
4. **Compact coming-soon** — a new, small client-side view over data
   `GET /api/time-horizon` already returns: the Next 7 Days and Next 30
   Days bucket counts and up to three item titles each, styled as a
   compact strip, not the full Buckets/Timeline page.
5. Deferred, named honestly below: a **"What changed"** section.

### Timeline (promotes the existing Time Horizon tab)

Same component, same route, same Buckets/Timeline toggle
([ADR-0029](0029-time-horizon-view.md)/[ADR-0035](0035-time-horizon-timeline-view.md)),
renamed in navigation from "Time Horizon" to "Timeline" and moved to
primary position. No component change beyond the label.

### People (new primary tab, thin composition over existing data)

A list of `node_type = "person"` nodes (`GET /api/nodes?node_type=person`,
already-accepted [ADR-0025](0025-node-edge-write-api-and-traversal.md)),
rendered as narrative cards (name, role/attributes already stored) instead
of the Graph Explorer's generic node table. Selecting a person calls the
existing `GET /api/nodes/:id` and renders its already-returned
`relationship` object (`at_risk`/`open` linked Obligations,
[ADR-0028](0028-person-relationship-view.md)) as a dedicated page — the
same data the Graph Explorer's node-detail panel shows today, given its
own first-class surface instead of requiring a detour through the Graph
tab. No new route; no new field.

### Inbox (reframes the existing Candidates tab)

Same route (`GET /api/candidates`), same accept/reject/promote actions
([ADR-0024](0024-candidate-accept-reject-buttons.md)/[ADR-0027](0027-promote-accepted-candidate-to-obligation.md)),
relabeled "Inbox" in navigation and framed as "things awaiting your
decision" rather than a generic entity table — the closest honest mapping
of an "Inbox" concept onto data that already exists, rather than
inventing a new one.

### Binding constraints on every surface touched by this ADR

- No vanity metrics, charts, velocity/burndown, large stat cards, or graph
  visualization on Today; no tabs named after a database table.
- No employee performance, sentiment, or morale scoring anywhere.
- No autonomous external action — every control remains a manager-invoked
  action against an existing route, never a scheduled or triggered one.
- Progressive disclosure: raw ids, event history, extraction confidence,
  edges, and embeddings stay out of the four primary surfaces' first
  view; they remain exactly where they are today, in the secondary/
  developer tabs.
- Visual direction: calm, spacious, one strong highlight color plus
  restrained semantic colors reusing the accent classes
  [ADR-0035](0035-time-horizon-timeline-view.md) already defined; red only
  for genuinely at-risk/overdue items (the existing `at_risk` status and
  Overdue bucket, never a new severity scale); narrative cards over dense
  tables on the four primary surfaces (the demoted Obligations/Graph/
  Search tabs keep their existing table-shaped presentation unchanged).
- Implementation constraint, restated as binding: reuse existing APIs and
  data wherever this ADR's scope allows; do not add a backend capability
  solely to decorate a screen; render an honest empty/unknown state
  rather than fabricate a description, date, reason, or relationship
  anywhere this ADR touches.

## Scope

**In scope:** the primary/secondary tab regrouping; the Today page's
greeting, capped ranked list, "Do these together" heading, and compact
coming-soon strip; the Timeline relabel; the new People list-and-detail
pages; the Inbox relabel/reframe of Candidates. All of it reuses existing
routes verbatim.

**Out of scope, named honestly (real gaps, not silently dropped):**

- **Per-item primary action, and review/correct/snooze/dismiss controls**
  named in the Today page spec. "Review" (navigate to detail) and the
  existing accept/reject/promote actions already exist for candidates,
  but Obligation has no `snoozed`/`dismissed` state or corrected-title
  flow today. Adding one is a genuine data-model decision (a new
  Obligation transition or field), not a navigation change — deferred to
  its own future ADR rather than bundled here.
- **Per-item related people/outcomes/services** on the Today ranked list.
  `GET /api/daily-brief` does not join against linked graph nodes today;
  doing so is a real, bounded backend addition this ADR does not include,
  to keep this record about information architecture, not a new API
  shape. A future ADR can extend the Daily Brief route the same way
  [ADR-0023](0023-evidence-backed-daily-brief-reasons.md) already did once.
- **"What changed" section.** Needs a decision about what "changed" means
  (newly at-risk since last view? a new status transition? a time
  window?) that this ADR does not make — a real design question, not an
  oversight, left for a follow-up ADR once a concrete definition exists.
- **A dedicated Raw Source Fragments or Extraction Metadata surface.**
  Neither exists as its own tab today (evidence text already appears
  inline in Obligations/Candidates/Daily Brief/Timeline rows); this ADR
  does not create new dedicated views for them, since none were asked
  for beyond being named as "secondary" if they existed.
- **Any visual redesign of the Obligations, Graph, or Search tabs**
  themselves. They move in the tab bar; their internals are untouched.
- **Any new backend route, migration, or dependency.** Everything above
  reads data at least one already-accepted, already-proven route already
  returns.

## Options considered

- **Reconcile navigation and compose Today from existing data (chosen):**
  directly answers the re-steer with zero new backend surface, reusing
  three already-proven routes (`daily-brief`, `focus-blocks`,
  `time-horizon`) and one already-proven node query, matching this
  session's own established precedent of shipping the smallest real
  slice and naming the rest honestly.
- **Design and build every Today-page requirement in one pass (per-item
  relationships, snooze/dismiss, What changed):** rejected as too broad
  to prove or safely reverse in one record — each of those is its own
  data-model decision this ADR would otherwise have to guess at, the same
  reasoning [ADR-0033](0033-progressive-graph-traversal-trail.md) used to
  split a larger design document into a foundational first slice.
- **Delete the Obligations/Graph/Search tabs outright instead of
  demoting them:** rejected — they remain genuinely useful developer/
  diagnostic surfaces (raw obligation list, node/edge authoring, semantic
  search) with their own accepted ADRs and tests; the re-steer asks for
  reduced prominence, not removal.
- **A new, separate `/api/today` aggregating route:** would let the
  frontend make one call instead of two, but adds backend surface the
  implementation constraint explicitly discourages when two already-
  proven calls already provide the data; revisit only if payload
  composition genuinely can't stay client-side as data volume grows.

## Consequences

- **Positive:** directly answers the re-steer's own success test — a
  first-time user should know what to deal with next without leaving the
  first screen — using only already-accepted, already-tested data paths.
- **Positive:** demoting rather than deleting the entity-named tabs keeps
  every existing test, route, and ADR's proof intact; this record changes
  presentation, not capability.
- **Negative / trade-off:** the Today page's per-item action set is
  genuinely thinner than the full spec (no snooze/dismiss/correct, no
  related-people join, no "What changed") until the named follow-up ADRs
  land — an honest, visible gap rather than a fabricated control that
  does nothing.
- **Risk:** low for the navigation/composition change itself (no backend
  change, every data source already proven); the real risk is scope
  creep back toward the full spec in one implementation pass, which the
  Scope section above is written to guard against.

## Exit criteria and evidence

Evidence: [EV-0039](../evidence.d/0039-product-re-steer-primary-navigation.md)

| Exit criterion | Evidence |
|---|---|
| Today/Timeline/People/Inbox render as the primary tab group, in that order, with Today the default | `primary-nav-order-and-default` |
| Obligations/Graph/Search render as a visually distinct secondary/developer group, not deleted | `secondary-nav-group-exists` |
| The Today page renders a greeting, the capped ranked list, a labeled "Do these together" section, and a compact coming-soon strip, in that order | `today-page-renders-required-sections` |
| The People tab lists person nodes and opens each into its existing relationship data, with no new backend route | `people-tab-lists-and-opens-relationship-data` |
| The Inbox tab is the relabeled Candidates route/actions, unchanged in behavior | `inbox-is-relabeled-candidates` |
| No new backend route, migration, or frontend dependency was added | `no-new-backend-or-dependency` |
