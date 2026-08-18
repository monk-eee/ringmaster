# EV-0077: Bulk candidate promotion — complete the triage loop ADR-0076 started

Evidence for [ADR-0077](../adr.d/0077-bulk-candidate-promotion.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0077-bulk-candidate-promotion"

[[check]]
id = "batch-promote-rebuilds-projections-once"
invariant = "POST /api/candidates/batch-promote promotes every requested candidate and rebuilds obligation_projection and candidate_projection exactly once per request."
type = "present"
pattern = "batch_promote_candidates"
paths = ["backend/src/api/candidates.rs"]

[[check]]
id = "batch-promote-tolerates-partial-failure"
invariant = "A candidate not yet accepted, or missing, is reported per-id without failing the rest of the batch."
type = "present"
pattern = "batch_promote_reports_per_id_errors_without_failing_the_rest"
paths = ["backend/src/api/candidates.rs"]

[[check]]
id = "batch-promote-carries-due-date-and-owner-forward"
invariant = "Batch-promoted candidates carry extracted due date and owner forward exactly as the single-item promote route does."
type = "present"
pattern = "batch_promote_carries_due_date_and_owner_forward"
paths = ["backend/src/api/candidates.rs"]

[[check]]
id = "frontend-bulk-promote-action"
invariant = "The Inbox table supports bulk-promoting selected accepted/corrected candidates."
type = "present"
pattern = "batchPromoteCandidates"
paths = ["frontend/src/components/CandidatesTable.tsx"]

[[check]]
id = "single-item-promote-route-unchanged"
invariant = "The single-item promote route keeps its exact existing path and response shape."
type = "present"
pattern = '"/api/candidates/:id/promote"'
paths = ["backend/src/api/mod.rs"]
```

## Notes

Implemented: `backend/src/api/candidates.rs` shares `promote_one` between the
single-item and batch routes, adds capped partial-success
`batch_promote_candidates`, and rebuilds both projections once after each
batch. `backend/src/api/mod.rs` registers `POST /api/candidates/batch-promote`.
The existing frontend client and Inbox table call that route for selected
accepted/corrected candidates.

Verified against an isolated `ringmaster_test` database: every candidate-route
test passed, including multiple promotion, per-id partial failure, due-date and
owner carry-forward, empty-input rejection, and all pre-existing single-item
promotion cases. The full backend suite passed through Unit Test MCP. The
focused Playwright scenarios for ADR-0076 and ADR-0077 both passed against the
isolated backend/Vite ports (`2 passed`), exercising multi-select Accept and
Promote through the real React UI.
