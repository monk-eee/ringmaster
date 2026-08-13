# EV-0003: Ringmaster ingests MindLeak as an MCP source, not a shared graph

Evidence for [ADR-0003](../adr.d/0003-ringmaster-ingests-mindleak-as-an-mcp-source.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0003-ringmaster-ingests-mindleak-as-an-mcp-source"

[[check]]
id = "no-direct-mindleak-storage-access"
invariant = "No Ringmaster source file opens MindLeak's SQLite database directly."
type = "absent"
pattern = '\.mindleak/|rusqlite|\bsqlite\b'
paths = ["backend/src/**", "backend/Cargo.toml"]

[[check]]
id = "vision-names-boundary-question"
invariant = "The vision document names the MindLeak/Ringmaster boundary as the open question this ADR addresses."
type = "present"
pattern = 'MindLeak/Ringmaster boundary'
paths = ["docs/VISION.md"]
```

## Notes

`no-direct-mindleak-storage-access` is now a declarative `absent` check now
that Rust source exists: no ingestion adapter has been built yet, so the
check currently passes vacuously over `backend/src/**` and `backend/Cargo.toml`.
It keeps proving the invariant once an adapter lands, since MindLeak must
still only be reached through its own MCP tools, never through
`rusqlite`/`.sqlite` file access or a linked SQLite dependency.
