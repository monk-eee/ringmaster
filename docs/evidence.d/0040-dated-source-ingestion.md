# EV-0040: Dated source ingestion — `occurred_at` becomes a required, structured field

Evidence for [ADR-0040](../adr.d/0040-dated-source-ingestion.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0040-dated-source-ingestion"

[[check]]
id = "nodes-have-occurred-at-column"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once a migration adds a nullable occurred_at TIMESTAMPTZ column to nodes."

[[check]]
id = "sources-ingest-requires-occurred-at"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once POST /api/sources/ingest creates a node + ordered fragments for a non-meeting source and rejects a missing/blank occurred_at with 400."

[[check]]
id = "non-meeting-source-splits-by-paragraph"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once a non-meeting source_type is confirmed to split into one fragment per blank-line-separated paragraph, not per speaker turn."

[[check]]
id = "meeting-ingest-requires-occurred-at"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once POST /api/meetings/ingest requires occurred_at and rejects its absence with 400, with every other field/response shape unchanged."

[[check]]
id = "ingestion-never-triggers-extraction-or-embedding"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once neither ingestion route is confirmed to call extraction or embedding implicitly."
```

## Notes

Pre-implementation: all five checks are deliberately `manual`/unproven,
per this repo's own convention (evidence stays honest about intent vs.
proof until the ADR is accepted and implemented). Do not implement before
[ADR-0040](../adr.d/0040-dated-source-ingestion.md)'s Status flips to
Accepted.
