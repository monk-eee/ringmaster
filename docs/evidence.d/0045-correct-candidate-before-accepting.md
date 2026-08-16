# EV-0045: Correct a candidate before accepting it

Evidence for [ADR-0045](../adr.d/0045-correct-candidate-before-accepting.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0045-correct-candidate-before-accepting"

[[check]]
id = "correct-route-exists"
invariant = "POST /api/candidates/:id/correct exists."
type = "present"
pattern = '"/api/candidates/:id/correct"'
paths = ["backend/src/api.rs"]

[[check]]
id = "correct-requires-candidate-state"
invariant = "Only a candidate in the candidate state may be corrected."
type = "present"
pattern = 'candidate is already \\"'
paths = ["backend/src/api.rs"]

[[check]]
id = "correct-rejects-a-no-op-change"
invariant = "A correction that changes nothing is rejected rather than silently accepted."
type = "present"
pattern = "must actually change"
paths = ["backend/src/api.rs"]

[[check]]
id = "promotion-accepts-corrected-state-too"
invariant = "Promotion accepts a corrected candidate, not only an accepted one."
type = "present"
pattern = '!= "accepted" && current.validation_state != "corrected"'
paths = ["backend/src/api.rs"]

[[check]]
id = "frontend-offers-a-correct-control"
invariant = "CandidatesTable.tsx offers a Correct control with an editable type/statement form."
type = "present"
pattern = "Save Correction"
paths = ["frontend/src/components/CandidatesTable.tsx"]
```

## Notes

`cargo test` covers: correcting `statement` alone, `candidate_type` alone,
and both together, each producing a `corrected` row with exactly the
changed field(s) applied (reusing `transition_candidate`'s already-proven
generic override handling); rejecting a correction with no actual change
(`400`); rejecting an invalid `candidate_type` (`400`); rejecting a
correction on a candidate that is not in the `candidate` state (`409`);
promoting a `corrected` candidate succeeds exactly like promoting an
`accepted` one. `tsc --noEmit` and `vite build` pass. All tests use the
unique-candidate-id-lookup pattern against the shared development
database, never an aggregate count.
