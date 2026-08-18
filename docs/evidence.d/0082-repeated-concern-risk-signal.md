# EV-0082: Repeated-concern signal — the same risk raised in multiple meetings, still unpromoted

Evidence for [ADR-0082](../adr.d/0082-repeated-concern-risk-signal.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0082-repeated-concern-risk-signal"

[[check]]
id = "repeated-concern-flags-cross-meeting-similar-risks"
invariant = "Two risk candidates from different meetings with cosine similarity >= 0.85, neither promoted, are both flagged repeated_concern naming each other."
type = "present"
pattern = 'fn repeated_concern_flags_similar_risks_from_different_meetings'
paths = ["backend/src/api/candidates.rs"]

[[check]]
id = "repeated-concern-requires-distinct-meetings"
invariant = "Two similar risk candidates from the same meeting are not flagged (distinct-meeting requirement)."
type = "present"
pattern = 'fn repeated_concern_does_not_flag_similar_risks_from_the_same_meeting'
paths = ["backend/src/api/candidates.rs"]

[[check]]
id = "repeated-concern-excludes-promoted-risks"
invariant = "A matching pair where either side is already promoted is not flagged."
type = "present"
pattern = 'fn repeated_concern_excludes_a_promoted_risk'
paths = ["backend/src/api/candidates.rs"]

[[check]]
id = "repeated-concern-excludes-rejected-candidates"
invariant = "A matching pair where either side is rejected is not flagged."
type = "present"
pattern = 'fn repeated_concern_excludes_a_rejected_risk'
paths = ["backend/src/api/candidates.rs"]

[[check]]
id = "repeated-concern-requires-similarity-threshold"
invariant = "Two dissimilar risk candidates (below the 0.85 threshold) are not flagged."
type = "present"
pattern = 'fn repeated_concern_does_not_flag_dissimilar_risks'
paths = ["backend/src/api/candidates.rs"]

[[check]]
id = "repeated-concern-attached-to-candidate-list-route"
invariant = "repeated_concern appears on GET /api/candidates responses, with an explanation naming the matched meeting(s)."
type = "present"
pattern = 'fn candidate_list_route_attaches_repeated_concern'
paths = ["backend/src/api/candidates.rs"]
```
