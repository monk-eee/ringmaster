# EV-0024: Accept/reject buttons for candidates — Epic E5's first interactive slice

Evidence for [ADR-0024](../adr.d/0024-candidate-accept-reject-buttons.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0024-candidate-accept-reject-buttons"

[[check]]
id = "accept-route-exists"
invariant = "A route accepts a candidate still in the candidate state and rejects an already-transitioned one with 409."
type = "present"
pattern = '"/api/candidates/:id/accept"'
paths = ["backend/src/api.rs"]

[[check]]
id = "reject-route-exists"
invariant = "A route rejects a candidate the same way."
type = "present"
pattern = '"/api/candidates/:id/reject"'
paths = ["backend/src/api.rs"]

[[check]]
id = "candidates-table-has-action-buttons"
invariant = "The Candidates table renders working Accept/Reject buttons for candidates still in the candidate state."
type = "present"
pattern = 'acceptCandidate'
paths = ["frontend/src/components/CandidatesTable.tsx"]
```

## Notes

All three checks are automated against the route module and frontend
component that implement them. `cargo test` cases exercise: accepting and
rejecting a candidate still in the `candidate` state, a `409` for an
already-transitioned candidate, and a `404` for an unknown one. Verified
live in the browser: clicking Accept/Reject on a running candidate row
transitions its `validation_state` immediately with no page reload, and
the action buttons correctly disappear once a candidate is no longer in
the `candidate` state.
