# ADR-0046: Unowned-obligation risk signal via existing `owns` edges

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Continuation of this session's established build pattern for well-scoped, low-risk, evidence-grounded additions; corrects ADR-0041's own scoping claim with a concrete finding, 2026-08-17
- **Depends on:** [ADR-0025](0025-node-edge-write-api-and-traversal.md), [ADR-0028](0028-person-relationship-view.md), [ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md)
- **Amends:** [ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md)'s scope claim that "unowned obligation" needs "an owner/accountable field" that "doesn't exist yet" — it does not; an owning link already exists via the general-purpose `edges` table.
- **Tags:** architecture, api, frontend

## Context

[ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md)
named `date_compression` and `stale` as the only two of
[PRODUCT-SPEC.md §7.1](../PRODUCT-SPEC.md#71-initial-risk-signals)'s nine
risk signals "derivable today with zero schema change," and listed
**unowned obligation** among the seven it judged blocked on "a concept or
data source that doesn't exist yet." That claim was wrong: `owns` is
already a real, exercised edge type.
[ADR-0028](0028-person-relationship-view.md)'s own test
(`person_relationship_view_groups_obligations_by_status`) already links a
person node to an Obligation with
`graph::create_edge(pool, person_id, obligation_id, "owns", None)`, and
`GET /api/nodes/:id` already resolves exactly this edge to render a
person's owned Obligations. "A request is accepted but no accountable
owner is linked" ([PRODUCT-SPEC.md §7.1](../PRODUCT-SPEC.md#71-initial-risk-signals))
is therefore answerable today with one `EXISTS` check against `edges`,
not a new field.

This does not reopen or weaken anything ADR-0041 decided about the other
six named signals (repeated concern, work disconnect, outcome disconnect,
coverage gap, cadence lapse, contradiction). Each of those still names a
real, currently-missing data source or concept (cross-meeting semantic
matching, ADO integration, customer-outcome nodes, calendar ingestion, a
recurrence field, cross-evidence conflict detection) and stays exactly as
out of scope as ADR-0041 left it.

## Decision

- `risk_signals` gains one new parameter, `has_owner: bool`, computed by
  the caller (not by this pure function) from whether any `edges` row
  links a `person`-type node to this Obligation as `from_id`/`to_id ==
  obligation_id` with `edge_type = 'owns'` — the same direction
  [ADR-0028](0028-person-relationship-view.md)'s own precedent already
  uses. When `false`, the function pushes one more independent signal:
  `unowned`, with a plain explanation ("No owner linked."). This mirrors
  `date_compression`/`stale`'s existing shape exactly: deterministic,
  no fabricated confidence or severity.
- `GET /api/daily-brief` and `GET /api/time-horizon` each add one
  read-only `EXISTS` subquery against `edges`/`nodes` to compute
  `has_owner` per row, alongside the fields they already select, then
  pass it into the same shared `risk_signals` call both routes already
  make. No new route, no duplicated query logic between the two routes.
- No frontend change. Both `DailyBrief.tsx` and `TimeHorizon.tsx` already
  render every entry of `item.risk_signals` generically
  (`<li key={signal.signal}>{signal.explanation}</li>`) with no
  per-signal-name branching — a new signal value renders correctly with
  zero component changes, exactly the "additive field" design ADR-0041
  already established.

## Scope

**In scope:** the `has_owner` parameter and `unowned` signal on
`risk_signals`; the `EXISTS` subquery on both existing routes.

**Out of scope, unchanged from ADR-0041:** the other six named signals
and any combined severity score — still genuinely blocked on data/concepts
that do not exist, as ADR-0041 already stated; any change to Daily
Brief ranking or Time Horizon bucketing (this stays an additive,
informational field only, matching ADR-0041's own posture); resolving
*which* person owns an Obligation beyond "at least one does/doesn't" —
this signal is binary, not a full ownership assignment UI; any new way to
*create* an `owns` edge (the existing Graph Explorer "Add relationship"
form, [ADR-0026](0026-graph-explorer-frontend.md), already does this).

## Options considered

- **One `EXISTS` subquery per route, reusing the existing `owns` edge
  type (chosen):** zero schema change, zero new route, corrects a wrong
  scoping claim with the smallest possible fix.
- **Add a dedicated `owner_id` column to `obligation_projection`:**
  rejected — would duplicate identity `edges` already models, and
  contradicts this repository's own established graph-substrate
  precedent ([ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md))
  of representing relationships as edges, not per-aggregate foreign keys.
- **Leave "unowned obligation" out of scope, as ADR-0041 originally
  claimed:** rejected now that the claim is known to be false — leaving
  a known-buildable, evidence-backed signal unbuilt would itself be
  dishonest by this repository's own evidence discipline.

## Consequences

- **Positive:** 3 of 9 §7.1 signals now real; corrects a factual error in
  an accepted ADR through the proper channel (a new amending record, not
  a rewrite); zero schema/dependency change; zero frontend change.
- **Negative / trade-off:** "unowned" only checks for *any* `owns` edge
  to *any* person, not whether that person is still the *right* or
  *active* owner — an honest, binary signal, not a full accountability
  model.
- **Risk:** low. One additive `EXISTS` subquery per already-read-only
  route; the signal function itself stays pure and directly unit-tested.

## Exit criteria and evidence

Evidence: [EV-0046](../evidence.d/0046-unowned-obligation-risk-signal.md)

| Exit criterion | Evidence |
|---|---|
| `risk_signals` pushes `unowned` when `has_owner` is false, and never when true | `unowned-signal-is-a-pure-function-of-has-owner` |
| `GET /api/daily-brief` flags an Obligation with no `owns` edge as unowned, and does not flag one that has one | `daily-brief-computes-has-owner-via-owns-edge` |
| `GET /api/time-horizon` does the same | `time-horizon-computes-has-owner-via-owns-edge` |
| No frontend change was needed or made | `no-frontend-change-required` |
