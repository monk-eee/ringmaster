# EV-0030: Human-readable titles and type iconography across the UI

Evidence for [ADR-0030](../adr.d/0030-human-readable-titles-and-type-iconography.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0030-human-readable-titles-and-type-iconography"

[[check]]
id = "obligations-table-renders-evidence-text"
invariant = "ObligationsTable renders the linked source fragment's evidence text as primary content, not a raw id."
type = "present"
pattern = 'obligation-title'
paths = ["frontend/src/components/ObligationsTable.tsx"]

[[check]]
id = "type-icon-map-exists"
invariant = "A shared typeIcon function covers every PRODUCT-SPEC node type and candidate type."
type = "present"
pattern = 'export function typeIcon\('
paths = ["frontend/src/icons.ts"]

[[check]]
id = "type-icon-wired-into-graph-and-candidates"
invariant = "GraphExplorer and CandidatesTable render a type icon alongside their existing color tags."
type = "present"
pattern = 'typeIcon\('
paths = ["frontend/src/components/GraphExplorer.tsx", "frontend/src/components/CandidatesTable.tsx"]
```

## Notes

All three checks are automated and verified directly against the
implementing frontend files. `tsc --noEmit` and `vite build` both pass.
