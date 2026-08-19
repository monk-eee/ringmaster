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
type = "absent"
pattern = "synthesize"
paths = ["backend/src/api/mod.rs", "frontend/src/api.ts"]
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
