# EV-0027: Promote an accepted candidate into an Obligation

Evidence for [ADR-0027](../adr.d/0027-promote-accepted-candidate-to-obligation.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0027-promote-accepted-candidate-to-obligation"

[[check]]
id = "promote-route-exists"
invariant = "A route promotes an accepted candidate into a new Obligation and rejects any other validation_state with 409."
type = "manual"
# Not yet implemented. ADR-0027 is Proposed; replace with a `present`
# check against "/api/candidates/:id/promote" in backend/src/api.rs once
# the route exists.

[[check]]
id = "promoted-obligation-id-column-exists"
invariant = "candidate_projection carries the linked Obligation id forward after promotion."
type = "manual"
# Not yet implemented. Replace with a `present` check against
# "promoted_obligation_id" in the migration that adds the column, once it
# exists.

[[check]]
id = "candidates-table-has-promote-control"
invariant = "The Candidates table offers a promote action for accepted candidates and shows the linked Obligation once promoted."
type = "manual"
# Not yet implemented. Replace with a `present` check against a promote
# call site in frontend/src/components/CandidatesTable.tsx once it exists.
```

## Notes

All three checks are honestly `manual` because no implementation exists
yet — ADR-0027 is `Proposed`, not `Accepted`. Per this repository's own
evidence policy ([ADR-0002](../adr.d/0002-keep-current-evidence-separate-from-accepted-decisions.md)),
these must become declarative `present`/`absent` checks as each piece
lands, not be marked proven in advance.
