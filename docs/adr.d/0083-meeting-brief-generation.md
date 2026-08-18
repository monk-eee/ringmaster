# ADR-0083: Meeting-brief generation — a person's open commitments, recent asks, and risks in one call

- **Status:** Accepted
- **Date:** 2026-08-19
- **Decider:** monk-eee
- **Approval:** Direct instruction ("keep going"), continuing this session's established practice of drafting and implementing the next item from `docs/IMPROVEMENT-PLAN.md`'s priority order, 2026-08-19
- **Depends on:** [ADR-0028](0028-person-relationship-view.md), [ADR-0040](0040-dated-source-ingestion.md), [ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md), [ADR-0046](0046-unowned-obligation-risk-signal.md), [ADR-0054](0054-congruence-engine-v1-isolated-commitment-signal.md), [ADR-0069](0069-resolve-participants-to-person-nodes-at-ingestion.md), [ADR-0071](0071-person-detail-recent-interactions.md)
- **Tags:** api, mcp, composition

## Context

`docs/PRODUCT-SPEC.md` §8.3 names "Prepare a factual brief for my next
management 1:1, with sources" as an example agent query.
`docs/IMPROVEMENT-PLAN.md` §1.2 names this as the next-highest-leverage
unbuilt gap: the underlying evidence already exists (Person detail's
`relationship` obligations with `risk_signals`, per-obligation source
citations, `last_interaction_at`/`recent_interactions` per
[ADR-0071](0071-person-detail-recent-interactions.md)), but nothing composes
it into the single artifact a manager would actually carry into a 1:1.

Two of the three named pieces ("open commitments", "outstanding risks") are
already computed, verbatim, by `get_node_detail`'s existing person
`relationship` block (`backend/src/api/nodes.rs`) — the risk_signals/
reason logic is already factored into reusable free functions
(`risk_signals`, `daily_brief_reason` in `backend/src/api/obligations.rs`).
The third piece, "recent asks", does not exist anywhere: a candidate
(request/commitment/follow_up/risk/decision/expectation,
[ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md))
has no direct link to a person today. The join that makes this possible
without new extraction: `candidate_projection.source_fragment_id` →
`source_fragments.source_id` (the meeting/source node) →
a `participated_in` edge from that node to the person
([ADR-0069](0069-resolve-participants-to-person-nodes-at-ingestion.md)) —
three tables, all already populated, no schema change.

## Decision

- **A new, pure composition function**, `person_brief(pool, person_id)` in
  `backend/src/api/nodes.rs`, returning:
  - `person`: `{ id, canonical_text }` (404 if the id doesn't resolve to a
    node, or isn't a `person`).
  - `open_commitments`: every Obligation linked to this person by any edge
    (matching `get_node_detail`'s existing definition of "linked"), with
    `status != 'closed'`, each carrying `reason`
    ([ADR-0044](0044-today-attention-items-management-meaning.md)) and
    `risk_signals` computed by the exact existing `risk_signals()`
    function — the same "outstanding risk" definition Daily Brief, Time
    Horizon, and Person detail already share. Sorted by due-date urgency
    (`due_date_sort_key`, already private to this module and reused
    as-is).
  - `recent_asks`: candidates whose source meeting this person
    participated in (the new join above), excluding `rejected` (not a
    genuine management object,
    [ADR-0045](0045-correct-candidate-before-accepting.md)) and `promoted`
    (already represented in `open_commitments`), newest source
    `occurred_at` first, capped at 10 with an honest total (matching
    [ADR-0071](0071-person-detail-recent-interactions.md)'s capping
    precedent), each carrying its `statement`, `candidate_type`,
    `validation_state`, `confidence`, and source citation (`source_text`,
    `speaker`, `occurred_at`).
- **Exposed twice, same underlying function:** `GET /api/people/:id/brief`
  (HTTP, read-only, joining the existing REST surface's `/:id`-scoped
  pattern already used by `/api/obligations/:id`/`/api/meetings/:id`) and
  a new `prepare_meeting_brief` MCP tool
  (`backend/src/bin/ringmaster-ingest/mcp.rs`, matching the existing
  `get_entity`/`recall_sources` tool conventions — UUID input, `tool_error`/
  `json_success` helpers), directly answering PRODUCT-SPEC.md §8.3's named
  agent query.
- **No new extraction, no new table, no new edge type.** Every field is a
  read over data ingestion/extraction/promotion already write today; a
  person who has never had a request/commitment extracted about them, or
  no participated_in edges, gets empty lists, never a fabricated brief.

## Scope

**In scope:** `person_brief` in `nodes.rs`; the `GET /api/people/:id/brief`
route; the `prepare_meeting_brief` MCP tool; unit/integration tests for
both the open-commitments and recent-asks halves, including the
excluded-rejected/excluded-promoted/capped-with-honest-total cases.

**Out of scope, named honestly:** any new frontend page or panel (the
plan itself scopes this as "a read-only endpoint/MCP tool" first); name-
based person resolution in the MCP tool (accepts a UUID only, matching
every other entity-scoped MCP tool's convention); attaching
[ADR-0082](0082-repeated-concern-risk-signal.md)'s `repeated_concern`
signal to `recent_asks` entries (a natural, cheap follow-up once this
lands, not bundled in here to keep this record's diff reviewable);
narrative/prose rendering of the brief (`docs/IMPROVEMENT-PLAN.md` §2.1,
a separate, larger UX item); pagination on `recent_asks` beyond the fixed
10-item cap (matching, not exceeding, existing capping precedent); any
change to `get_node_detail`'s existing response shape or behavior.

## Options considered

- **A dedicated composition function/route/tool over already-existing
  data (chosen):** delivers exactly what PRODUCT-SPEC.md §8.3 and
  IMPROVEMENT-PLAN.md §1.2 ask for, reusing `risk_signals`/
  `daily_brief_reason` verbatim and adding exactly one new join (candidate
  → source fragment → meeting → participated_in → person) that doesn't
  exist elsewhere. Zero risk to `get_node_detail`'s existing behavior
  since it is not modified or refactored.
- **Refactor `get_node_detail`'s inline relationship logic into a shared
  helper first, then call it from both routes:** would reduce a small
  amount of duplication (the "obligations linked to a person" query
  shape), but risks a behavior change to an already-`PROVEN`, heavily
  tested route for a marginal DRY gain. Rejected for this record; a safe,
  no-behavior-change refactor could still be proposed separately if the
  duplication becomes a real maintenance cost.
- **Have the brief resolve a person by name, not just id:** more
  agent-ergonomic (an agent may know a name, not a UUID), but every
  existing MCP tool in this file resolves entities by UUID only, and
  `list_entities`/`get_entity` already exist for name→id discovery.
  Rejected to keep this record's surface consistent with established
  convention; revisit if this specific friction is reported.

## Exit criteria and evidence

| Exit criterion | Evidence |
|---|---|
| `person_brief` returns open commitments linked to the person, excluding closed, each with risk_signals | `brief-returns-open-commitments-with-risk-signals` |
| `person_brief` returns recent asks: candidates from meetings the person participated in, excluding rejected and promoted | `brief-recent-asks-excludes-rejected-and-promoted` |
| Recent asks are capped with an honest total, newest source first | `brief-recent-asks-capped-with-honest-total` |
| `GET /api/people/:id/brief` serves the same composition over HTTP | `http-route-serves-person-brief` |
| `prepare_meeting_brief` MCP tool serves the same composition | `mcp-tool-serves-person-brief` |
| `get_node_detail`'s existing response is unchanged | `node-detail-route-unchanged` |
