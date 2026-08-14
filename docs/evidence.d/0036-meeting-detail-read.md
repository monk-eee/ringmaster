# EV-0036: Meeting detail read — one meeting with its ordered transcript fragments

Evidence for [ADR-0036](../adr.d/0036-meeting-detail-read.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0036-meeting-detail-read"

[[check]]
id = "ingest-transcript-writes-sequence"
invariant = "ingest_transcript writes a 0-based sequence per fragment."
type = "present"
pattern = 'INSERT INTO source_fragments \(source_id, text, speaker, hash, sequence\)'
paths = ["backend/src/transcript.rs"]

[[check]]
id = "list-source-fragments-orders-by-sequence"
invariant = "Fragments are read back ordered by sequence, not raw created_at."
type = "present"
pattern = 'ORDER BY sequence ASC NULLS LAST, created_at ASC, id ASC'
paths = ["backend/src/graph.rs"]

[[check]]
id = "meeting-detail-route-returns-ordered-fragments"
invariant = "GET /api/meetings/:id returns the meeting plus its ordered fragments."
type = "present"
pattern = 'async fn get_meeting_detail'
paths = ["backend/src/api.rs"]

[[check]]
id = "meeting-detail-route-404s-for-non-meeting"
invariant = "GET /api/meetings/:id returns 404 for an unknown id or a non-meeting node."
type = "present"
pattern = 'meeting_detail_route_404s_for_a_non_meeting_node'
paths = ["backend/src/api.rs"]
```

## Notes

All four checks are automated against the implementing function/route/test.
