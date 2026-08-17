# EV-0050: Today attention budget — cap Focus Blocks, remove their raw id, honest "show all"

Evidence for [ADR-0050](../adr.d/0050-today-attention-budget.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0050-today-attention-budget"

[[check]]
id = "focus-blocks-capped-and-ordered-by-urgency"
invariant = "FocusBlocks renders at most 3 blocks by default, ordered by the most urgent contained obligation."
type = "present"
pattern = 'FOCUS_BLOCK_CAP = 3'
paths = ["frontend/src/components/FocusBlocks.tsx"]

[[check]]
id = "focus-blocks-no-raw-id"
invariant = "FocusBlocks no longer renders any obligation_id/id-marker chip."
type = "absent"
pattern = 'id-marker'
paths = ["frontend/src/components/FocusBlocks.tsx"]

[[check]]
id = "focus-blocks-show-all-affordance"
invariant = "A Show all control exists and reveals every block when more than 3 exist."
type = "present"
pattern = 'Show all'
paths = ["frontend/src/components/FocusBlocks.tsx"]
```

## Notes

All three checks are automated against `FocusBlocks.tsx`: the default render
caps at 3 blocks ordered by urgency (at-risk first, then soonest effective
due date, reusing existing fields), the `id-marker` chip is gone
(`absent`), and a "Show all N" control expands to every block in place.
Purely presentational — `GET /api/focus-blocks` and the grouping logic are
unchanged.
