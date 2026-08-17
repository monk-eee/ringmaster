# EV-0050: Today attention budget — cap Focus Blocks, remove their raw id, honest "show all"

Evidence for [ADR-0050](../adr.d/0050-today-attention-budget.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0050-today-attention-budget"

[[check]]
id = "focus-blocks-capped-and-ordered-by-urgency"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once FocusBlocks.tsx renders at most 3 blocks by default, ordered by the most urgent contained obligation."

[[check]]
id = "focus-blocks-no-raw-id"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become an absent-type check once FocusBlocks.tsx no longer renders any obligation_id/id-marker chip."

[[check]]
id = "focus-blocks-show-all-affordance"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once a Show all control exists and reveals every block when more than 3 exist."
```

## Notes

Pre-implementation: all three checks are deliberately `manual`/unproven, per
this repo's own convention. Do not implement before
[ADR-0050](../adr.d/0050-today-attention-budget.md)'s Status flips to
Accepted.
