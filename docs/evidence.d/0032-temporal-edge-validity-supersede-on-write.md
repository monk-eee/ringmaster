# EV-0032: Wire up edge temporal validity — supersede-on-write and relationship history in the Graph Explorer

Evidence for [ADR-0032](../adr.d/0032-temporal-edge-validity-supersede-on-write.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0032-temporal-edge-validity-supersede-on-write"

[[check]]
id = "supersede-defaults-false-and-is-unchanged"
invariant = "POST /api/edges with supersede omitted/false stores valid_from/valid_to as NULL, unchanged from today."
type = "present"
pattern = 'edge_create_route_without_supersede_leaves_valid_from_and_valid_to_null'
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "supersede-closes-prior-current-edge"
invariant = "POST /api/edges with supersede: true closes the prior current edge's valid_to and inserts the new edge as current."
type = "present"
pattern = 'edge_create_route_with_supersede_closes_the_prior_current_edge'
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "supersede-matches-from-id-and-edge-type"
invariant = "Supersede matching keys on (from_id, edge_type) only, not to_id."
type = "present"
pattern = 'WHERE from_id = \$2 AND edge_type = \$3 AND valid_to IS NULL'
paths = ["backend/src/graph/edge.rs"]

[[check]]
id = "edge-reads-include-validity-window"
invariant = "GET /api/nodes/:id neighbors and the POST /api/edges response both include valid_from/valid_to."
type = "present"
pattern = 'e\.valid_from, e\.valid_to'
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "graph-explorer-renders-temporal-edges"
invariant = "The Graph Explorer renders a superseded edge dashed/muted with an until label, and a current dated edge with a since label."
type = "present"
pattern = 'relationship-edge-superseded'
paths = ["frontend/src/components/GraphExplorer.tsx"]
```

## Notes

All five checks are automated against the implementing route/function/
component. `cargo test` covers: `supersede: false`/omitted leaving
`valid_from`/`valid_to` NULL exactly as before; `supersede: true` closing
the prior current edge and inserting the new one current; matching keyed
on `(from_id, edge_type)` only. Verified live end to end against the
reference example (a person `LIVES_IN` Barcelona from January, superseded
by `LIVES_IN` Madrid from March): the Barcelona edge's `valid_to` closed
to the Madrid edge's `valid_from`, both edges remain visible via
`GET /api/nodes/:id`, and the Graph Explorer renders Barcelona's edge
dashed with "LIVES_IN · UNTIL 3/1/2026" and Madrid's edge solid with
"LIVES_IN · SINCE 3/1/2026".
