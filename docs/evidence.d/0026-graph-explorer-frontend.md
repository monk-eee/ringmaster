# EV-0026: Graph explorer frontend — data entry, drill-down, and relationship visualization

Evidence for [ADR-0026](../adr.d/0026-graph-explorer-frontend.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0026-graph-explorer-frontend"

[[check]]
id = "graph-tab-exists"
invariant = "A Graph tab exists in the SPA alongside Obligations/Candidates/Search."
type = "present"
pattern = 'graph'
paths = ["frontend/src/App.tsx"]

[[check]]
id = "node-create-and-list-exist"
invariant = "The Graph tab can create a node and lists existing nodes."
type = "present"
pattern = 'createNode'
paths = ["frontend/src/api.ts"]

[[check]]
id = "node-detail-component-exists"
invariant = "Selecting a node shows a detail view with its attributes and one-hop neighbors."
type = "present"
pattern = 'neighbors'
paths = ["frontend/src/components/GraphExplorer.tsx"]

[[check]]
id = "svg-relationship-view-exists"
invariant = "The one-hop neighborhood is rendered as an SVG relationship diagram."
type = "present"
pattern = '<svg'
paths = ["frontend/src/components/GraphExplorer.tsx"]

[[check]]
id = "traversal-recenter-exists"
invariant = "Clicking a neighbor re-centers the view on it."
type = "present"
pattern = 'setSelectedNodeId'
paths = ["frontend/src/components/GraphExplorer.tsx"]
```

## Notes

All five checks are automated, verified directly against the implementing
frontend files. `npx tsc --noEmit` and `npm run build` both pass; manual
verification against the running dev stack confirms creating a node,
enriching it, adding an edge to a second node, and traversing between
them by clicking updates the rendered SVG view each time.
