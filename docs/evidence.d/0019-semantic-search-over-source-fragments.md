# EV-0019: Semantic search over embedded source fragments

Evidence for [ADR-0019](../adr.d/0019-semantic-search-over-source-fragments.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0019-semantic-search-over-source-fragments"

[[check]]
id = "search-function-exists"
invariant = "A function embeds a query and ranks embedded source fragments by cosine similarity to it."
type = "present"
pattern = 'fn search_source_fragments'
paths = ["backend/src/graph.rs"]

[[check]]
id = "search-route-exists"
invariant = "A read-only route returns ranked search results, or a typed error for every embedding-adapter/validation outcome."
type = "present"
pattern = '"/api/search"'
paths = ["backend/src/api.rs"]
```

## Notes

Both checks are automated and verified directly against the crate files
that implement them. `cargo test` cases exercise: `400` for a missing or
blank `q` (deterministic, no live model needed), and — when
`RINGMASTER_EMBEDDING_URL` is configured — a live round-trip that embeds a
source fragment, searches for related text, and confirms the fragment is
ranked back. Unconfigured behavior (`503`, never a panic) mirrors the
already-proven extraction-route posture (ADR-0013) rather than repeating
its own dedicated test.
