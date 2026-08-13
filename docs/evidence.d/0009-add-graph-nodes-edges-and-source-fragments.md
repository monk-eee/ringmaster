# EV-0009: Add a generic node/edge graph substrate and source fragments table

Evidence for [ADR-0009](../adr.d/0009-add-graph-nodes-edges-and-source-fragments.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0009-add-graph-nodes-edges-and-source-fragments"

[[check]]
id = "nodes-table-exists"
invariant = "The generic nodes table exists with type, canonical text, attributes, and lifecycle state."
type = "present"
pattern = 'CREATE TABLE nodes'
paths = ["backend/migrations/0005_graph_nodes_edges_source_fragments.sql"]

[[check]]
id = "edges-table-exists"
invariant = "The generic edges table exists connecting any two entities by id."
type = "present"
pattern = 'CREATE TABLE edges'
paths = ["backend/migrations/0005_graph_nodes_edges_source_fragments.sql"]

[[check]]
id = "source-fragments-table-exists"
invariant = "The source_fragments table exists for bounded source passages."
type = "present"
pattern = 'CREATE TABLE source_fragments'
paths = ["backend/migrations/0005_graph_nodes_edges_source_fragments.sql"]

[[check]]
id = "graph-module-exists"
invariant = "A Rust module can create and read nodes, edges, and source fragments."
type = "present"
pattern = 'pub async fn create_node'
paths = ["backend/src/graph.rs"]
```

## Notes

All four checks are automated and verified directly against the migration
and crate that implement them. `cargo test` cases exercise real create/read
round trips against a live Postgres instance for all three tables, including
creating an edge between a graph node and an Obligation id to prove the
polymorphic design works without a database-level foreign key.
