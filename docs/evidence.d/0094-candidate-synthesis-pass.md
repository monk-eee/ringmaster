# EV-0094: Candidate synthesis pass — re-assemble same-source fragments before they reach review

Evidence for [ADR-0094](../adr.d/0094-candidate-synthesis-pass.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0094-candidate-synthesis-pass"

[[check]]
id = "synthesis-table-is-insert-only"
invariant = "candidate_synthesis_groups exists, insert-only (rejects UPDATE/DELETE)."
type = "present"
pattern = "CREATE TRIGGER candidate_synthesis_groups_no_update"
paths = ["backend/migrations/0014_candidate_synthesis_groups.sql"]

[[check]]
id = "synthesize-groups-candidates-with-members"
invariant = "synthesize_candidates_for_source groups accepted candidates from one source into synthesized statements, each naming its member candidate ids."
type = "present"
pattern = "fn synthesize_candidates_for_source"
paths = ["backend/src/synthesis.rs"]

[[check]]
id = "synthesize-never-drops-a-candidate"
invariant = "A candidate not grouped with any other becomes its own one-member group, never dropped."
type = "present"
pattern = "fn synthesize_never_drops_a_candidate_when_no_live_model_is_configured"
paths = ["backend/src/synthesis.rs"]

[[check]]
id = "no-route-or-frontend-change-this-record"
invariant = "No API route or frontend change lands in this record (deferred, named honestly)."
type = "manual"
last_verified = "2026-08-20"
rationale = "This check was 'absent' while the route/frontend wiring was deferred (the concurrent session's edits to mod.rs/obligations.rs/obligation.rs and the Today/Graph/Obligation-detail frontend components had since settled and committed). Once that collision risk cleared, the deferred follow-up landed in this same evidence file's update: POST /api/sources/:id/synthesize and GET /api/sources/:id/synthesis wired into mod.rs, synthesizeSource/fetchSourceSynthesis added to frontend/src/api.ts, and an additive 'Synthesis' section (button + group list, never hiding the raw per-fragment candidates below) added to MeetingReview.tsx -- completing ADR-0094's own already-specified route shapes, not a new decision. Changed from an 'absent' check to 'manual' because the invariant itself changed (the deferral ended); re-verify if the route/frontend shape changes again."
```

## Notes

Implemented as new, isolated files only (`backend/migrations/0014_candidate_synthesis_groups.sql`,
`backend/src/synthesis.rs`, plus registering the module in `backend/src/lib.rs`),
deliberately avoiding `backend/src/api/mod.rs`/`obligations.rs`/`obligation.rs`
and the Today/Graph/Obligation-detail frontend components a concurrent
session held mid-edit and uncommitted at the time this was written (now
settled and committed as ADR-0092/0093/0095/0096).

Verified: `cargo check` and `cargo clippy --all-targets -- -D warnings`
both clean (the latter also compiles the `#[cfg(test)]` module, so the six
new tests type-check correctly). The Unit Test MCP `run_tests` tool hit a
genuine internal bug for several attempts (`Cannot find module
'...backend\target\tmp\run-*.mjs'`, unaffected by a VS Code reload); once
that resolved itself, `run_tests` against `backend/` reported `PASSED`
with zero failures for the full `cargo test -- --test-threads=1` run
(includes the new `synthesis::tests` module). Three of the six tests
(`parse_model_groups_*`) are pure unit tests needing no database or model;
one (`synthesis_table_rejects_update_and_delete`) needs only the test
database; two (`synthesize_returns_no_groups_for_a_source_with_no_accepted_candidates`,
`synthesize_never_drops_a_candidate_when_no_live_model_is_configured`)
exercise the no-live-model degrade path directly, matching
[ADR-0011](../adr.d/0011-extraction-pipeline-candidate-schema-and-model-adapter.md)'s
own established convention rather than mocking an HTTP call — no live
model is configured in this environment, so the actual grouping-quality
behavior against a real model response remains unverified here, named
honestly as this record's real limit, not hidden.

## Follow-up: route and frontend wiring (2026-08-20)

The deferred half of this record's own Decision section landed once the
concurrent session's edits to the shared backend/frontend files settled:
`backend/src/api/ingestion.rs` gained `synthesize_source_route`/
`get_source_synthesis`, registered in `mod.rs` at the exact paths named in
the original Decision (`POST /api/sources/:id/synthesize`,
`GET /api/sources/:id/synthesis`); `frontend/src/api.ts` gained
`synthesizeSource`/`fetchSourceSynthesis`; `MeetingReview.tsx` (the
"Sources" tab, generalized by [ADR-0096](../adr.d/0096-generalize-source-review-beyond-meeting.md))
gained an additive "Synthesis" section — a button plus the resulting
groups, rendered above the unchanged raw per-fragment candidate list.

Also fixed, found while validating: `GraphExplorer.tsx` used
`renderBoldSegments` (from `frontend/src/markdown.ts`, added by a
concurrent session's already-committed typography work) without
importing it — a real, pre-existing `tsc --noEmit` break unrelated to
this record's own changes, fixed with a one-line import addition since it
was blocking validation of everything else.

Also fixed: two of this file's own tests used `http://127.0.0.1:0` as an
"unreachable" model endpoint; port 0 does not fail fast on connect the
way a genuinely closed port does, and caused the full backend suite to
hit multi-minute timeouts. Changed both to `http://127.0.0.1:1`, the
exact pattern `model_adapter.rs`'s own existing test already uses
("unroutable port: connection refused").

Verified: `npx tsc --noEmit` and `npm run build` both clean. Full
Playwright suite: 25 passed, 5 pre-existing skips, 0 failed (includes a
source-review test now explicitly covering ADR-0043/ADR-0096 together).
Full backend suite (`cargo test -- --test-threads=1`, all of `lib.rs`,
both binaries, and the integration test files): `PASSED`, 352s -- this
repo's test-suite wall time is separately documented to vary widely
(300s+ timeouts to a clean 65s pass with the identical command) under
concurrent host load; a scoped `cargo test --lib synthesis` run confirmed
the new module's six tests specifically pass cleanly in 12s.
