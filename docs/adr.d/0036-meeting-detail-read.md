# ADR-0036: Meeting detail read — one meeting with its ordered transcript fragments

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Continuation of this session's established build pattern ("commit and push and keep going"), 2026-08-14
- **Depends on:** [ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md), [ADR-0034](0034-http-meeting-transcript-ingestion.md)
- **Tags:** architecture, api, data-model, meeting, transcript

## Context

[ADR-0034](0034-http-meeting-transcript-ingestion.md) added
`POST /api/meetings/ingest`, which creates a Meeting node and its ordered
source fragments in one transaction and returns their ids in that one
response. There is still no way to read a meeting back afterward: no route
fetches the meeting's title/participants together with its transcript
fragments. [MEETING-REVIEW-DESIGN.md](../MEETING-REVIEW-DESIGN.md) names
this as the second candidate implementation slice, directly after ingestion,
and the meeting review screen it describes cannot be built at all without a
meeting-scoped read.

A genuine ordering problem surfaced while scoping this: `source_fragments`
has no explicit sequence column, only `created_at`. Because
`ingest_transcript` writes every fragment inside one transaction
([ADR-0034](0034-http-meeting-transcript-ingestion.md)), and Postgres's
`now()` returns the transaction's start time rather than a per-statement
time, every fragment from the same ingestion call can share the exact same
`created_at` value. Sorting by `created_at` alone cannot reliably reproduce
transcript turn order once ties are possible. This must be fixed at the
schema level, not papered over in a query.

## Decision

- Migration `0012_source_fragment_sequence.sql` adds a nullable
  `source_fragments.sequence INTEGER` column. Existing rows keep `NULL`,
  the same additive-column posture already used for `hard_due_at` /
  `soft_due_at` ([ADR-0020](0020-obligation-due-date-fields.md)) and
  `valid_from` / `valid_to`
  ([ADR-0032](0032-temporal-edge-validity-supersede-on-write.md)).
- `transcript::ingest_transcript` sets `sequence` to the fragment's 0-based
  position within that ingestion call. This is the only writer of this
  column; nothing else sets or edits it, consistent with
  `source_fragments`' append-only guarantee
  ([ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md)) —
  `sequence` is written once at insert, never updated afterward.
- A new `graph::list_source_fragments_by_meeting` reads a meeting's
  fragments ordered by `sequence ASC NULLS LAST, created_at ASC, id ASC` —
  a fully deterministic order that favors the new column when present and
  falls back safely for any fragment created before this ADR.
- `GET /api/meetings/:id` reads the node via the existing
  `graph::get_node` ([ADR-0025](0025-node-edge-write-api-and-traversal.md)),
  returning `404` when the id doesn't exist or when its `node_type` is not
  `"meeting"` — this route's contract is specifically a meeting, not any
  node. On success it returns the meeting's `id`, `canonical_text`,
  `attributes` (title/date/organiser/participants/raw transcript hash, per
  [ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md)),
  and an ordered `fragments` array (`id`, `text`, `speaker`, `sequence`,
  `created_at`) — read-only, no writes.

## Scope

**In scope:** the `sequence` column and its migration; `ingest_transcript`
writing it; `list_source_fragments_by_meeting`; `GET /api/meetings/:id`.

**Out of scope, named honestly (deferred to later, already-sequenced
slices of [MEETING-REVIEW-DESIGN.md](../MEETING-REVIEW-DESIGN.md)):**
listing or counting candidates already extracted per fragment (slice 3);
any frontend meeting-review screen or transcript/proposal synchronization
(slice 4 onward); backfilling `sequence` for fragments ingested before this
ADR (they keep `NULL` and sort after sequenced ones, exactly like every
other additive-nullable-column precedent in this repository); pagination
for very large transcripts; any change to `POST /api/meetings/ingest`'s own
request or response shape.

## Options considered

- **Add a `sequence` column plus a dedicated read route (chosen):** the
  smallest change that makes ordering genuinely reliable and satisfies the
  design document's next named slice; reuses `graph::get_node` rather than
  duplicating node-fetch logic.
- **Order by `created_at` alone, accepting ties:** rejected — this is not a
  cosmetic risk. Every fragment from one ingestion call can share an
  identical transaction-start timestamp, so this would silently scramble
  transcript order the very first time it matters.
- **Order by `id` (UUID):** rejected — UUIDs carry no ordering relationship
  to insertion time; this would be arbitrary, not deterministic-by-design.
- **Fold this into `GET /api/nodes/:id` instead of a dedicated meetings
  route:** rejected — that route's contract is a generic one-hop
  neighborhood ([ADR-0025](0025-node-edge-write-api-and-traversal.md)), not
  transcript fragments, and overloading it would blur two different reads
  behind one response shape, the same reasoning
  [ADR-0029](0029-time-horizon-view.md) already used to justify a dedicated
  route over a generic one.

## Consequences

- **Positive:** closes the "cannot read a meeting back" gap
  [ADR-0034](0034-http-meeting-transcript-ingestion.md) left open, and
  fixes a real, previously-latent ordering bug before any caller could hit
  it in practice.
- **Positive:** unblocks the next slice of
  [MEETING-REVIEW-DESIGN.md](../MEETING-REVIEW-DESIGN.md) (candidate
  listing per fragment) without redoing this one.
- **Negative / trade-off:** fragments ingested before this ADR have no
  `sequence` and fall back to `created_at`/`id` ordering, which is only as
  reliable as those columns already were — an honestly bounded, not
  silently hidden, limitation.
- **Risk:** low. One additive nullable column, one new read-only route, no
  change to any existing route's request or response shape.

## Exit criteria and evidence

Evidence: [EV-0036](../evidence.d/0036-meeting-detail-read.md)

| Exit criterion | Evidence |
|---|---|
| `ingest_transcript` writes a 0-based `sequence` per fragment | `ingest-transcript-writes-sequence` |
| Fragments are read back ordered by sequence, not raw `created_at` | `list-source-fragments-orders-by-sequence` |
| `GET /api/meetings/:id` returns the meeting plus its ordered fragments | `meeting-detail-route-returns-ordered-fragments` |
| `GET /api/meetings/:id` returns 404 for an unknown id or a non-meeting node | `meeting-detail-route-404s-for-non-meeting` |
