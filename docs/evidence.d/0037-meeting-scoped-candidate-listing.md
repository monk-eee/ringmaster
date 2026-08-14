# EV-0037: Meeting-scoped candidate listing and extraction progress

Evidence for [ADR-0037](../adr.d/0037-meeting-scoped-candidate-listing.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0037-meeting-scoped-candidate-listing"

[[check]]
id = "meeting-candidates-route-lists-all-fragments"
invariant = "GET /api/meetings/:id/candidates lists every fragment, including those with zero candidates."
type = "present"
pattern = 'async fn get_meeting_candidates'
paths = ["backend/src/api.rs"]

[[check]]
id = "meeting-candidates-route-includes-candidate-state"
invariant = "A fragment's extracted candidates appear with their real validation state."
type = "present"
pattern = 'meeting_candidates_route_lists_extracted_and_pending_fragments'
paths = ["backend/src/api.rs"]

[[check]]
id = "meeting-candidates-route-computes-fragment-progress"
invariant = "progress counts fragments (extracted/pending), not candidates."
type = "present"
pattern = 'extracted_fragment_count'
paths = ["backend/src/api.rs"]

[[check]]
id = "meeting-candidates-route-404s-for-non-meeting"
invariant = "The route 404s for an unknown id or a non-meeting node."
type = "present"
pattern = 'meeting_candidates_route_404s_for_a_non_meeting_node'
paths = ["backend/src/api.rs"]

[[check]]
id = "meeting-candidates-route-never-triggers-extraction"
invariant = "No extraction is triggered by this route."
type = "present"
pattern = 'meeting_candidates_route_never_triggers_extraction'
paths = ["backend/src/api.rs"]
```

## Notes

All five checks are automated against the implementing route and its tests.
