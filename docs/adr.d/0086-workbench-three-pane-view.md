# ADR-0086: Workbench — a three-pane, no-navigation view over already-built data

- **Status:** Accepted
- **Date:** 2026-08-19
- **Decider:** monk-eee
- **Approval:** Direct instruction ("take it on"), continuing this session's established practice of drafting and implementing the next item from `docs/IMPROVEMENT-PLAN.md`'s suggested order (§2.3), 2026-08-19
- **Depends on:** [ADR-0022](0022-daily-brief-endpoint.md), [ADR-0039](0039-product-re-steer-primary-navigation.md), [ADR-0047](0047-obligation-detail-page.md), [ADR-0080](0080-promote-graph-explorer-to-primary-navigation.md), [ADR-0083](0083-meeting-brief-generation.md)
- **Tags:** frontend, ux

## Context

`docs/IMPROVEMENT-PLAN.md` §2.3 names this "the largest frontend change in
this plan" and says it "should be scoped as its own ADR rather than
folded into 2.1 or 2.2" — this record is that ADR. `docs/VISION.md`'s
"manager's workbench, not a dashboard" section describes three panes:

| Pane | Content |
|---|---|
| Left — Attention | Needs attention now / soon / recently changed. |
| Centre — Current focus | The selected item's owners, risks, evidence, related commitments. |
| Right — Relationship context | The relevant person: open commitments, recent asks, next scheduled conversation. |

"selecting an item in the left pane fills the centre pane with its full
context... without a page navigation" — replacing today's tab-based
click-through-to-a-separate-page flow with a persistent three-column
layout.

Nearly every piece of data this needs already exists as a proven,
composed read: the left pane is `GET /api/daily-brief`
([ADR-0022](0022-daily-brief-endpoint.md)), already rendered by the
existing `DailyBrief` component; the centre pane is `GET
/api/obligations/:id` ([ADR-0047](0047-obligation-detail-page.md)),
already rendered by the existing `ObligationDetail` component; the right
pane is almost exactly `GET /api/people/:id/brief`
([ADR-0083](0083-meeting-brief-generation.md)), which already returns
"open commitments" and "recent asks" — the literal two labels this
section's table asks for. The one field the table names that has no real
source is "next scheduled conversation" — no calendar/future-meeting
source exists, the same honestly-refused gap
[ADR-0051](0051-relationship-workspace.md) already named for Person
detail; not fabricated here either.

Given the scale the plan itself flags, this record ships the workbench as
a **new, additional destination**, not a replacement of Today's existing
list-and-navigate flow — matching the exact precedent
[ADR-0026](0026-graph-explorer-frontend.md)→[ADR-0080](0080-promote-graph-explorer-to-primary-navigation.md)
already set (ship a feature secondary/opt-in first, promote it later once
proven, in its own separate decision). Today's existing behavior, its
Playwright coverage, and `ObligationDetail`/`DailyBrief`'s existing
components are **not modified** by this record — reused exactly as they
already work.

## Decision

- **A new "Workbench" tab** in the secondary/"Developer" group (matching
  where Graph Explorer started, ADR-0026), composing three already-proven
  reads with zero new backend routes.
- **Left pane — Attention:** the existing `DailyBrief` component, unchanged,
  rendering the same capped ranked list Today already shows. Selecting a
  row sets local `selectedId` state instead of navigating to a page.
- **Centre pane — Current focus:** the existing `ObligationDetail`
  component, unchanged, rendered in place for the selected id. Its
  existing "← Back" button clears the selection (fits a workbench exactly
  as well as a page-back action — a to-Do list of one meaning, not two
  different behaviors bolted together).
- **Right pane — Relationship context:** a new `PersonBriefPanel`
  component. Once a centre-pane obligation loads, this record fetches
  that same `GET /api/obligations/:id` response a second time
  independently (a small, already-cheap single-row read) to find its
  `owns` edge to a `person` node — the exact lookup `ObligationDetail`
  itself already performs internally, just not lifted to a shared parent
  state, so `ObligationDetail` needs zero changes. That person id calls
  `GET /api/people/:id/brief` and renders `open_commitments`/
  `recent_asks` with source citations. No linked person, or nothing
  selected, is an honest empty state — never a fabricated relationship.
- **Urgency indication reuses the existing `StatusBadge` component**
  verbatim (already used by `DailyBrief`/`ObligationDetail`) rather than
  inventing new color/emoji semantics `VISION.md`'s mockup shows but no
  other surface in this app uses.

## Scope

**In scope:** a new Workbench tab; a three-pane CSS layout; the new
`PersonBriefPanel` component; wiring `fetchPersonBrief` in `api.ts`.

**Out of scope, named honestly:** replacing or modifying Today's existing
list-and-navigate flow, `DailyBrief`, or `ObligationDetail` (all reused
as-is, unchanged); promoting Workbench to primary navigation (a later,
separate decision once this is proven, matching ADR-0080's precedent for
Graph Explorer); "next scheduled conversation" (no real data source,
matches ADR-0051); color/emoji-coded urgency (reuses `StatusBadge`
instead); Focus Blocks/"Do these together" integration into the
workbench (a separate composition question); any change to
`GET /api/daily-brief`, `GET /api/obligations/:id`, or
`GET /api/people/:id/brief` (all already proven, all unchanged).

## Options considered

- **A new, additive tab composing three already-proven reads (chosen):**
  delivers the real three-pane experience `VISION.md` describes with zero
  backend change and zero risk to Today's existing, heavily-tested flow.
- **Replace Today's own layout with the three-pane workbench directly:**
  closer to VISION.md's eventual described end-state, but a materially
  riskier change (breaks the existing Today Playwright suite's
  navigate-to-detail assumptions, and forces the promotion decision
  before the pattern is even proven) for a record already flagged as
  this plan's largest item. Rejected for now; a future ADR can promote
  Workbench to primary/replace Today once this shape is validated, same
  as ADR-0080 did for Graph Explorer.
- **Lift `ObligationDetail`'s internal fetch into shared parent state**
  (avoiding a second `/api/obligations/:id` call for the right pane):
  marginally more efficient, but requires changing an already-proven,
  tested component's props/behavior for a single-row read that is already
  cheap. Rejected to keep `ObligationDetail` provably unchanged.

## Exit criteria and evidence

| Exit criterion | Evidence |
|---|---|
| A Workbench tab renders three panes: Attention, Current focus, Relationship context | `workbench-tab-renders-three-panes` |
| Selecting a left-pane item fills the centre pane without page navigation | `workbench-selection-fills-centre-pane` |
| The right pane shows the selected item's linked person's open commitments and recent asks | `workbench-right-pane-shows-person-brief` |
| No linked person, or nothing selected, is an honest empty state | `workbench-honest-empty-states` |
| `DailyBrief`/`ObligationDetail`/Today's existing behavior is unchanged | `existing-today-components-unchanged` |
