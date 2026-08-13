# ADR-0010: Transcript ingestion — parsing, chunking, and immutable source fragments

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Continuation of accepted [docs/PRODUCT-SPEC.md](../PRODUCT-SPEC.md) Epic E3 under "keep going", 2026-08-14
- **Depends on:** [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md)
- **Tags:** architecture, data-model, ingestion

## Context

[docs/PRODUCT-SPEC.md § 6.1](../PRODUCT-SPEC.md#61-input-contract) and
[§ 6.2](../PRODUCT-SPEC.md#62-extraction-pipeline) (steps 1–2) describe the
first stage of transcript ingestion: capture source metadata and an
immutable raw transcript reference, then chunk by conversation structure and
speaker turns, "not arbitrary character count alone." Epic E3 in § 16 names
this as "upload/import contract, parsing, chunking, hashing and source
provenance." [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md)
already provides the `nodes` and `source_fragments` tables this needs, but
left `source_fragments`' mutability undecided — it discusses `nodes`/`edges`
mutability explicitly and does not mention `source_fragments`.

Source fragments are exact quotes: the spec's "Evidence before confidence"
principle and its extraction-object contract (§6.3, `quote`) depend on a
captured quote never silently changing. That is the same shape of problem
[ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md) and
[ADR-0008](0008-add-append-only-audit-events-table.md) already solved for
commitment and audit events, so this ADR closes the gap the same way rather
than inventing a new mechanism.

## Decision

- `source_fragments` becomes append-only at the database level: the
  database rejects `UPDATE` and `DELETE` on existing rows, the same
  enforcement pattern as `obligation_events` and `audit_events`. A
  correction belongs to a future Obligation/candidate event that references
  the original fragment; it must never edit the fragment's own text.
- A Rust `transcript` module provides `ingest_transcript(pool, metadata,
  raw_text)`, which:
  - creates one `nodes` row (`node_type = "meeting"`) carrying the meeting's
    title, date, organiser, and participants as attributes, plus a
    SHA-256 hash of the full raw text as its immutable reference;
  - parses `raw_text` into per-speaker turns, not fixed-size chunks;
  - inserts one `source_fragments` row per turn, each with its own SHA-256
    content hash, linked to the meeting node via `source_id`.
- The turn parser uses a minimal, explicitly provisional `Speaker: text`
  line convention. It exists to make chunking, hashing, and immutability
  real and testable now; it is not a decision about what a real meeting
  provider (Teams, Scout) actually exports. That remains Epic E10 (Providers)
  and will very likely replace or extend this parser.
- This ADR does not deduplicate meetings or fragments across repeated
  ingestion (already deferred by [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md)),
  does not extract candidates (Epic E4), does not generate embeddings
  (Epic E6), and does not add an HTTP API layer, authentication, or a web
  framework choice — ingestion is exercised as a Rust function and test
  today, not a network-facing service.

## Scope

**In scope:** the `source_fragments` immutability trigger; a Rust module
that parses a raw transcript into speaker turns and ingests a meeting node
plus hashed, immutable source fragments.

**Out of scope:** real provider transcript formats, deduplication/entity
resolution, extraction of obligations/candidates, embeddings, and any
HTTP/API surface.

## Options considered

- **Placeholder line-based parser plus immutable fragments (chosen):**
  makes the provenance/hash/immutability mechanics real and tested without
  waiting on an undecided provider integration that doesn't change how
  fragments must be stored.
- **Wait for a real provider format before building any parser:** avoids a
  throwaway format, but blocks provenance and immutability work that does
  not actually depend on which provider is chosen.
- **Leave `source_fragments` mutable, like `nodes`/`edges`:** simpler, but
  would let a quote be silently edited after capture, undermining the
  "Evidence before confidence" principle the whole product depends on.

## Consequences

- **Positive:** transcript ingestion produces real, hash-verifiable,
  tamper-evident source fragments today; the gap ADR-0009 left open is
  closed with the same, already-proven mechanism.
- **Negative / trade-off:** the `Speaker: text` parser is a known throwaway;
  real provider integration will need its own, likely more complex, parsing
  logic.
- **Risk:** ingesting the same transcript twice creates two meeting nodes
  and duplicate fragments today, since deduplication is explicitly out of
  scope. This is a known, visible gap, not a silent one.

## Exit criteria and evidence

Evidence: [EV-0010](../evidence.d/0010-transcript-ingestion-parsing-chunking-provenance.md)

| Exit criterion | Evidence |
|---|---|
| `source_fragments` rejects mutation or deletion of an existing row | `source-fragments-are-immutable` |
| A Rust function ingests a transcript into a meeting node plus hashed source fragments | `ingest-transcript-function-exists` |
| Parsing splits by speaker turn, not arbitrary character count | `parse-transcript-function-exists` |
