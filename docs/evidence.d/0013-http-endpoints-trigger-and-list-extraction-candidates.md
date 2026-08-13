# EV-0013: HTTP endpoints trigger and list model-based extraction candidates

Evidence for [ADR-0013](../adr.d/0013-http-endpoints-trigger-and-list-extraction-candidates.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0013-http-endpoints-trigger-and-list-extraction-candidates"

[[check]]
id = "extract-route-exists"
invariant = "A route triggers extract_candidate_via_model for one named source_fragment_id and returns a typed, non-panicking result for every model-adapter outcome (201/204/404/503)."
type = "manual"

[[check]]
id = "candidates-route-exists"
invariant = "A read-only route returns current candidate_projection rows as JSON."
type = "manual"
```

## Notes

Both checks are `manual` and unverified (`ASSERTED`) because ADR-0013 is
**Proposed**, not yet accepted or implemented. Once accepted, replace both
with `present` pattern checks against the implementing route module and its
declared path strings, mirroring EV-0012's
`api-route-exists`/`axum-dependency-declared` shape.
