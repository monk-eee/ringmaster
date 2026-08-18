# ADR-0085: Focus Sessions filter to People-linked blocks — the one honestly-groundable attention-type slice

- **Status:** Accepted
- **Date:** 2026-08-19
- **Decider:** monk-eee
- **Approval:** Direct instruction ("ok makes sense do it"), confirming the next item from `docs/IMPROVEMENT-PLAN.md`'s suggested order (§2.2) after this session's own scoping analysis, 2026-08-19
- **Depends on:** [ADR-0031](0031-suggested-focus-blocks.md), [ADR-0052](0052-context-derived-focus-sessions.md)
- **Tags:** frontend, ux

## Context

`docs/IMPROVEMENT-PLAN.md` §2.2 names a real gap: Focus Blocks
([ADR-0031](0031-suggested-focus-blocks.md)/[ADR-0052](0052-context-derived-focus-sessions.md))
group by shared node and Time Horizon bucket, but `docs/VISION.md`
describes a complementary grouping "by *kind of attention* (People /
Delivery / Leadership / Operations), so a manager can run a 'People Focus
Session' end to end instead of context-switching between unrelated
obligation types."

That full four-category taxonomy is not buildable honestly today.
[ADR-0082](0082-repeated-concern-risk-signal.md) already confirmed
Obligation carries no `kind`/type column, and nothing else in this
codebase stores a Delivery/Leadership/Operations/People classification —
`candidate_type` (commitment/request/risk/follow_up/decision/expectation,
[ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md))
is an extraction category, not a domain one. Inventing a mapping from
`node_type` to those four labels (e.g., deciding a `connect` node means
"Leadership") would be exactly the kind of fabrication this repo's
conventions consistently refuse — a judgment call with no real basis to
check it against, the same reasoning [ADR-0054](0054-congruence-engine-v1-isolated-commitment-signal.md)
used to ship only the narrow, honest "isolated" signal instead of the
full Congruence Engine.

One category *is* honestly groundable today, with zero invention: a Focus
Block's shared node is either a real `person` node or it is not — a fact
`GET /api/focus-blocks` already returns as `node_type` on every block
([ADR-0031](0031-suggested-focus-blocks.md)). "People" is also the exact
example `docs/IMPROVEMENT-PLAN.md` itself names ("run a 'People Focus
Session' end to end"). This record ships that one slice and names the
rest honestly as not yet possible without a real domain-category decision.

## Decision

- **A People/All filter toggle above the Focus Blocks list** (matching
  the existing People tab's `needs_attention`/"Show everyone" toggle
  pattern), filtering the already-fetched `blocks` array client-side by
  `node_type === "person"`. No new backend field, no new route — `GET
  /api/focus-blocks` already returns `node_type` per block.
- **The toggle only renders when it would change anything**: at least one
  block is person-linked *and* at least one is not. A repo with only
  People-linked blocks, or none at all, is unaffected — no useless control
  offered to filter nothing.
- **Existing ordering, capping, and "Show all N" behavior are unchanged**,
  applied to whichever set (all blocks, or the People-filtered subset) is
  currently selected.

## Scope

**In scope:** a client-side People/All filter toggle in
`FocusBlocks.tsx`; conditional rendering of the toggle only when both
sides are non-empty.

**Out of scope, named honestly:** Delivery/Leadership/Operations
categories (no real `kind` concept exists to ground them in; a future
ADR would need to first decide how/when an Obligation gets classified,
which is a materially larger decision than this record's scope); any
change to `GET /api/focus-blocks`'s response shape (already returns
`node_type`); any change to Focus Block grouping, ordering, or the
Time Horizon bucket split ADR-0052 already established; a dedicated
"Start People Focus Session" mode or flow (VISION.md's "[Start Focus
Session]" button — no real backing action exists for it, matching
ADR-0031's own already-stated scope limit).

## Options considered

- **A People/All filter toggle over the existing `node_type` field
  (chosen):** delivers exactly the example `docs/IMPROVEMENT-PLAN.md`
  names, with zero invented categories and zero backend change.
- **Invent a full People/Delivery/Leadership/Operations mapping from
  `node_type` or `candidate_type` heuristics:** rejected — there is no
  real classification to check such a mapping against; it would be a
  judgment call presented as fact, which this repo's own evidence and
  ADR discipline exists to prevent.
- **Add a real `kind` column to Obligations, set at extraction/promotion
  time:** the actually-correct way to eventually build the full
  taxonomy, but a materially larger decision (model prompt changes,
  migration, promotion-time logic) than this bounded record should carry;
  left as an explicit future ADR if the four-category grouping is still
  wanted once a real classification source exists.

## Exit criteria and evidence

| Exit criterion | Evidence |
|---|---|
| A People/All toggle filters Focus Blocks by node_type === "person" | `focus-blocks-people-filter-toggle` |
| The toggle only renders when both People and non-People blocks exist | `focus-blocks-toggle-hidden-when-uniform` |
| Existing ordering/capping/"Show all" behavior is unchanged for either filter state | `playwright-proves-focus-blocks-people-filter` |
