# ADR-0031: Suggested Focus Blocks — group Obligations sharing a linked node

- **Status:** Proposed
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Pending — awaiting monk-eee's decision
- **Depends on:** [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md), [ADR-0022](0022-daily-brief-endpoint.md), [ADR-0023](0023-evidence-backed-daily-brief-reasons.md), [ADR-0025](0025-node-edge-write-api-and-traversal.md), [ADR-0028](0028-person-relationship-view.md)
- **Tags:** architecture, api, frontend, data-model

## Context

[VISION.md § Congruence over completion](../VISION.md#congruence-over-completion----the-killer-widget)
names Suggested Focus Blocks monk-eee's own words: *"the most useful thing
in the application."* Its mockup:

> 🎯 **Reorg Transition** — these belong together: Transition Plan, Service
> Ownership, New Team Members, Knowledge Transfer... **[Start Focus
> Session]**

The same section explicitly separates this from the larger **Congruence
Engine** (drift between a stated commitment, its derived goals, and actual
work) — and says the Congruence Engine itself "deserves its own future
bounded ADR once the underlying obligation/work-item linkage exists to
detect it from" (i.e. ADO integration, not yet built). Suggested Focus
Blocks is different and smaller: its own text says the grouping "isn't
manual linking — it comes from the graph already knowing these share the
same people, the same services, the same meetings, the same dates" — data
[ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md)'s `edges`
table and [ADR-0028](0028-person-relationship-view.md)'s
Obligation-neighbor resolution already make available today. This ADR
builds that smaller, already-buildable piece; it is deliberately **not**
the Congruence Engine, and does not claim to detect drift, goal
misalignment, or missing work-item coverage.

## Decision

- A new read-only route, `GET /api/focus-blocks`: for every node (Person,
  Meeting, Risk, or any other type) linked via an edge to two or more
  non-closed Obligations, return one "block": the shared node's
  `id`/`node_type`/`canonical_text`, and its linked Obligations (each with
  `obligation_id`, `status`, `hard_due_at`, `soft_due_at`, and the same
  `reason` string [ADR-0022](0022-daily-brief-endpoint.md)/[ADR-0023](0023-evidence-backed-daily-brief-reasons.md)
  already compute — no new reasoning logic). A node linked to zero or one
  non-closed Obligation forms no block; a closed Obligation is never
  counted or shown, matching every prior list route's convention.
  Blocks are ordered by Obligation count descending (the most-connected
  block first) — the closest thing to "significance" this data can
  honestly support without a real scoring model.
- **No estimated time, no automatic "Start Focus Session" action.**
  VISION's mockup shows "Estimated effort: 90 mins" and a start button;
  neither is built here. A fabricated time estimate with no real basis
  (no time-tracking data exists anywhere in this system) would violate
  this repo's own "evidence before confidence, never fabricate" principle
  the same way a made-up due date or invented quote would. "Start Focus
  Session" implies session state (start/pause/complete) that doesn't
  exist and is a separate, undecided feature.
- **Frontend:** a "Suggested Focus Blocks" card rendered on the existing
  Daily Brief tab, above the ranked list (matching VISION's own framing
  that this sits alongside, not apart from, the Daily Brief screen).
  Each block shows the shared node's name and type icon
  ([ADR-0030](0030-human-readable-titles-and-type-iconography.md)),
  followed by its Obligations with the same status badge + reason
  presentation already used everywhere else. No blocks render nothing (an
  absent card, not an empty one), since most obligations won't yet be
  linked to any node — this is honestly a thin slice until real
  extraction-time linking exists.

## Scope

**In scope:** `GET /api/focus-blocks`; the grouping-by-shared-node query;
the Daily Brief's new card.

**Out of scope, named honestly (deferred, larger/separate work):** the
full Congruence Engine (commitment/goal/work-item drift detection) —
explicitly named in VISION.md as needing ADO integration first; any time
estimate or effort scoring; a "Start Focus Session" action or any session
state; grouping by shared *date* or shared *service* specifically (this
ADR groups by any shared node, regardless of type, which subsumes but
doesn't distinguish those cases); automatic linking of an Obligation to a
node at extraction time (still only possible via the existing manual
`POST /api/edges`, per [ADR-0028](0028-person-relationship-view.md)'s own
same limitation).

## Options considered

- **Group by any shared node via existing edges (chosen):** zero schema
  change, zero new reasoning logic, works today for any Obligation a user
  has already linked to a Person/Meeting/etc. via the existing write API.
- **Wait for automatic extraction-time linking:** would make blocks
  populate themselves without manual edge creation, but that's a materially
  larger, separate decision (extraction.rs schema/prompt changes) with no
  committed timeline — rejected for the same reason ADR-0028 rejected it.
- **Build the full Congruence Engine now (drift detection):** rejected
  outright — VISION.md's own text says the prerequisite (obligation/work-item
  linkage via ADO integration) does not exist yet; attempting it now would
  mean either fabricating a "goal" concept with no real backing data, or
  silently descoping into exactly this ADR's scope anyway.

## Consequences

- **Positive:** directly serves monk-eee's stated #4 priority, ships the
  specific piece VISION.md itself calls "already buildable from the
  graph," and does so with zero new reasoning logic (reuses
  `daily_brief_reason` verbatim) and zero schema change.
- **Negative / trade-off:** blocks stay empty until Obligations are
  manually linked to shared nodes (no automatic linking exists) — real,
  but the same honestly-named limitation ADR-0028 already accepted for
  the Relationship view.
- **Risk:** low. Purely additive read route; reuses existing, already-
  proven building blocks (the evidence-join pattern, the reason function)
  rather than adding new ones.

## Exit criteria and evidence

Evidence: [EV-0031](../evidence.d/0031-suggested-focus-blocks.md)

| Exit criterion | Evidence |
|---|---|
| `GET /api/focus-blocks` groups non-closed Obligations sharing a linked node | `focus-blocks-route-groups-by-shared-node` |
| A node linked to fewer than two non-closed Obligations forms no block | `single-linked-obligation-forms-no-block` |
| A closed Obligation is never counted or shown in any block | `closed-excluded-from-focus-blocks` |
| The Daily Brief tab renders a Suggested Focus Blocks card | `focus-blocks-card-exists` |
