# EV-0032: Wire up edge temporal validity — supersede-on-write and relationship history in the Graph Explorer

Evidence for [ADR-0032](../adr.d/0032-temporal-edge-validity-supersede-on-write.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0032-temporal-edge-validity-supersede-on-write"

[[check]]
id = "supersede-defaults-false-and-is-unchanged"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once POST /api/edges with supersede omitted/false stores valid_from/valid_to as NULL, unchanged from today."

[[check]]
id = "supersede-closes-prior-current-edge"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once POST /api/edges with supersede: true closes the prior current edge's valid_to and inserts the new edge as current."

[[check]]
id = "supersede-matches-from-id-and-edge-type"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once supersede matching is confirmed to key on (from_id, edge_type) only, not to_id."

[[check]]
id = "edge-reads-include-validity-window"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once GET /api/nodes/:id neighbors and the POST /api/edges response both include valid_from/valid_to."

[[check]]
id = "graph-explorer-renders-temporal-edges"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once the Graph Explorer renders a superseded edge dashed/muted with an until label, and a current dated edge with a since label."
```

## Notes

Pre-implementation: all five checks are deliberately `manual`/unproven,
per this repo's own convention (evidence stays honest about intent vs.
proof until the ADR is accepted and implemented). Do not implement before
[ADR-0032](../adr.d/0032-temporal-edge-validity-supersede-on-write.md)'s
Status flips to Accepted.
