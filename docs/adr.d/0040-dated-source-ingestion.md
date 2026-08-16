# ADR-0040: Dated source ingestion — `occurred_at` becomes a required, structured field across every ingested source

- **Status:** Proposed
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Pending — awaiting monk-eee's decision
- **Depends on:** [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md), [ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md), [ADR-0034](0034-http-meeting-transcript-ingestion.md)
- **Amends:** [ADR-0034](0034-http-meeting-transcript-ingestion.md)'s `IngestMeetingRequest` contract (`date: Option<String>` becomes `occurred_at`, required and validated) and [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md)'s `nodes` schema (a new, nullable `occurred_at TIMESTAMPTZ` column for existing rows, but enforced non-null at the application layer for every node this ADR's ingestion path creates)
- **Tags:** architecture, data-model, api, ingestion

## Context

monk-eee's own working process today: collect emails, transcripts, verbal
instructions, and Teams messages into a separate repository ("adoa") as
plain markdown, so a later "what's next" question has every decision and
message available. Two concrete problems, in monk-eee's own words: *"its
now too much and it fills the context window and most importantly its not
date driven... Microsoft is date driven... i need to be able to collate
this information — people, decisions, historical data on a graph with
dates so i can build an ever growing corpus."* Priority stated directly:
*"getting data in and storing it is pretty important, we can work on
clever ways of getting out [later]."*

Ringmaster already has the graph, the immutable evidence fragments
([ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md)),
and temporally-valid edges
([ADR-0032](0032-temporal-edge-validity-supersede-on-write.md)) this needs
— but today's one ingestion path
([ADR-0034](0034-http-meeting-transcript-ingestion.md)) only accepts a
*meeting* transcript, and even there its own date is an optional,
unvalidated free-text string buried in `nodes.attributes` JSONB, not a
real, structured, queryable column. That is the literal, narrow cause of
"not date driven": nothing about the schema *requires* or *indexes* when
an ingested thing actually happened, and nothing outside meetings can be
ingested at all.

[VISION.md](../VISION.md) already names Outlook, Teams, Calendar,
SharePoint, and OneNote as intended future sources, and separately flags
that live access to some of those carries a real, undecided access-control
question for sensitive content. This ADR does not resolve that; it makes
the *storage* half of the problem (any dated text, structured, on the
graph) correct first, independent of which live connector eventually
supplies the text.

## Decision

- **Schema:** `nodes` gains a nullable `occurred_at TIMESTAMPTZ` column
  (migration, existing rows unaffected — `NULL` for anything ingested
  before this ADR). This is the real-world time the underlying event
  happened, distinct from `created_at` (when Ringmaster stored it).
- **Generalized ingestion route**, `POST /api/sources/ingest`, extending
  [ADR-0034](0034-http-meeting-transcript-ingestion.md)'s exact atomic
  pattern (one Meeting-shaped node + its ordered, hashed, immutable source
  fragments, one transaction, never triggers extraction/embedding):
  - Accepts `source_type` (free text, matching `node_type`'s own
    established convention — `"meeting"`, `"email"`, `"teams_message"`,
    `"note"` are the suggested, not enforced, vocabulary), a required
    `title`, a required **`occurred_at`** (RFC 3339; `400` if missing or
    unparseable — the one new hard requirement this ADR exists for),
    optional `participants`, and the required raw `text`.
  - `source_type: "meeting"` keeps today's exact behavior: `text` is split
    into per-speaker-turn fragments via the existing
    `transcript::parse_transcript` (`"Speaker: text"` lines).
  - Any other `source_type` uses a new, simpler splitter: one fragment per
    blank-line-separated paragraph (no speaker field — emails and Teams
    messages don't carry the meeting transcript's turn-taking shape).
    A single-paragraph submission (a short note, a one-line instruction)
    becomes exactly one fragment.
  - The created node's `occurred_at` column is set from the request; its
    `node_type` is set from `source_type` unchanged (no new type
    vocabulary is enforced at the database level, consistent with
    [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md)'s own
    free-text `node_type`).
- **`POST /api/meetings/ingest` is amended, not replaced**: `date:
  Option<String>` becomes a required `occurred_at`, validated the same
  way as the new route (`400` if missing/unparseable). Every other field
  and its exact response shape (`meeting_id`, `fragment_ids`) stays
  unchanged. This is the one behavior-breaking piece of this ADR, named
  explicitly: an existing caller that omitted `date` must now supply
  `occurred_at`.
- **No new node type enforcement, no owner/participant-to-Person-node
  linking, no candidate extraction change.** `participants` is stored
  verbatim in `attributes`, exactly like today's meeting `participants` —
  resolving a name string to a real Person node, or extracting an `owner`/
  `counterparty` per [PRODUCT-SPEC.md §6.3](../PRODUCT-SPEC.md#63-extraction-object-contract)'s
  own long-stated (never implemented) contract, is real, separate,
  larger work this ADR does not decide.

## Scope

**In scope:** the `occurred_at` column; `POST /api/sources/ingest` for
non-meeting dated text; amending `POST /api/meetings/ingest` to require
`occurred_at`; the paragraph-based fragment splitter for non-meeting
sources.

**Out of scope, named honestly:**

- **Live Outlook/Teams/Calendar/SharePoint connectors.** This ADR accepts
  already-collected text (pasted, scripted, or piped in) exactly the way
  meeting ingestion already does; it does not add a live MCP/API
  integration. [VISION.md](../VISION.md#open-questions-for-future-adrs)'s
  own named access-control question for that class of data is untouched
  and still blocks it.
- **Owner/counterparty/Person-node linking at ingestion or extraction
  time.** `participants` stays a plain string list, matching today's
  meeting behavior exactly — no dedup, no resolution, no new edge.
- **Surfacing `occurred_at` in the Timeline/Time Horizon views.** Those
  views rank and bucket by *Obligation* due dates today
  ([ADR-0029](0029-time-horizon-view.md)); teaching them to also consider
  a linked source's `occurred_at` is a real, separate follow-up.
- **A CLI or batch-import tool.** The HTTP route is the only surface this
  ADR adds; how monk-eee's existing "adoa" corpus gets fed through it
  (one-by-one, scripted, or a later bulk endpoint) is left open.
- **Retroactively backfilling `occurred_at` on already-ingested nodes** —
  they keep `NULL`, rendered/queried as unknown, never guessed.

## Options considered

- **Generalize the existing atomic ingestion route with a required
  `occurred_at` (chosen):** reuses everything
  [ADR-0034](0034-http-meeting-transcript-ingestion.md) already proved
  (atomicity, immutable fragments, never-implicit extraction), fixes the
  actual named complaint (date is optional/unstructured) at its root
  rather than adding a parallel, inconsistent path.
- **Leave meeting ingestion alone; add only a new route for other
  sources:** rejected — would leave meetings themselves still
  "not date driven," the exact problem stated, and would create two
  different date-handling behaviors for no real reason.
- **Store `occurred_at` only in `attributes` JSONB, not a real column:**
  rejected — a JSONB string can't be indexed, range-queried, or validated
  as a real timestamp by the database; this is the status quo's actual
  defect, not a fix for it.
- **Design and build live Outlook/Teams ingestion now:** rejected as
  premature — VISION.md already names a real, unresolved access-control
  decision for that data class; building a live connector ahead of that
  decision would be the kind of ungoverned mutation
  [ADR-0001](0001-require-governing-adr-coverage-before-implementation.md)
  itself exists to prevent.

## Consequences

- **Positive:** directly answers the stated priority — dated, structured,
  graph-native storage for any collected text — without inventing new
  ingestion machinery; reuses [ADR-0034](0034-http-meeting-transcript-ingestion.md)'s
  proven atomicity and [ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md)'s
  immutable-fragment guarantee for every source type, not just meetings.
- **Negative / trade-off:** any existing caller of
  `POST /api/meetings/ingest` that relied on `date` being optional breaks
  until it supplies `occurred_at` — an intentional, named break, not an
  oversight.
- **Risk:** low. One additive nullable column; one new route reusing an
  already-proven transactional pattern; one narrow, explicitly-named
  contract change to an existing route.

## Exit criteria and evidence

Evidence: [EV-0040](../evidence.d/0040-dated-source-ingestion.md)

| Exit criterion | Evidence |
|---|---|
| `nodes` has a nullable `occurred_at` column | `nodes-have-occurred-at-column` |
| `POST /api/sources/ingest` creates a node + ordered fragments for a non-meeting source, rejecting a missing/blank `occurred_at` with `400` | `sources-ingest-requires-occurred-at` |
| A non-meeting source is split into one fragment per paragraph, not per speaker turn | `non-meeting-source-splits-by-paragraph` |
| `POST /api/meetings/ingest` now requires `occurred_at` and rejects its absence with `400`, all other behavior unchanged | `meeting-ingest-requires-occurred-at` |
| Neither route triggers extraction or embedding implicitly | `ingestion-never-triggers-extraction-or-embedding` |
