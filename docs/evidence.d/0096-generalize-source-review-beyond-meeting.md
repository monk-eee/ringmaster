# EV-0096: Source review generalized beyond "meeting" — every ingested source type gets a browsable review UI

Evidence for [ADR-0096](../adr.d/0096-generalize-source-review-beyond-meeting.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0096-generalize-source-review-beyond-meeting"

[[check]]
id = "has-source-fragments-filter-exists"
invariant = "list_nodes_filtered supports a has_source_fragments existence filter."
type = "present"
pattern = "has_source_fragments"
paths = ["backend/src/graph/node.rs"]

[[check]]
id = "nodes-route-supports-filter"
invariant = "GET /api/nodes accepts a has_source_fragments query parameter."
type = "present"
pattern = "has_source_fragments"
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "meeting-routes-accept-any-source-type"
invariant = "The meeting detail/candidates routes no longer gate on node_type == 'meeting'."
type = "absent"
pattern = 'node\.node_type != "meeting"'
paths = ["backend/src/api/ingestion.rs"]

[[check]]
id = "frontend-fetches-by-has-source-fragments"
invariant = "MeetingReview.tsx fetches nodes by has_source_fragments, not node_type=meeting."
type = "present"
pattern = "has_source_fragments|hasSourceFragments"
paths = ["frontend/src/components/MeetingReview.tsx", "frontend/src/api.ts"]

[[check]]
id = "honest-sources-label"
invariant = "The tab label and empty state read Sources, not Meetings."
type = "present"
pattern = "Sources"
paths = ["frontend/src/App.tsx", "frontend/src/components/MeetingReview.tsx"]

[[check]]
id = "playwright-suite-passes-generalized-sources"
invariant = "The full Playwright suite passes with the updated tab-label assertion."
type = "manual"
last_verified = "2026-08-19"
rationale = "`npx playwright test --project=chromium` run after renaming the tab label and updating tests/obligations.spec.ts's tab-order and Meeting Review assertions to match; full suite passed."
```

## Notes

Internal identifiers (`MeetingReview.tsx`, `fetchMeetingDetail`/
`fetchMeetingCandidates`, `.meeting-review*` CSS classes, the `"meetings"`
tab id, the `/api/meetings/:id` route path) are deliberately unchanged —
this ADR is a data-coverage bug fix (real sources were unbrowsable), not a
rename/rebrand.

This change intentionally renamed two Rust test functions and one
Playwright test's title (the reasoning changed from "wrong node_type" to
"no source fragments," and the tab label changed from "Meetings" to
"Sources"), which required updating the matching evidence checks in
[EV-0036](0036-meeting-detail-read.md),
[EV-0037](0037-meeting-scoped-candidate-listing.md), and
[EV-0043](0043-meeting-review-page.md) to the renamed identifiers — each
now notes it was updated for ADR-0096; the invariant each check proves is
unchanged.
