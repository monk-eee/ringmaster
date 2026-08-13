# ADR-0005: Adopt a Rust service with an event-sourced Postgres commitment graph

- **Status:** Accepted
- **Date:** 2026-08-13
- **Decider:** monk-eee
- **Approval:** Explicitly accepted by monk-eee on 2026-08-13
- **Amended by:** [ADR-0007](0007-generalize-obligation-and-require-pgvector.md) (renames the aggregate from Commitment to Obligation; every other guarantee below is unchanged)
- **Depends on:** [ADR-0003](0003-ringmaster-ingests-mindleak-as-an-mcp-source.md), [ADR-0004](0004-defer-multi-user-access-control-single-user-v1.md)
- **Tags:** architecture, storage, data-model, rust, postgres

## Context

[docs/VISION.md § Technology direction](../VISION.md#technology-direction)
names Rust and an event-sourced Postgres commitment graph as intended
direction, and explicitly says that direction is not yet an accepted
engineering decision. [ADR-0003](0003-ringmaster-ingests-mindleak-as-an-mcp-source.md)
already assumes Ringmaster has its own internal commitment-graph
representation that MindLeak-derived facts are translated into.
[ADR-0004](0004-defer-multi-user-access-control-single-user-v1.md) fixes v1
to a single operator and a single local Postgres instance.

The vision's central claim is that **the commitment is the durable object**:
work items change, people move, teams reorganize, but the commitment
persists — and organizational memory must be able to answer, months later,
"was this completed, who owns it, is it still relevant." A storage model that
only keeps current state cannot answer "was this completed" once that state
is overwritten; it can only answer "is this completed now." Preserving that
history is the specific, load-bearing requirement this ADR exists to satisfy.

## Decision

- Ringmaster's core backend service must be implemented in Rust.
- Ringmaster must persist the commitment graph in Postgres as an
  event-sourced log: every change to a commitment (for example: created, a
  promise attached, a risk flagged, evidence attached, status changed,
  closed) must be appended as an immutable event row. Existing event rows
  must not be mutated or deleted to reflect a later change.
- **Commitment** is the core aggregate. It must have a stable identity that
  persists independently of the work items, people, or teams currently
  attached to it, matching the vision's "the commitment remains" principle.
  Its full history is the ordered sequence of its own events.
- Current or queryable state (for example, "is this commitment at risk," a
  7/30/60/90-day risk view) must be derived through projections/read models
  built from the event log. A projection must never be treated as more
  authoritative than the event log; if they disagree, the event log wins and
  the projection must be rebuilt.
- Links from a commitment to work items, people, dates, and source facts
  (including MindLeak-derived facts under [ADR-0003](0003-ringmaster-ingests-mindleak-as-an-mcp-source.md))
  must themselves be recorded as typed events referencing the commitment, not
  as foreign-key mutations against one mutable row.
- This ADR does not fix the specific event types, table layout, or migration
  files. Schema and event-type details may evolve through ordinary,
  ADR-0001-governed changes as long as they keep satisfying: the event log is
  the source of truth, commitment identity is stable, and projections remain
  rebuildable from events alone.

## Scope

**In scope:** the backend language (Rust); the storage engine (Postgres); the
event-sourcing pattern for the Commitment aggregate; the rule that
projections are derived, never authoritative.

**Out of scope:** the exact event/table schema and migrations; the MCP tool
surface exposed to agents; hosting or deployment beyond the single local
instance already fixed by [ADR-0004](0004-defer-multi-user-access-control-single-user-v1.md);
the specific event types needed for each commitment kind named in
[docs/VISION.md](../VISION.md#what-ringmaster-tracks) (delivery, leadership,
team, people, operational, personal) — those may be added incrementally
provided they fit this aggregate shape.

## Options considered

- **Event-sourced Postgres commitment graph in Rust (chosen):** directly
  supports "was this completed, who owns it, is it still relevant" by
  construction, since full commitment history remains queryable rather than
  overwritten; Rust matches the backend direction already named in the
  vision and suits a long-running local service.
- **Plain mutable-row (CRUD) model in Postgres:** simpler to build first, but
  loses history by construction — once a row is updated, the prior state is
  gone, which directly defeats the organizational-memory capability the
  vision names as its most important one.
- **A document store or native graph database instead of Postgres:** could
  model commitment relationships more natively, but introduces a second
  database technology beyond the single local Postgres instance
  [ADR-0004](0004-defer-multi-user-access-control-single-user-v1.md) already
  assumes, for no demonstrated current need.
- **A different backend language (for example, all-Node):** the vision
  already separates a Rust backend from a Node front end; re-deciding the
  backend language here would relitigate a split this ADR does not need to
  reopen.

## Consequences

- **Positive:** full commitment history is preserved by construction,
  directly enabling the organizational-memory queries the vision names;
  projections can be corrected or rebuilt without losing history; Rust gives
  the core service one settled runtime instead of an open question.
- **Negative / trade-off:** event sourcing costs more implementation and
  query complexity up front than a mutable-row model; projections require
  explicit maintenance and rebuild tooling that a CRUD model would not need.
- **Risk:** a poorly designed event vocabulary could still leave key
  questions hard to answer even with full history retained. Mitigated by
  keeping this ADR scoped to the pattern rather than freezing a specific
  schema, so the event vocabulary can iterate under ordinary ADR-governed
  review instead of requiring this decision to be reopened.

## Exit criteria and evidence

Evidence: [EV-0005](../evidence.d/0005-adopt-rust-event-sourced-postgres-commitment-graph.md)

| Exit criterion | Evidence |
|---|---|
| The vision's technology-direction section names this ADR as the accepted decision for backend/storage architecture | `vision-names-storage-decision` |
| The backend service is implemented in Rust | `backend-is-rust` |
| Commitment events are appended immutably, never mutated or deleted in place | `events-are-immutable` |
| Projections are documented as derived and rebuildable, never authoritative over the event log | `projections-are-derived` |
