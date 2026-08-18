# EV-0069: Resolve participant/speaker names to existing Person nodes at ingestion

Evidence for [ADR-0069](../adr.d/0069-resolve-participants-to-person-nodes-at-ingestion.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0069-resolve-participants-to-person-nodes-at-ingestion"

[[check]]
id = "ingestion-resolves-participants-by-exact-match"
invariant = "ingest_source resolves each unique participant/speaker name against existing Person nodes by exact, case-insensitive canonical_text match."
type = "present"
pattern = "lower\\(canonical_text\\) = lower\\(\\$1\\)"
paths = ["backend/src/transcript.rs"]

[[check]]
id = "ingestion-creates-participated-in-edge-on-exact-match"
invariant = "A matched name creates a participated_in edge from the person node to the new source node, in the same transaction as fragment creation."
type = "present"
pattern = 'graph::create_edge\([\s\S]*?&mut \*tx,[\s\S]*?person_id,[\s\S]*?source_node_id,[\s\S]*?"participated_in",[\s\S]*?Some\(1\.0\)'
paths = ["backend/src/transcript.rs"]

[[check]]
id = "ingestion-unchanged-without-a-match"
invariant = "An unmatched name creates no new Person node and no edge; ingestion with no matching participants/speakers behaves exactly as before."
type = "manual"
last_verified = "2026-08-17"
rationale = "The focused participant_linking integration target passed against ringmaster_test. Its unmatched-name case ingests a meeting source, then proves no participated_in edge and no Person node for either unmatched participant or speaker name while ordinary fragment creation succeeds."

[[check]]
id = "backend-suite-passes-with-participant-linking"
invariant = "The full backend suite passes against ringmaster_test with participant resolution in place."
type = "manual"
last_verified = "2026-08-17"
rationale = "The full serial backend suite passed through Unit Test MCP against ringmaster_test with participant resolution in place. The focused participant_linking target also passed and covers SourceMetadata meeting ingestion, legacy MeetingMetadata transcript ingestion, exact case-insensitive resolution, deduplication, and unmatched-name behavior."
```

## Notes

Participant/speaker resolution runs inside the existing ingestion transaction
in [backend/src/transcript.rs](../../backend/src/transcript.rs). The focused
integration target uses `ringmaster_test`; it covers both the current source
ingestion route and the legacy transcript path.
