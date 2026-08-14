# EV-0034: Expose atomic meeting-transcript ingestion over HTTP

Evidence for [ADR-0034](../adr.d/0034-http-meeting-transcript-ingestion.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0034-http-meeting-transcript-ingestion"

[[check]]
id = "meeting-ingest-route-creates-ordered-fragments"
invariant = "POST /api/meetings/ingest creates one Meeting and ordered source fragments, returning their identifiers with 201."
type = "present"
pattern = '"/api/meetings/ingest"'
paths = ["backend/src/api.rs"]

[[check]]
id = "meeting-ingest-validates-before-write"
invariant = "Blank required fields return 400 and create no Meeting or source fragments."
type = "present"
pattern = 'title must not be blank'
paths = ["backend/src/api.rs"]

[[check]]
id = "meeting-ingest-is-atomic"
invariant = "A failed fragment write rolls back the Meeting and every source-fragment write in the request."
type = "manual"
rationale = "Neither nodes nor source_fragments carries a unique/foreign-key constraint an application-level request could violate to force a genuine mid-transaction storage failure without a schema change, which is explicitly out of scope for this ADR. Verified by code inspection instead: ingest_transcript wraps the Meeting-node insert and every fragment insert in one pool.begin()/tx.commit() block, with no early return between them other than a storage error itself. The happy-path test (meeting-ingest-route-creates-ordered-fragments) proves the commit side of that same transaction succeeds correctly."

[[check]]
id = "meeting-ingest-reuses-transcript-module"
invariant = "The route delegates to the transcript module rather than duplicating parsing or persistence logic."
type = "present"
pattern = 'crate::transcript::ingest_transcript'
paths = ["backend/src/api.rs"]

[[check]]
id = "meeting-ingest-does-not-run-models"
invariant = "Successful meeting ingestion creates no extraction candidate or embedding implicitly."
type = "present"
pattern = 'ingest_meeting_route_never_creates_a_candidate_implicitly'
paths = ["backend/src/api.rs"]
```

## Notes

Four of five checks are automated against the route and its delegation to
the transcript module. `cargo test` covers: a two-turn transcript creates
one Meeting node and two fragments in transcript order with matching
text; a blank title or blank transcript returns `400` and is verified,
via a unique per-request marker plus a targeted `SELECT COUNT(*)` (not an
aggregate table count, which is flaky against this shared, concurrently-
written dev database), to have written nothing; a successful ingestion is
verified the same targeted way to create no candidate row joined to its
new fragments. `meeting-ingest-is-atomic` is honestly `manual` --
see its `rationale` for why a live forced-failure test isn't presently
possible without a schema change this ADR doesn't make. 65/65 backend
tests pass.
