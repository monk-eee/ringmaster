# EV-0080: Promote Graph Explorer to primary navigation

Evidence for [ADR-0080](../adr.d/0080-promote-graph-explorer-to-primary-navigation.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0080-promote-graph-explorer-to-primary-navigation"

[[check]]
id = "graph-tab-promoted-to-primary"
invariant = "\"graph\" is a member of PRIMARY_TABS in frontend/src/App.tsx."
type = "present"
pattern = 'PRIMARY_TABS: Tab\[\] = \[[^\]]*"graph"[^\]]*\]'
paths = ["frontend/src/App.tsx"]

[[check]]
id = "graph-no-longer-in-secondary-tabs"
invariant = "\"graph\" is no longer a member of SECONDARY_TABS."
type = "absent"
pattern = 'SECONDARY_TABS: Tab\[\] = \[[^\]]*"graph"[^\]]*\]'
paths = ["frontend/src/App.tsx"]

[[check]]
id = "obligations-and-search-remain-demoted"
invariant = "Obligations and Search stay in SECONDARY_TABS, unchanged, per ADR-0039's still-correct reasoning for those two tabs."
type = "present"
pattern = 'SECONDARY_TABS: Tab\[\] = \[[^\]]*"obligations"[^\]]*"search"[^\]]*\]'
paths = ["frontend/src/App.tsx"]

[[check]]
id = "nav-regrouping-proven-live"
invariant = "Focused Playwright coverage proves the tab bar renders Today/Timeline/People/Inbox/Graph as primary and Obligations/Search/Meetings/Activity as secondary."
type = "present"
pattern = 'primary navigation is Today/Timeline/People/Inbox/Graph; Obligations/Search remain as secondary tabs \(ADR-0080\)'
paths = ["frontend/tests/obligations.spec.ts"]

[[check]]
id = "graph-explorer-component-and-routes-unchanged"
invariant = "No route, schema, or GraphExplorer component behavior changes as part of this navigation-only ADR."
type = "manual"
rationale = "A negative claim (no unrelated change) is not reliably provable by a positive regex match; verified by direct review of the implementing diff (two array-literal edits in App.tsx plus test expectations, no changes to GraphExplorer.tsx, api.ts, or any backend route), matching EV-0033's own precedent for the same kind of claim."
last_verified = "2026-08-18"

[[check]]
id = "narrow-screen-navigation-still-reachable"
invariant = "All five primary tabs plus the secondary group remain reachable via horizontal scrolling at narrow viewport widths."
type = "manual"
rationale = "Proven by running the existing 'primary navigation scrolls internally without widening a narrow viewport' Playwright test directly against the implemented change (npx playwright test tests/obligations.spec.ts): passed in 1.6s with Graph now present as a fifth primary tab."
last_verified = "2026-08-18"
```
