# ADR-0007: Generalize the event-sourced aggregate to Obligation and require pgvector

- **Status:** Accepted
- **Date:** 2026-08-13
- **Decider:** monk-eee
- **Approval:** Direct instruction ("resolve data model and enforce pgvector"), 2026-08-13
- **Amends:** [ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md)
- **Depends on:** [ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md)
- **Tags:** architecture, storage, data-model, pgvector

## Context

[docs/PRODUCT-SPEC.md](../PRODUCT-SPEC.md) (v0.2) names **Obligation** as the
primary entity, with Commitment as an explicit promise subtype, and requires
pgvector as a mandatory dependency, not an optional enhancement (§1, §3.1,
§9.2, §15). [ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md)
is Accepted and already implemented and tested: an immutable, append-only
`commitment_events` log with a `commitment_projection` read model that is
always fully truncated and rebuilt from that log, and its Decision section
literally names Commitment as the core aggregate.

The spec's §9.2 table describes `obligations` with `status`/`validation
state` fields that read as updated in place, plus separate `evidence_events`
and `audit_events` tables — on a literal reading, a different guarantee from
"state is never stored, only derived from an immutable log." Read
charitably, it is compatible: an `obligations` row is exactly what
`commitment_projection` already is (a derived, rebuildable view), and a
domain event log by any name is what `commitment_events` already is. The
event-sourcing guarantee is strictly stronger for the spec's own stated goal
of preserving provenance and answering "was this completed, is it still
relevant" than a mutable-status-plus-audit-log pattern would be, so this ADR
keeps ADR-0005's guarantee rather than weakening it, and resolves the
terminology gap instead.

## Decision

- The event-sourced aggregate ADR-0005 established is renamed from
  Commitment to **Obligation**, matching the spec's primary entity.
  Commitment remains a promise subtype of Obligation, not a separate
  aggregate; today that distinction lives in the free-text `event_type` /
  future `obligation_type` field, not a separate table.
- Every other guarantee in ADR-0005 continues to hold under the new name:
  events are appended immutably (the database rejects mutation and
  deletion), and current state is derived by fully rebuilding a projection
  from the event log, never treated as authoritative.
- `commitment_events` and `commitment_projection` are renamed to
  `obligation_events` and `obligation_projection`. Since this repository has
  no commits yet and these tables carry no real data, the migration files
  are replaced directly rather than layered with a rename migration.
- pgvector is a required Postgres extension. A migration enabling it must
  run before any other migration that depends on it, and migrations must
  fail if the extension cannot be created — that failure is how "mandatory"
  is enforced today, ahead of a real embedding pipeline.
- A minimal `embeddings` table (entity id, entity type, model id, vector,
  source hash, created at) is added per [docs/PRODUCT-SPEC.md §9.2](../PRODUCT-SPEC.md#92-postgresql-and-pgvector).
  Its vector column is left dimension-unconstrained: no embedding model has
  been chosen yet ([docs/PRODUCT-SPEC.md §15](../PRODUCT-SPEC.md#15-open-design-decisions)
  leaves that open). No Rust code writes to or reads from it yet.
- This ADR does not adopt the full node/edge graph schema, the provider or
  persona architecture, or Ringmaster's own outward-facing MCP server. Those
  remain open per [docs/PRODUCT-SPEC.md § Relationship to accepted ADRs and Vision](../PRODUCT-SPEC.md#relationship-to-accepted-adrs-and-vision)
  and each needs its own bounded ADR before implementation.

## Scope

**In scope:** renaming the existing event-sourced aggregate and its schema
from Commitment to Obligation; enabling pgvector as a required extension;
adding a minimal, dimension-unconstrained `embeddings` table.

**Out of scope:** the full 15-node graph/edge schema, an embedding model or
vector dimension choice, any Rust code that produces or queries embeddings,
provider/persona architecture, and Ringmaster's own outward-facing MCP
server.

## Options considered

- **Rename to Obligation, keep event-sourcing (chosen):** matches the spec's
  primary entity without discarding a stronger, already-tested data
  integrity guarantee than the spec's literal table description implies.
- **Supersede ADR-0005 with a mutable `obligations` row plus a separate audit
  log, as literally described in §9.2:** matches the spec's wording most
  literally, but weakens the "was this completed, is it still relevant"
  guarantee the spec itself depends on, and discards already-passing tests
  for no material benefit.
- **Leave ADR-0005 as Commitment and treat Obligation as spec-only
  terminology:** avoids a rename, but leaves the codebase and the accepted
  product spec permanently disagreeing on the name of the primary entity,
  which is exactly the kind of drift ADR coverage exists to prevent.

## Consequences

- **Positive:** the codebase's primary entity name now matches the accepted
  product specification; pgvector is genuinely required rather than
  aspirational; the stronger event-sourcing guarantee is preserved.
- **Negative / trade-off:** every reference to "commitment" in code, migrations,
  and evidence needed updating in the same change to avoid a half-renamed
  state.
- **Risk:** a future embedding pipeline may need a fixed vector dimension;
  deferring that choice means the `embeddings` table will need a follow-up,
  ADR-governed migration once a model is chosen.

## Exit criteria and evidence

Evidence: [EV-0007](../evidence.d/0007-generalize-obligation-and-require-pgvector.md)

| Exit criterion | Evidence |
|---|---|
| The event-sourced aggregate and its schema are named Obligation, not Commitment | `obligation-events-table-exists`, `lib-wires-obligation-module`, `lib-no-longer-mentions-commitment` |
| pgvector is a required extension; migrations fail without it | `pgvector-extension-required` |
| A minimal, dimension-unconstrained embeddings table exists | `embeddings-table-exists` |
