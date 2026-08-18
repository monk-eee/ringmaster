# EV-0039: Product re-steer — Today/Timeline/People/Inbox as primary navigation

Evidence for [ADR-0039](../adr.d/0039-product-re-steer-primary-navigation.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0039-product-re-steer-primary-navigation"

[[check]]
id = "primary-nav-order-and-default"
invariant = "Today/Timeline/People/Inbox render as the primary tab group, in that order, with Today the default landing tab."
type = "present"
pattern = 'const PRIMARY_TABS: Tab\[\] = \["today", "timeline", "people", "inbox"\]'
paths = ["frontend/src/App.tsx"]

[[check]]
id = "secondary-nav-group-exists"
invariant = "Obligations/Graph/Search render as a visually distinct secondary/developer group, not deleted (later ADRs, e.g. ADR-0043's Meetings tab, may append further secondary tabs without breaking this check)."
type = "present"
pattern = 'const SECONDARY_TABS: Tab\[\] = \["obligations", "search", "graph"'
paths = ["frontend/src/App.tsx"]

[[check]]
id = "narrow-nav-scrolls-within-tab-list"
invariant = "On narrow viewports, the complete primary/secondary navigation scrolls inside its tab list instead of widening the document."
type = "present"
pattern = '\.tabs\s*\{[\s\S]*?max-width: 100%;[\s\S]*?overflow-x: auto;'
paths = ["frontend/public/style.css"]

[[check]]
id = "playwright-proves-narrow-nav-remains-reachable"
invariant = "Focused browser coverage proves the document stays viewport-wide at 390px and the final Activity tab remains reachable."
type = "present"
pattern = 'primary navigation scrolls internally without widening a narrow viewport[\s\S]*?scrollWidth[\s\S]*?Activity'
paths = ["frontend/tests/obligations.spec.ts"]

[[check]]
id = "today-page-renders-required-sections"
invariant = "The Today page renders a greeting, the capped ranked list, a labeled \"Do these together\" section, and a compact coming-soon strip, in that order (robust to DailyBrief's props being reformatted across multiple lines, e.g. ADR-0047's onSelect prop)."
type = "present"
pattern = 'today-greeting[\s\S]*DailyBrief[\s\S]*items=\{dailyBrief\.slice\(0, TODAY_ITEM_CAP\)\}[\s\S]*Do these together[\s\S]*ComingSoonStrip'
paths = ["frontend/src/App.tsx"]

[[check]]
id = "people-tab-lists-and-opens-relationship-data"
invariant = "The People tab lists person nodes and opens each into its existing relationship data, with no new backend route."
type = "present"
pattern = 'fetchNodes\("person"[\s\S]*fetchNodeDetail'
paths = ["frontend/src/components/People.tsx"]

[[check]]
id = "inbox-is-relabeled-candidates"
invariant = "The Inbox tab is the relabeled Candidates route/actions, unchanged in behavior."
type = "present"
pattern = 'inbox: "Inbox"'
paths = ["frontend/src/App.tsx"]

[[check]]
id = "no-new-backend-or-dependency"
invariant = "No new backend route, migration, or frontend dependency was added by this ADR."
type = "manual"
rationale = "A negative claim (no new route/migration/dependency) is not reliably provable by a positive regex match; verified by direct review of the implementation diff (no new backend/migrations/*.sql, no new .route() registration in backend/src/api.rs, no new frontend/package.json dependency) once implemented, matching EV-0021's and EV-0033's own precedent for the same kind of claim."
last_verified = "2026-08-14"
```

## Notes

Implemented: the primary/secondary tab regrouping, the Today page
composition (greeting, capped ranked list with a "N more in Timeline"
escape hatch, "Do these together" heading over the existing Suggested
Focus Blocks card, and a compact coming-soon strip over
`GET /api/time-horizon`'s own Next 7/30 Days data), the new People tab
(`GET /api/nodes?node_type=person` list, `GET /api/nodes/:id` detail
reusing the same relationship-group rendering the Graph Explorer already
had), and the Inbox relabel of Candidates. Verified with
`npx tsc --noEmit`, `npm run build`, and Playwright coverage exercising
the new tab labels, the primary/secondary grouping, and the People list/
detail flow. Narrow-viewport coverage also proves the tab list contains its
own horizontal scroll at 390px while Activity remains reachable, without
widening the document. `no-new-backend-or-dependency` stays a reasoned `manual`
check for the same reason EV-0021/EV-0033 keep an equivalent claim
manual; confirmed by diff review that `backend/` changes in this slice
are comment-only (correcting a stale ADR-number reference) and
`frontend/package.json` is untouched.
