# EV-0072: Split oversized, low-cohesion backend modules with no behavior change

Evidence for [ADR-0072](../adr.d/0072-split-oversized-low-cohesion-backend-modules.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0072-split-oversized-low-cohesion-backend-modules"

[[check]]
id = "api-split-preserves-public-surface"
invariant = "backend/src/api.rs no longer exists as a single file; backend/src/api/ exists with per-responsibility submodules, and the crate still compiles with no renamed public/pub(crate) item."
type = "manual"

[[check]]
id = "api-split-tests-pass"
invariant = "The full backend test suite passes unchanged after the api.rs split."
type = "manual"

[[check]]
id = "graph-split-preserves-public-surface"
invariant = "backend/src/graph.rs no longer exists as a single file; backend/src/graph/ exists with per-responsibility submodules, and the crate still compiles with no renamed public/pub(crate) item."
type = "manual"

[[check]]
id = "graph-split-tests-pass"
invariant = "The full backend test suite passes unchanged after the graph.rs split."
type = "manual"

[[check]]
id = "architecture-doc-reflects-new-layout"
invariant = "docs/ARCHITECTURE.md's module map names the new backend/src/api/ and backend/src/graph/ submodule layout instead of the old single files."
type = "manual"
```

## Notes

Not yet implemented. This ADR is `Proposed`; all checks are honestly `manual`
with no `last_verified`, reporting `ASSERTED` until the named decider accepts
the ADR and each split lands, at which point each check should be replaced
with a `present`/`absent` check (e.g. `absent` for `backend/src/api.rs` as a
file, `present` for `backend/src/api/mod.rs`) plus one `manual` check
recording the live full-suite test run, matching this repository's own
precedent (e.g. EV-0069).
