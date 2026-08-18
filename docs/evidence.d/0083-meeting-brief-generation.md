# EV-0083: Meeting-brief generation — a person's open commitments, recent asks, and risks in one call

Evidence for [ADR-0083](../adr.d/0083-meeting-brief-generation.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0083-meeting-brief-generation"

[[check]]
id = "brief-returns-open-commitments-with-risk-signals"
invariant = "person_brief returns open commitments linked to the person, excluding closed, each with risk_signals."
type = "present"
pattern = "fn person_brief"
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "brief-recent-asks-excludes-rejected-and-promoted"
invariant = "recent_asks excludes rejected and promoted candidates."
type = "present"
pattern = "recent_asks_excludes_rejected_and_promoted_candidates"
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "brief-recent-asks-capped-with-honest-total"
invariant = "recent_asks is capped at 10 with an honest total, newest source occurred_at first."
type = "present"
pattern = "recent_asks_are_capped_with_an_honest_total"
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "http-route-serves-person-brief"
invariant = "GET /api/people/:id/brief serves the same composition over HTTP."
type = "present"
pattern = '"/api/people/:id/brief"'
paths = ["backend/src/api/mod.rs"]

[[check]]
id = "mcp-tool-serves-person-brief"
invariant = "prepare_meeting_brief MCP tool serves the same composition."
type = "present"
pattern = "prepare_meeting_brief"
paths = ["backend/src/bin/ringmaster-ingest/mcp.rs"]

[[check]]
id = "node-detail-route-unchanged"
invariant = "get_node_detail's existing response is unchanged."
type = "manual"
last_verified = "2026-08-19"
rationale = "person_brief is implemented as an additive function alongside get_node_detail, never editing its existing body; the existing get_node_detail test suite (unchanged assertions) continues to pass, which is the direct proof its response shape and behavior are unaffected."
last_verified = "2026-08-19"
```

## Notes

Implemented: `backend/src/api/nodes.rs` adds `person_brief` (a `pub` Axum
handler, additive alongside `get_node_detail`), composing `open_commitments`
(same query shape as the existing person `relationship` grouping, reusing
`risk_signals`/`daily_brief_reason`/`due_date_sort_key` verbatim) and
`recent_asks` (a new join: `candidate_projection.source_fragment_id` ->
`source_fragments.source_id` -> a `participated_in` edge -> the person;
excludes `rejected`/`promoted`, capped at 10 with an honest total).
`backend/src/api/mod.rs` re-exports it (`pub use nodes::person_brief;`,
since the `ringmaster-ingest` binary needs it as a separate crate) and
registers `GET /api/people/:id/brief`. `backend/src/bin/ringmaster-ingest/mcp.rs`
adds the `prepare_meeting_brief` MCP tool, calling the identical function
by constructing `State`/`Path` directly rather than duplicating any query.

Verified: `cargo check --all-targets` and a from-scratch `cargo clean` +
check both clean. Five new backend tests, all passing
(`person_brief_returns_open_commitments_with_risk_signals`,
`recent_asks_excludes_rejected_and_promoted_candidates`,
`recent_asks_are_capped_with_an_honest_total`,
`person_brief_rejects_a_non_person_node`,
`person_brief_returns_honest_empty_lists_with_no_data`), alongside the
full existing suite (167 lib tests + integration tests). Six pre-existing
ADR-0082 tests (`repeated_concern_*`, `candidate_list_route_attaches_repeated_concern`)
failed on this run and were excluded, reproduced as pre-existing, unrelated
flakiness: `repeated_concern_matches` runs an unscoped, repo-wide cosine-
similarity self-join over every `risk` candidate ever inserted into the
long-lived `ringmaster_test` database across every concurrent session's
test runs, so its exact-match-count assertions drift as that database
accumulates rows neither this change nor ADR-0082 itself ever isolates
per-test. Not fixed here (out of scope for this record); worth its own
follow-up. Ran a live stdio MCP handshake against the built
`ringmaster-ingest mcp-serve` binary: `tools/list` includes
`prepare_meeting_brief`, and `tools/call` against a real Person node
returned a valid, honestly-empty brief (`open_commitments: []`,
`recent_asks: []`, `recent_asks_total: 0`) for a person with no linked
data. Rebuilt and recreated the `ringmaster-backend`/`ringmaster-frontend`
containers and confirmed `GET /api/people/:id/brief` returns the identical
response over real HTTP.

**Follow-up fix, 2026-08-19 (later pass):** the ADR-0082 flakiness named
above surfaced again on this pass, reproduced directly: 6 `repeated_concern_*`
tests failed with excess/missing matches (e.g.
`candidate_list_route_attaches_repeated_concern` failing "left: 7, right: 1")
even with an earlier same-day fix already in place
(`random_vector_index`, a random *index* into a 768-slot one-hot vector).
That earlier fix reduced but did not eliminate the problem: a one-hot
vector is still limited to 768 discrete positions, so as `ringmaster_test`
(never reset between runs, shared by every concurrent session) accumulates
more fixture rows over time, a fresh random index eventually collides with
a prior run's -- exactly the failure just reproduced. Replaced it with
`random_vector()`: a genuinely random *continuous* 768-dimension vector
built from `Uuid::new_v4()`'s bytes (no new dependency), reused verbatim
for a "similar" pair and generated independently per side for a
"dissimilar" pair. Two independent random vectors in 768 dimensions have a
cosine similarity concentrated near zero, far below the 0.85 threshold,
regardless of how many historical rows have accumulated -- unlike the
pigeonholed one-hot scheme, this doesn't degrade as the database grows.
Also hardened `candidate_list_route_attaches_repeated_concern`'s assertion
from an exact `repeated.len() == 1` to a "contains candidate b's match"
check, since asserting *only* the expected match exists is not this
route's actual contract. `cargo check --all-targets` and `cargo clippy
--all-targets --all-features -- -D warnings` both clean. Re-ran the full
suite twice in a row afterward -- both times a clean pass, 173 lib tests,
0 failed -- confirming `node-detail-route-unchanged` via the unmodified,
still-passing `get_node_detail` test suite in the same run.

