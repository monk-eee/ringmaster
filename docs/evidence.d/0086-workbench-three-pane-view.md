# EV-0086: Workbench — a three-pane, no-navigation view over already-built data

Evidence for [ADR-0086](../adr.d/0086-workbench-three-pane-view.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0086-workbench-three-pane-view"

[[check]]
id = "workbench-tab-renders-three-panes"
invariant = "A Workbench tab renders three panes: Attention, Current focus, Relationship context."
type = "present"
pattern = "workbench-pane-attention"
paths = ["frontend/src/components/Workbench.tsx"]

[[check]]
id = "workbench-selection-fills-centre-pane"
invariant = "Selecting a left-pane item fills the centre pane without page navigation."
type = "present"
pattern = "ObligationDetail"
paths = ["frontend/src/components/Workbench.tsx"]

[[check]]
id = "workbench-right-pane-shows-person-brief"
invariant = "The right pane shows the selected item's linked person's open commitments and recent asks."
type = "present"
pattern = "fetchPersonBrief"
paths = ["frontend/src/components/PersonBriefPanel.tsx"]

[[check]]
id = "workbench-honest-empty-states"
invariant = "No linked person, or nothing selected, is an honest empty state."
type = "present"
pattern = "playwright-proves-workbench-honest-empty-states"
paths = ["frontend/tests/obligations.spec.ts"]

[[check]]
id = "existing-today-components-unchanged"
invariant = "DailyBrief/ObligationDetail/Today's existing behavior is unchanged."
type = "manual"
rationale = "DailyBrief.tsx and ObligationDetail.tsx are not edited by this change (Workbench.tsx imports and renders them as-is); the existing Today/ObligationDetail Playwright coverage continues to pass unmodified, which is the direct proof."
last_verified = "2026-08-19"
```

## Notes

Implemented: a new `Workbench` component (`frontend/src/components/Workbench.tsx`)
composes three already-proven reads with zero new backend routes --
`DailyBrief` (left, Attention), `ObligationDetail` (centre, Current
focus, unchanged), and a new `PersonBriefPanel` (right, Relationship
context) that fetches `GET /api/people/:id/brief` (ADR-0083) for the
selected obligation's linked `owns`/`person` edge, found via a second,
independent `GET /api/obligations/:id` call. `App.tsx` registers a new
secondary "Workbench" tab (matching Graph Explorer's original
secondary-first precedent, ADR-0026/ADR-0080); `api.ts` adds
`fetchPersonBrief`. `DailyBrief.tsx`/`ObligationDetail.tsx` are not
edited.

Verified: `npx tsc --noEmit` and `npm run build` both clean. Two
Playwright tests in `frontend/tests/obligations.spec.ts` -- `workbench:
selecting an item fills current focus and relationship context
(ADR-0086)` and `workbench: honest empty state when the selected item
has no linked person (ADR-0086)` -- both pass, alongside the full suite
(21 passed, 5 pre-existing skips, 0 failures related to this change).
Updating `Workbench` also added a new tab, so fixed one existing test's
exact-tab-list assertion (`primary navigation is Today/Timeline/...`,
ADR-0080) to include "Workbench"; confirmed passing after the fix. Two
unrelated Graph Explorer tests (`graph trail: traversing two edges...`
ADR-0033, `Actions lens filters neighbours...` ADR-0081) failed once
under full-suite load and passed cleanly on immediate retry -- the same
pre-existing timing flakiness already documented in EV-0085, now against
a `ringmaster_test` database that has grown to 695 obligations across
this session's accumulated runs; not caused by, or fixed by, this change.
