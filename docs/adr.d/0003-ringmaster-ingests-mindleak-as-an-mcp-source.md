# ADR-0003: Ringmaster ingests MindLeak as an MCP source, not a shared graph

- **Status:** Accepted
- **Date:** 2026-08-13
- **Decider:** monk-eee
- **Approval:** Explicitly accepted by monk-eee on 2026-08-13
- **Depends on:** [ADR-0001](0001-require-governing-adr-coverage-before-implementation.md)
- **Tags:** integration, mcp, mindleak, architecture

## Context

[docs/VISION.md](../VISION.md) describes MindLeak and Ringmaster as two
temporal memory systems: MindLeak models repositories, files, decisions, and
agents; Ringmaster models people, commitments, outcomes, and organizations.
Both already listen for repository/agent context, so it is easy for a future
change to quietly merge them into one schema or have Ringmaster query
MindLeak's SQLite files directly. That would blur two deliberately different
domains and couple Ringmaster's commitment graph to MindLeak's internal
storage format and lifecycle.

Ringmaster's technology direction is MCP-first: it consumes source systems
rather than replacing them, and MindLeak itself is listed in
[docs/VISION.md](../VISION.md#technology-direction) as one of Ringmaster's
sources alongside Azure DevOps, Outlook, Teams, and GitHub.

## Decision

Ringmaster must treat MindLeak as one ingested MCP source among others, using
the same ingestion shape as every other source.

- Ringmaster must read MindLeak only through MindLeak's own MCP tools (for
  example `recall`, graph/impact queries), never by opening MindLeak's SQLite
  files directly.
- Ringmaster's ingestion layer must translate MindLeak's responses into
  Ringmaster's own commitment-graph facts and events at ingestion time. It
  must not adopt MindLeak's node/edge schema as Ringmaster's internal
  representation.
- Ringmaster's commitment graph remains the sole authority for commitment
  state. MindLeak-derived facts are inputs to that graph, not a second store
  of record.
- Ringmaster must not implement live query federation across the two graphs
  (querying both stores per request and merging results at read time).
- This decision governs the integration boundary only. The specific MCP
  calls, the Rust ingestion adapter, and authentication to a local MindLeak
  server are separate, later implementation work.

## Scope

**In scope:** the architectural relationship between Ringmaster's commitment
graph and MindLeak; the constraint that MindLeak is read-only input.

**Out of scope:** the Rust connector implementation, the specific MindLeak MCP
tool calls used, ingestion scheduling/polling, and the event-sourced
commitment schema itself (see the forthcoming schema ADR).

## Options considered

- **Ingest as one MCP source, translated at the boundary (chosen):** keeps a
  clean ownership boundary, matches the already-declared MCP-first
  architecture, and prevents MindLeak's decay-weighted graph semantics from
  leaking into commitment semantics.
- **Federate live queries across both graphs:** would let a single query span
  code context and commitments, but couples Ringmaster's read path to
  MindLeak's availability and query language, and blurs which store is
  authoritative for what.
- **Shared schema / single graph:** most "unified" on paper, but merges two
  domains with different lifetimes, decay models, and audiences; contradicts
  the explicit domain split already stated in the vision.
- **No integration for now:** simplest, but drops a source the vision already
  names, and defers a decision an agent could otherwise make silently later.

## Consequences

- **Positive:** MindLeak and Ringmaster stay independently understandable;
  Ringmaster's commitment graph has one authoritative internal shape
  regardless of how many sources feed it.
- **Negative / trade-off:** Ringmaster must maintain its own ingestion adapter
  and re-derive facts MindLeak already computed once, rather than reusing its
  storage directly.
- **Risk:** an overly broad or frequent ingestion query could duplicate facts
  already known from other sources (for example, a decision already recorded
  as a commitment). Mitigated by scoping ingestion queries narrowly and
  de-duplicating at the commitment-graph layer when the schema ADR defines
  that layer.

## Exit criteria and evidence

Evidence: [EV-0003](../evidence.d/0003-ringmaster-ingests-mindleak-as-an-mcp-source.md)

| Exit criterion | Evidence |
|---|---|
| Ringmaster's source code has no direct dependency on MindLeak's SQLite storage format | `no-direct-mindleak-storage-access` |
| The vision document names this boundary as the open question this ADR resolves | `vision-names-boundary-question` |
