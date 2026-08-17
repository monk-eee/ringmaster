# ADR-0050: Today attention budget — cap Focus Blocks, remove their raw id, honest "show all"

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Continuation of this session's established build pattern ("accept everything continue"), 2026-08-17
- **Depends on:** [ADR-0031](0031-suggested-focus-blocks.md), [ADR-0039](0039-product-re-steer-primary-navigation.md), [ADR-0044](0044-today-attention-items-management-meaning.md)
- **Tags:** frontend, architecture

## Context

[docs/current-status.md](../current-status.md)'s audit found the Today
page's main ranked list correctly capped at 10 items
([ADR-0039](0039-product-re-steer-primary-navigation.md)/[ADR-0044](0044-today-attention-items-management-meaning.md)),
but the "Do these together" section right below it (`FocusBlocks`,
[ADR-0031](0031-suggested-focus-blocks.md)) has no cap at all — with the
dev database's current volume it rendered all 110 focus blocks
unfiltered, and each block still shows a raw truncated-UUID chip
(`item.obligation_id.slice(0, 8)…` in `FocusBlocks.tsx`) that
[ADR-0044](0044-today-attention-items-management-meaning.md) believed it
had removed from Today. It removed it from the ranked list; the same page
still leaks an id a few inches lower. An independent product review of
that audit reached the same conclusion in stronger terms: *"the attention
budget gets destroyed"* when a page says "here are the 10 things that
matter" immediately followed by 110 more things.

## Decision

- **`FocusBlocks` renders at most 3 blocks by default**, ordered by the
  same urgency signal the ranked list already uses (the block containing
  the most urgent obligation — soonest `hard_due_at`/`soft_due_at`, then
  `at_risk` status — sorts first; reuses existing fields, no new scoring
  model).
- **The raw id chip is removed from `FocusBlocks`' obligation rows.** Each
  row instead shows the same `reason` string it already computes
  (`daily_brief_reason`, already present in the API response) — matching
  [ADR-0044](0044-today-attention-items-management-meaning.md)'s existing
  "management meaning, not identifiers" principle, applied to the one
  place on Today it wasn't.
- **An honest "Show all N" affordance** appears when more than 3 blocks
  exist, mirroring `DailyBrief`'s existing "N more in Timeline" pattern —
  clicking it reveals the rest in place, never a silent truncation.
- No backend change: `GET /api/focus-blocks` already returns every block
  and every obligation within it; the cap and expansion are frontend-only.

## Scope

**In scope:** capping `FocusBlocks`' default render to 3; removing its raw
id chip; a "Show all N" expand affordance.

**Out of scope, named honestly:**

- **Changing what makes a Focus Block** (currently: 2+ non-closed
  Obligations sharing any linked node). That is
  [ADR-0052](0052-context-derived-focus-sessions.md)'s question, not this
  one's — this ADR only bounds how many render and what each row shows.
- **A numeric cap on the People, Obligations, or Candidates list views.**
  [docs/current-status.md](../current-status.md) named the same
  no-pagination pattern there; each is real, separate follow-up work
  (People's is [ADR-0051](0051-relationship-workspace.md)'s concern).
- **Changing `GET /api/focus-blocks`'s response shape.** It already
  returns everything; capping happens in the client.

## Options considered

- **Cap at 3, reuse existing urgency ordering and reason strings (chosen):**
  matches `DailyBrief`'s already-proven pattern exactly; no new backend
  logic; directly closes the gap the audit and the product review both
  named.
- **Cap at some other number (5, 10):** rejected — 3 matches the product
  review's own explicit suggestion and keeps "Do these together" honestly
  secondary to the primary 10-item ranked list above it, not a second
  competing list of the same size.
- **Leave Focus Blocks uncapped but only remove the id chip:** rejected —
  removing the id without bounding the count still leaves a page that
  says "10 things matter" then dumps 110 more; the count is the actual
  attention-budget violation, not just the id.

## Consequences

- **Positive:** Today becomes an honest attention budget end-to-end, not
  just in its primary list — directly closes a gap this session's own
  audit found and an independent review confirmed.
- **Negative / trade-off:** none identified — purely a frontend rendering
  change; no data is hidden permanently, only collapsed behind an explicit
  "Show all."
- **Risk:** low. No backend/schema change; reuses existing fields and an
  already-proven UI pattern.

## Exit criteria and evidence

Evidence: [EV-0050](../evidence.d/0050-today-attention-budget.md)

| Exit criterion | Evidence |
|---|---|
| `FocusBlocks` renders at most 3 blocks by default, ordered by urgency | `focus-blocks-capped-and-ordered-by-urgency` |
| No raw obligation id is rendered anywhere in `FocusBlocks` | `focus-blocks-no-raw-id` |
| A "Show all N" control appears and reveals the rest when more than 3 blocks exist | `focus-blocks-show-all-affordance` |
