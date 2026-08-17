# ADR-0052: Context-derived Focus Sessions — group by shared node *and* similar timeframe, not node alone

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Continuation of this session's established build pattern ("accept everything continue"), 2026-08-17
- **Depends on:** [ADR-0031](0031-suggested-focus-blocks.md), [ADR-0050](0050-today-attention-budget.md)
- **Amends:** [ADR-0031](0031-suggested-focus-blocks.md)'s grouping rule (shared node alone is no longer sufficient; a shared node's Obligations must also cluster in time to form one block)
- **Tags:** architecture, api, frontend

## Context

[ADR-0031](0031-suggested-focus-blocks.md) groups non-closed Obligations
into a "Focus Block" whenever they share any linked node — a real,
honest, non-fabricated signal, but a purely structural one. An
independent product review of [docs/current-status.md](../current-status.md)'s
audit named the resulting gap directly: *"Managers don't care that items
share a node. They care that work belongs together... same outcome, same
people, same time horizon — not because they happen to touch the same
graph object."* Concretely: a person node linked to a commitment due
tomorrow and an unrelated request from eight months ago currently forms
one "these belong together" block, when they plainly don't.

Detecting "same outcome" honestly would require a judgment [PRODUCT-SPEC.md](../PRODUCT-SPEC.md)'s
own extraction contract doesn't yet make (no `owner`/`counterparty`
resolution, [ADR-0040](0040-dated-source-ingestion.md)'s own named
deferral) and this repo's own posture throughout has been to never
fabricate a signal the data doesn't actually support. The one honest,
already-available second dimension is *time*: every Obligation this
groups already carries `hard_due_at`/`soft_due_at`, and Time Horizon
([ADR-0029](0029-time-horizon-view.md)) already buckets by exactly this.

## Decision

- **A Focus Block's Obligations must share both a linked node and the
  same Time Horizon bucket** ([ADR-0029](0029-time-horizon-view.md)'s
  existing buckets — overdue/this week/this month/later/no date). A node
  linked to Obligations spanning several buckets now forms one block per
  bucket it has 2+ Obligations in, not one block spanning all of them.
  Reuses `time_horizon_bucket` verbatim; no new bucketing logic.
- **Each block's label names both dimensions honestly**: *"\<node text\> —
  \<bucket label\>"* (e.g., "Roopa — Due this week"), replacing today's
  node-only label, so the reason two things are grouped is stated, not
  implied.
- **A node linked to Obligations in only one bucket behaves exactly as
  today** — this only splits blocks that were silently spanning
  unconnected timeframes; it never merges or invents a new grouping key.

## Scope

**In scope:** splitting a shared-node group by Time Horizon bucket;
labeling each block with both the node and the bucket.

**Out of scope, named honestly:**

- **Semantic "same outcome" clustering** (a model judging that two
  differently-worded Obligations serve the same underlying goal). No
  infrastructure exists for this and building it now would be exactly the
  kind of fabricated signal this repo's own principles reject.
- **Grouping across different nodes** (e.g., two people jointly owning one
  outcome). Requires real owner/counterparty modeling
  ([ADR-0040](0040-dated-source-ingestion.md)'s own deferred work), not
  decided here.
- **"Start Focus Session"** or any estimated-time/scheduling feature —
  [ADR-0031](0031-suggested-focus-blocks.md) already named this out of
  scope for the same reason (no real backing data); unchanged.

## Options considered

- **Split by shared node + Time Horizon bucket (chosen):** reuses
  [ADR-0029](0029-time-horizon-view.md)'s already-proven bucketing exactly;
  adds a real, honest second dimension without fabricating one.
- **Leave grouping as shared-node-only:** rejected — this is the exact gap
  the audit and the product review both named; leaving it unchanged fixes
  nothing.
- **Add a model-scored "relatedness" between Obligations:** rejected as
  premature and dishonest — no evidence this repo has the data or a
  validated model to back such a score; would violate the no-fabrication
  posture this project has held since [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md).

## Consequences

- **Positive:** "these belong together" becomes true in both the graph
  sense and the calendar sense, closing the review's core objection,
  using only fields that already exist.
- **Positive:** composes directly with [ADR-0050](0050-today-attention-budget.md)'s
  cap — fewer, more genuinely coherent blocks to choose the top 3 from.
- **Negative / trade-off:** a node previously forming one large block may
  now form several smaller ones (or none, if no bucket has 2+); this is
  the intended correction, named as a behavior change, not a regression.
- **Risk:** low. Reuses two already-proven functions
  (`time_horizon_bucket`, the existing node-join query); no schema
  change.

## Exit criteria and evidence

Evidence: [EV-0052](../evidence.d/0052-context-derived-focus-sessions.md)

| Exit criterion | Evidence |
|---|---|
| A shared node's Obligations spanning two Time Horizon buckets form two blocks, not one | `focus-blocks-split-by-time-horizon-bucket` |
| A shared node's Obligations all in one bucket still form exactly one block | `focus-blocks-single-bucket-unchanged` |
| Each block's label names both the node and the bucket | `focus-block-label-names-node-and-bucket` |
