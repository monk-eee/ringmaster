# EV-0018: Generate and store embeddings for source fragments

Evidence for [ADR-0018](../adr.d/0018-generate-and-store-source-fragment-embeddings.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0018-generate-and-store-source-fragment-embeddings"

[[check]]
id = "embedding-column-has-fixed-dimension"
invariant = "embeddings.embedding is a fixed-dimension vector(768) column, matching nomic-embed-text's output size."
type = "manual"

[[check]]
id = "embedding-adapter-function-exists"
invariant = "An embedding adapter calls an OpenAI-compatible endpoint and returns a typed error, without panicking, when unconfigured."
type = "manual"

[[check]]
id = "embed-source-fragment-function-exists"
invariant = "A function reads one source fragment's text, embeds it, and stores the result in the embeddings table."
type = "manual"
```

## Notes

All three checks are `manual` and unverified (`ASSERTED`) because ADR-0018
is **Proposed**, not yet accepted or implemented. Once accepted, replace
each with a `present` pattern check against the implementing migration and
module, mirroring EV-0011's shape.
