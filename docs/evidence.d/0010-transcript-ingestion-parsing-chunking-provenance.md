# EV-0010: Transcript ingestion — parsing, chunking, and immutable source fragments

Evidence for [ADR-0010](../adr.d/0010-transcript-ingestion-parsing-chunking-provenance.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0010-transcript-ingestion-parsing-chunking-provenance"

[[check]]
id = "source-fragments-are-immutable"
invariant = "The database rejects mutation or deletion of an existing source_fragments row."
type = "present"
pattern = 'reject_source_fragment_mutation'
paths = ["backend/migrations/0005_graph_nodes_edges_source_fragments.sql"]

[[check]]
id = "ingest-transcript-function-exists"
invariant = "A Rust function ingests a transcript into a meeting node plus hashed source fragments."
type = "present"
pattern = 'pub async fn ingest_transcript'
paths = ["backend/src/transcript.rs"]

[[check]]
id = "parse-transcript-function-exists"
invariant = "Parsing splits a transcript by speaker turn, not arbitrary character count."
type = "present"
pattern = 'pub fn parse_transcript'
paths = ["backend/src/transcript.rs"]
```

## Notes

All three checks are automated and verified directly against the migration
and crate that implement them. `cargo test` cases exercise: parsing a
multi-speaker transcript into distinct turns; ingesting a transcript and
reading back the created meeting node and its hashed fragments; and
confirming `UPDATE`/`DELETE` against a source fragment are rejected, all
against a live Postgres instance.
