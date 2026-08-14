# ADR-0034: Expose atomic meeting-transcript ingestion over HTTP

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Implemented following this session's established delegation pattern for well-scoped, low-risk proposals ("keep going"), 2026-08-14
- **Depends on:** [ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md), [ADR-0012](0012-minimal-http-api-and-node-web-front-end.md), [ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md)
- **Amends:** [ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md)'s explicit exclusion of an HTTP/API surface and its current non-transactional ingestion boundary
- **Tags:** architecture, api, ingestion, meeting, transcript

## Context

[ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md) built
`transcript::ingest_transcript`: it creates a Meeting node, splits the raw
transcript into speaker turns, and stores immutable source fragments with
content hashes. The running product cannot invoke that function. Only Rust
tests can ingest a complete meeting.

[ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md)
subsequently exposed extraction for one already-existing source fragment, but
there is still no supported way for an agent, CLI, web client, or script to
create the Meeting and fragments that route requires. A caller must currently
reproduce internal storage details through unrelated low-level node and
fragment operations; source fragments do not even have a public write route.

[Relationship Memory and Progressive Graph Design](../RELATIONSHIP-GRAPH-DESIGN.md)
defines a meeting as one ingestion unit: an agent submits meeting metadata and
the transcript, receives the resulting evidence objects, and can then trigger
the separately governed extraction flow. The HTTP API is the smallest shared
boundary that both a future Ringmaster CLI and MCP server can wrap without
duplicating ingestion logic.

The existing function also performs multiple writes without an explicit
transaction. If a fragment insert fails after the Meeting node is created,
partial meeting memory can remain. Exposing that behavior over a network
boundary would make retries and failures harder to reason about, so atomicity
belongs in the same decision as public reachability.

## Decision

- Add `POST /api/meetings/ingest` to the existing Axum router. Its JSON body
  contains:

  ```json
  {
    "title": "Weekly 1:1",
    "date": "2026-08-14",
    "organiser": "Lyndon",
    "participants": ["Lyndon", "Roopa Venkat"],
    "transcript": "Roopa: Please bring me a transition plan."
  }
  ```

- `title` and `transcript` are required non-blank strings. `date` and
  `organiser` are nullable strings. `participants` defaults to an empty array
  when omitted. A structurally invalid request or blank required field returns
  `400` and performs no writes.
- The route delegates parsing, hashing, Meeting-node construction, and source-
  fragment construction to the existing `transcript` module. The route does
  not duplicate the provisional `Speaker: text` parser or issue its own SQL.
- Ingestion becomes atomic: creation of the Meeting node and every source
  fragment occurs inside one database transaction. Success commits all rows;
  any storage error rolls the transaction back, including the Meeting node.
  This changes the internal transaction boundary but not ADR-0010's data
  shape, parser, hashes, or append-only fragment guarantee.
- A successful request returns `201` with the created Meeting id and ordered
  source-fragment ids:

  ```json
  {
    "meeting_id": "...",
    "fragment_ids": ["...", "..."]
  }
  ```

  Fragment order matches transcript turn order so a caller can deliberately
  invoke [ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md)'s
  existing `POST /api/source-fragments/:id/extract` route for each fragment.
- Ingestion never invokes a language model, candidate extraction, embedding,
  or graph enrichment automatically. A missing or unavailable model cannot
  prevent the Meeting and its evidence from being stored. This preserves
  ADR-0011/ADR-0013's explicit, non-blocking extraction posture.
- Repeating the same request creates another Meeting and another set of
  fragments, matching ADR-0010's accepted lack of deduplication. The route
  does not claim retry idempotency. A provider identity/idempotency contract
  requires a separate decision with a real external identifier to key on;
  transcript text hash alone is not adopted as Meeting identity here.
- Storage failures return `500` without exposing SQL, credentials, or internal
  connection details in the response. Existing server-side diagnostics remain
  available through the application's normal logging path.
- The route is covered by backend tests for successful ordered ingestion,
  validation with zero writes, transactional rollback, and the guarantee that
  no extraction candidate is created implicitly.

## Scope

**In scope:** one JSON HTTP ingestion route; request validation; delegation to
the existing transcript parser; one transaction spanning Meeting and fragment
writes; a `201` response with ordered identifiers; focused API and transaction
tests.

**Out of scope:** a CLI executable or command; a Ringmaster MCP server or tool;
multipart/file upload; a browser meeting-upload screen; provider-specific
Teams, Scout, or Graph transcript formats; storage of the full raw transcript
body; retrieval of a Meeting with all fragments; automatic extraction or
embedding; batch extraction; idempotency/deduplication; Person-node resolution
for organiser, speakers, or participants; edges connecting those people to the
Meeting; proposed-subgraph submission beyond the Meeting and its evidence.

## Options considered

- **One atomic HTTP route over the existing transcript module (chosen):**
  makes proven ingestion reachable through the service boundary already used
  by every current feature, gives future CLI/MCP clients one stable primitive,
  and removes partial-write behavior before external callers depend on it.
- **Build the Ringmaster MCP server first and expose ingestion only as an MCP
  tool:** matches the eventual integration direction, but no Ringmaster MCP
  server exists yet. It would combine protocol hosting, tool policy, and
  meeting ingestion in one decision while still needing an internal service
  boundary underneath.
- **Add a Rust CLI that calls `ingest_transcript` directly:** is quick for one
  machine, but bypasses the running service, requires direct database access,
  and would make a future MCP tool duplicate or wrap a CLI-specific boundary.
- **Automatically extract every fragment inside the ingestion request:**
  appears convenient but ties durable evidence capture to model availability
  and latency, directly reversing ADR-0013's deliberate explicit-trigger
  decision.
- **Use the transcript SHA-256 as an idempotency key:** prevents exact-content
  duplicates but can incorrectly merge distinct meetings with identical
  templated transcripts and cannot distinguish corrected provider exports.
  Defer until a provider or caller supplies a stable external identity.
- **Keep the existing non-transactional writes:** requires no internal
  refactor, but exposes partial meeting state after a mid-ingestion failure;
  rejected because one network request should have one durable outcome.

## Consequences

- **Positive:** an agent or script can finally load one real meeting through a
  supported product boundary, then use the existing fragment extraction route.
- **Positive:** evidence capture remains available when the language model is
  absent or unhealthy.
- **Positive:** callers receive ordered fragment ids without knowing the
  source-fragment schema or writing directly to Postgres.
- **Positive:** failed ingestion cannot leave a Meeting node with only some of
  its evidence.
- **Negative / trade-off:** exact request retries create duplicates until a
  real external identity and idempotency policy are designed.
- **Negative / trade-off:** clients must still trigger extraction once per
  fragment; convenient batch orchestration remains future work.
- **Risk:** the provisional line parser can assign `unknown` speakers for
  provider formats it does not understand. The route does not present that
  parser as a Teams/Scout compatibility guarantee.
- **Risk:** large transcript bodies are accepted as JSON in this slice without
  a product-specific size limit. Request-size policy should be decided with
  the real provider/upload format rather than guessed here.

## Exit criteria and evidence

Evidence: [EV-0034](../evidence.d/0034-http-meeting-transcript-ingestion.md)

| Exit criterion | Evidence |
|---|---|
| `POST /api/meetings/ingest` accepts meeting metadata and transcript text and returns `201` with Meeting and ordered fragment ids | `meeting-ingest-route-creates-ordered-fragments` |
| Blank required fields return `400` and create no Meeting or fragments | `meeting-ingest-validates-before-write` |
| A failed fragment write rolls back the Meeting and all fragment writes | `meeting-ingest-is-atomic` |
| The route delegates to the transcript module rather than duplicating parsing or persistence logic | `meeting-ingest-reuses-transcript-module` |
| Successful ingestion creates no extraction candidate or embedding implicitly | `meeting-ingest-does-not-run-models` |
