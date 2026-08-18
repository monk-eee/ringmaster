# EV-0031: Suggested Focus Blocks — group Obligations sharing a linked node

Evidence for [ADR-0031](../adr.d/0031-suggested-focus-blocks.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0031-suggested-focus-blocks"

[[check]]
id = "focus-blocks-route-groups-by-shared-node"
invariant = "GET /api/focus-blocks groups non-closed Obligations that share a linked node."
type = "present"
pattern = '"/api/focus-blocks"'
paths = ["backend/src/api/obligations.rs"]

[[check]]
id = "single-linked-obligation-forms-no-block"
invariant = "A node linked to fewer than two non-closed Obligations forms no block."
type = "present"
pattern = 'block.obligations.len\(\) >= 2'
paths = ["backend/src/api/obligations.rs"]

[[check]]
id = "closed-excluded-from-focus-blocks"
invariant = "A closed Obligation is never counted or shown in any block."
type = "present"
pattern = "op.status <> 'closed'"
paths = ["backend/src/api/obligations.rs"]

[[check]]
id = "focus-blocks-card-exists"
invariant = "The Daily Brief tab renders a Suggested Focus Blocks card."
type = "present"
pattern = 'export default function FocusBlocks'
paths = ["frontend/src/components/FocusBlocks.tsx"]
```

## Notes

All four checks are automated against the route module and frontend
component that implement them. `cargo test` covers: two non-closed
Obligations linked to the same node form a block with both reasons
present (reusing `daily_brief_reason` verbatim); a node linked to only
one non-closed Obligation forms no block; a closed Obligation is excluded
from the count and from the block's obligation list even when two other
open Obligations still form one. Verified live: linking two Obligations
to the same person node via `POST /api/edges` produces a Suggested Focus
Blocks card above the Daily Brief's ranked list. `tsc --noEmit`, `vite
build`, and all 5 Playwright tests pass; 61/61 backend tests pass.
[ADR-0031](../adr.d/0031-suggested-focus-blocks.md)'s Status flips to
Accepted.
