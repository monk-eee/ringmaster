# EV-0076: Bulk candidate triage — multi-select accept/reject, confidence-first ordering

Evidence for [ADR-0076](../adr.d/0076-bulk-candidate-triage.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0076-bulk-candidate-triage"

[[check]]
id = "batch-endpoint-rebuilds-projection-once"
invariant = "POST /api/candidates/batch transitions every requested candidate and rebuilds candidate_projection exactly once per request, not once per candidate."
type = "present"
pattern = "batch_transition_candidates"
paths = ["backend/src/api/candidates.rs"]

[[check]]
id = "batch-endpoint-tolerates-partial-failure"
invariant = "A candidate already transitioned, or missing, is reported per-id without failing the rest of the batch."
type = "present"
pattern = "batch_route_reports_per_id_errors_without_failing_the_rest"
paths = ["backend/src/api/candidates.rs"]

[[check]]
id = "candidates-ordered-by-confidence-first"
invariant = "GET /api/candidates orders by confidence descending with candidate_id as a deterministic tiebreak."
type = "present"
pattern = 'confidence DESC NULLS LAST'
paths = ["backend/src/api/candidates.rs"]

[[check]]
id = "frontend-bulk-select-and-act"
invariant = "The Inbox table supports selecting loaded candidates and bulk accept/reject."
type = "present"
pattern = "batchTransitionCandidates"
paths = ["frontend/src/components/CandidatesTable.tsx"]

[[check]]
id = "single-item-routes-unchanged"
invariant = "accept/reject/correct/promote single-item routes keep their exact existing paths and behavior."
type = "present"
pattern = '"/api/candidates/:id/accept"'
paths = ["backend/src/api/mod.rs"]
```

## Notes

Implemented: `backend/src/api/candidates.rs` adds `transition_one` (shared
by the single-item routes and the new batch route), `batch_transition_candidates`
(`POST /api/candidates/batch`, capped at 200 ids, one `rebuild_candidate_projection`
call regardless of batch size), and changes `list_candidates`'s `ORDER BY`
to `confidence DESC NULLS LAST, candidate_id`. `frontend/src/api.ts` adds
`batchTransitionCandidates`. `frontend/src/components/CandidatesTable.tsx`
adds a header "select all loaded" checkbox, a per-row checkbox on every
`candidate`-state row, and a bulk action bar. `frontend/public/style.css`
adds matching `.bulk-action-bar`/`.bulk-accept-button`/`.bulk-reject-button`
styles.

Verified: `cargo check --tests` clean; five new backend tests all pass
(`candidates_route_orders_by_confidence_descending`,
`batch_route_accepts_multiple_candidates_in_one_request`,
`batch_route_reports_per_id_errors_without_failing_the_rest`,
`batch_route_rejects_an_invalid_action`, `batch_route_rejects_empty_candidate_ids`),
alongside the full existing backend suite (152 pre-existing + 5 new = 157
lib tests, plus integration tests, all passing). `npx tsc --noEmit` and
`npm run build` clean. A new Playwright test
(`inbox tab: bulk-select and Accept transitions multiple candidates in one
request (ADR-0076)`) mocks `GET /api/candidates`/`POST /api/candidates/batch`,
selects two of three candidates, clicks "Accept 2 selected", and asserts
the batch request carried exactly those two ids with `action: "accept"`
and that only those two rows show `accepted` afterward — passed. Ran the
full Playwright suite against the isolated stack: 14 passed, 5 skipped
(pre-existing, data/model-dependent skips, unrelated to this change), 1
failed on the first pass (`graph trail: traversing two edges`, ADR-0033) —
reproduced in isolation as flaky/pre-existing (passed cleanly on retry),
not a regression from this change. Also fixed one existing Playwright
assertion (`correcting a candidate ... (ADR-0045)`) whose `td` column
index shifted by 1 because of the new leading checkbox column; confirmed
passing after the fix.

