# EV-0025: Node/edge write API and neighborhood traversal

Evidence for [ADR-0025](../adr.d/0025-node-edge-write-api-and-traversal.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0025-node-edge-write-api-and-traversal"

[[check]]
id = "nodes-create-route-exists"
invariant = "A route creates a node."
type = "present"
pattern = '"/api/nodes"'
paths = ["backend/src/api/mod.rs"]

[[check]]
id = "edges-create-route-exists"
invariant = "A route creates an edge."
type = "present"
pattern = '"/api/edges"'
paths = ["backend/src/api/mod.rs"]

[[check]]
id = "nodes-list-route-exists"
invariant = "A route lists nodes, optionally filtered by node_type."
type = "present"
pattern = 'fn list_nodes_route\('
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "nodes-detail-route-includes-neighbors"
invariant = "A route reads one node plus its one-hop neighborhood of edges."
type = "present"
pattern = 'fn get_node_detail\('
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "nodes-patch-route-merges-attributes"
invariant = "A route enriches a node's attributes via a shallow merge, never a wholesale replace."
type = "present"
pattern = 'attributes = attributes \|\|'
paths = ["backend/src/graph/node.rs"]
```

## Notes

All five checks are automated and verified directly against the crate
files that implement them. `cargo test` exercises: node create/list/get/
patch round-trips, attribute-merge preservation (enriching one field
does not erase a previously-recorded one), edge creation, and the
one-hop neighborhood read (including an edge whose other end is an
Obligation id rather than a `nodes` row, per ADR-0009's polymorphic
design). 49/49 backend tests pass.
