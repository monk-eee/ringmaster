# EV-0031: Suggested Focus Blocks — group Obligations sharing a linked node

Evidence for [ADR-0031](../adr.d/0031-suggested-focus-blocks.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0031-suggested-focus-blocks"

[[check]]
id = "focus-blocks-route-groups-by-shared-node"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once GET /api/focus-blocks groups non-closed Obligations that share a linked node."

[[check]]
id = "single-linked-obligation-forms-no-block"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once a node linked to fewer than two non-closed Obligations is confirmed to form no block."

[[check]]
id = "closed-excluded-from-focus-blocks"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once a closed Obligation is confirmed absent from every block and never counted toward the two-or-more threshold."

[[check]]
id = "focus-blocks-card-exists"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once the Daily Brief tab renders a Suggested Focus Blocks card."
```

## Notes

Pre-implementation: all four checks are deliberately `manual`/unproven,
per this repo's own convention (evidence stays honest about intent vs.
proof until the ADR is accepted and implemented). Do not implement before
[ADR-0031](../adr.d/0031-suggested-focus-blocks.md)'s Status flips to
Accepted.
