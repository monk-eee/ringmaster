# EV-0028: Person relationship view — resolve linked Obligations into a per-person page

Evidence for [ADR-0028](../adr.d/0028-person-relationship-view.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0028-person-relationship-view"

[[check]]
id = "obligation-neighbor-resolves"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once GET /api/nodes/:id resolves an Obligation-typed edge target instead of returning a null neighbor."

[[check]]
id = "unknown-neighbor-still-null"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once the existing ADR-0025 test is updated to assert only a genuinely-unknown id (neither nodes nor obligation_projection) still reports a null neighbor."

[[check]]
id = "person-relationship-grouping"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once a person node's GET /api/nodes/:id response includes an at_risk/open grouped relationship object."

[[check]]
id = "relationship-view-component-exists"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once the Graph Explorer's detail panel renders a Relationship view for person nodes."
```

## Notes

Pre-implementation: all four checks are deliberately `manual`/unproven,
per this repo's own convention (evidence stays honest about intent vs.
proof until the ADR is accepted and implemented). Do not implement before
[ADR-0028](../adr.d/0028-person-relationship-view.md)'s Status flips to
Accepted.
