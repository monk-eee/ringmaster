# EV-0072: Split oversized, low-cohesion backend modules with no behavior change

Evidence for [ADR-0072](../adr.d/0072-split-oversized-low-cohesion-backend-modules.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0072-split-oversized-low-cohesion-backend-modules"

[[check]]
id = "api-split-preserves-public-surface"
invariant = "backend/src/api.rs no longer exists as a single file; backend/src/api/ exists with per-responsibility submodules, and the crate still compiles with no renamed public/pub(crate) item."
type = "present"
pattern = 'mod obligations;'
paths = ["backend/src/api/mod.rs"]

[[check]]
id = "api-split-tests-pass"
invariant = "The full backend test suite passes unchanged after the api.rs split."
type = "manual"
last_verified = "2026-08-18"
rationale = "backend/src/api.rs (4774 lines, 53 non-test items, 100 tests) split into api/{mod,obligations,ingestion,candidates,search,audit_events,nodes}.rs by route/responsibility (mod.rs wires the Router plus the shared ListQuery/clamp_list_params; each submodule owns its own handlers, request/query types, and tests). Handler fns and their request/query struct parameters bumped to pub(super) only where called across the new module boundary (mod.rs's app(), and nodes.rs's cross-reference to obligations.rs's daily_brief_reason/risk_signals); no public route path, method, or JSON contract changed. cargo build, cargo test --no-run (zero warnings), the full backend suite (152 tests: 147 lib + 3 + 2 integration, all passing) via cargo test -- --test-threads=1 against ringmaster_test, cargo clippy --all-targets --all-features -- -D warnings (zero warnings), and cargo fmt --all --check (no diff) all passed cleanly with the split in place."

[[check]]
id = "graph-split-preserves-public-surface"
invariant = "backend/src/graph.rs no longer exists as a single file; backend/src/graph/ exists with per-responsibility submodules, and the crate still compiles with no renamed public/pub(crate) item."
type = "present"
pattern = 'pub use source_fragment::\{'
paths = ["backend/src/graph/mod.rs"]

[[check]]
id = "graph-split-tests-pass"
invariant = "The full backend test suite passes unchanged after the graph.rs split."
type = "manual"
last_verified = "2026-08-18"
rationale = "backend/src/graph.rs split into graph/{mod,node,edge,source_fragment}.rs (one submodule per responsibility, all public items re-exported unchanged from mod.rs). cargo check --tests, cargo clippy --all-targets --all-features -- -D warnings, and the full backend suite via the Unit Test MCP custom command (cargo test -- --test-threads=1) against ringmaster_test all passed cleanly with the split in place, before the separate, concurrently in-progress api.rs split began touching backend/src/api/."

[[check]]
id = "architecture-doc-reflects-new-layout"
invariant = "docs/ARCHITECTURE.md's module map names the new backend/src/api/ and backend/src/graph/ submodule layout instead of the old single files."
type = "manual"
last_verified = "2026-08-18"
rationale = "docs/ARCHITECTURE.md section 5's module table now names api/{mod,obligations,ingestion,candidates,search,audit_events,nodes}.rs and graph/{mod,node,edge,source_fragment}.rs in place of the old api.rs/graph.rs single-file rows."
```

## Notes

The `graph.rs` split (node/edge/source_fragment submodules) is implemented and
verified: build, clippy (`-D warnings`), and the full test suite all passed
with the old `graph.rs` deleted and `graph/mod.rs` re-exporting every public
item unchanged. Fixed four pre-existing `-D warnings` findings encountered
while first satisfying this ADR's own validation gate (unrelated to the
split itself: a `type_complexity`/`too_many_arguments` allow and two
explicit-auto-deref simplifications in `transcript.rs`).

The `api.rs` split (obligations/ingestion/candidates/search/audit_events/nodes
submodules, plus a `mod.rs` owning the `Router` and the shared list-pagination
helpers) is also implemented and verified: `cargo build`, `cargo test --no-run`
(zero warnings after a hand-verified per-module `use` prune -- `cargo fix`
proved unreliable here since it strips imports that are only referenced from
inside inline `#[cfg(test)] mod tests { .. }` blocks when its per-target
passes don't see cfg(test) and non-test code at once), the full 152-test
backend suite, `cargo clippy --all-targets --all-features -- -D warnings`,
and `cargo fmt --all --check` all passed cleanly with the old `api.rs` deleted
and every route/handler/test moved (not forked) into its new home. Both
splits under this ADR are now complete.
