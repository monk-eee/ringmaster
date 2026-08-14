# EV-0039: Product re-steer — Today/Timeline/People/Inbox as primary navigation

Evidence for [ADR-0039](../adr.d/0039-product-re-steer-primary-navigation.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0039-product-re-steer-primary-navigation"

[[check]]
id = "primary-nav-order-and-default"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once Today/Timeline/People/Inbox render as the primary tab group, in that order, with Today the default landing tab."

[[check]]
id = "secondary-nav-group-exists"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once Obligations/Graph/Search render as a visually distinct secondary/developer group, still present and unchanged in behavior."

[[check]]
id = "today-page-renders-required-sections"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once the Today page renders, in order: a greeting/summary, the capped ranked list, a labeled Do These Together section, and a compact coming-soon strip."

[[check]]
id = "people-tab-lists-and-opens-relationship-data"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once the People tab lists person nodes and opens each into its existing relationship data via GET /api/nodes/:id, with no new backend route."

[[check]]
id = "inbox-is-relabeled-candidates"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once the Inbox tab is confirmed to be the relabeled Candidates route/actions, unchanged in behavior."

[[check]]
id = "no-new-backend-or-dependency"
invariant = "No new backend route, migration, or frontend dependency was added by this ADR."
type = "manual"
rationale = "A negative claim (no new route/migration/dependency) is not reliably provable by a positive regex match; verify by direct review of the implementation diff (no new backend/migrations/*.sql, no new .route() registration, no new frontend/package.json dependency) once implemented, matching EV-0021's and EV-0033's own precedent for the same kind of claim."
```

## Notes

Pre-implementation: the first five checks are deliberately `manual`/
unproven, per this repo's own convention (evidence stays honest about
intent vs. proof until the ADR is accepted and implemented). The sixth
stays a reasoned `manual` check even after implementation, for the same
reason EV-0021/EV-0033 keep an equivalent claim manual. Do not implement
before [ADR-0039](../adr.d/0039-product-re-steer-primary-navigation.md)'s
Status flips to Accepted.
