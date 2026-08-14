# EV-0019: Semantic search over embedded source fragments

Evidence for [ADR-0019](../adr.d/0019-semantic-search-over-source-fragments.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0019-semantic-search-over-source-fragments"

[[check]]
id = "search-function-exists"
invariant = "A function embeds a query and ranks embedded source fragments by cosine similarity to it."
type = "manual"

[[check]]
id = "search-route-exists"
invariant = "A read-only route returns ranked search results, or a typed error for every embedding-adapter/validation outcome."
type = "manual"
```

## Notes

Both checks are `manual` and unverified (`ASSERTED`) because ADR-0019 is
**Proposed**, not yet accepted or implemented. Once accepted, replace both
with `present` pattern checks against the implementing module and route,
mirroring EV-0018's shape.
