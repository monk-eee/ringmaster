# EV-0027: Promote an accepted candidate into an Obligation

Evidence for [ADR-0027](../adr.d/0027-promote-accepted-candidate-to-obligation.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0027-promote-accepted-candidate-to-obligation"

[[check]]
id = "promote-route-exists"
invariant = "A route promotes an accepted candidate into a new Obligation and rejects any other validation_state with 409."
type = "present"
pattern = '"/api/candidates/:id/promote"'
paths = ["backend/src/api/mod.rs"]

[[check]]
id = "promoted-obligation-id-column-exists"
invariant = "candidate_projection carries the linked Obligation id forward after promotion."
type = "present"
pattern = 'promoted_obligation_id'
paths = ["backend/migrations/0011_candidate_promoted_obligation.sql"]

[[check]]
id = "candidates-table-has-promote-control"
invariant = "The Candidates table offers a promote action for accepted candidates and shows the linked Obligation once promoted."
type = "present"
pattern = 'promoteCandidate'
paths = ["frontend/src/components/CandidatesTable.tsx"]
```

## Notes

All three checks are automated and verified directly against the route,
migration, and frontend component that implement them. `cargo test`
covers: promoting an accepted candidate creates an `open` Obligation
carrying its `source_fragment_id` forward and marks the candidate
`promoted` with `promoted_obligation_id` linked; `409` for a candidate
still in `candidate` state, already `rejected`, or already `promoted`;
`404` for an unknown candidate. 55/55 backend tests pass; `tsc --noEmit`
and `vite build` both pass on the frontend.
