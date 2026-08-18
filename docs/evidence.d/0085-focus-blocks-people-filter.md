# EV-0085: Focus Sessions filter to People-linked blocks — the one honestly-groundable attention-type slice

Evidence for [ADR-0085](../adr.d/0085-focus-blocks-people-filter.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0085-focus-blocks-people-filter"

[[check]]
id = "focus-blocks-people-filter-toggle"
invariant = "A People/All toggle filters Focus Blocks by node_type === \"person\"."
type = "present"
pattern = 'node_type === "person"'
paths = ["frontend/src/components/FocusBlocks.tsx"]

[[check]]
id = "focus-blocks-toggle-hidden-when-uniform"
invariant = "The toggle only renders when both People and non-People blocks exist."
type = "present"
pattern = "hasPeopleBlocks"
paths = ["frontend/src/components/FocusBlocks.tsx"]

[[check]]
id = "playwright-proves-focus-blocks-people-filter"
invariant = "Existing ordering/capping/\"Show all\" behavior is unchanged for either filter state."
type = "present"
pattern = "focus blocks: People/All filter"
paths = ["frontend/tests/obligations.spec.ts"]
```

## Notes

Implemented: `frontend/src/components/FocusBlocks.tsx` adds a
`peopleOnly` toggle rendered only when `hasPeopleBlocks &&
hasNonPeopleBlocks` (both truthy), filtering the already-fetched `blocks`
array by `node_type === "person"` before the existing rank/cap logic
runs. No backend change -- `GET /api/focus-blocks` already returns
`node_type` per block (ADR-0031).

Verified: `npx tsc --noEmit` and `npm run build` both clean. Two
Playwright tests in `frontend/tests/obligations.spec.ts` --
`focus blocks: People/All filter shows and hides person-linked blocks
(ADR-0085)` and `focus blocks: People/All filter toggle is hidden when
every block shares one attention type (ADR-0085)` -- both pass in
isolation and as part of the full suite (19 passed, 5 pre-existing
skips, 0 failed; two unrelated graph tests transiently failed once under
full-suite concurrent-session load and passed cleanly on immediate
re-run in isolation, consistent with this repo's known shared-database
contention across concurrent sessions, not a regression from this
change).
