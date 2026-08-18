# EV-0013: HTTP endpoints trigger and list model-based extraction candidates

Evidence for [ADR-0013](../adr.d/0013-http-endpoints-trigger-and-list-extraction-candidates.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0013-http-endpoints-trigger-and-list-extraction-candidates"

[[check]]
id = "extract-route-exists"
invariant = "A route triggers extract_candidate_via_model for one named source_fragment_id and translates every model-adapter outcome into a typed HTTP status without panicking."
type = "present"
pattern = '"/api/source-fragments/:id/extract"'
paths = ["backend/src/api/mod.rs"]

[[check]]
id = "candidates-route-exists"
invariant = "A read-only route returns current candidate_projection rows as JSON."
type = "present"
pattern = '"/api/candidates"'
paths = ["backend/src/api/mod.rs"]
```

## Notes

Both checks are automated against the route module that implements them.
`cargo test` cases exercise, against a live Postgres instance: a 404 for an
unknown source_fragment_id (deterministic, regardless of model
configuration); the candidates route returning a JSON array; and, when
`RINGMASTER_LLM_URL` is configured, a full live HTTP round-trip that creates
a real candidate and returns it with a `201`. The `204` (nothing
extractable) and `503` (model unconfigured/unreachable) branches are
implemented but not unit-tested, since neither can be triggered
deterministically without either depending on a specific model judgement or
racily mutating process-wide environment state in a parallel test binary.
