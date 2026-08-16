# ADR-0040: Dated source ingestion — `occurred_at` required, exposed over API, CLI, and MCP

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Continuation of this session's established build pattern ("pick up work"), following monk-eee's own two direct corrections to this ADR's content, 2026-08-17
- **Depends on:** [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md), [ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md), [ADR-0034](0034-http-meeting-transcript-ingestion.md)
- **Amends:** [ADR-0034](0034-http-meeting-transcript-ingestion.md)'s `IngestMeetingRequest` contract (`date: Option<String>` becomes `occurred_at`, required and validated) and [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md)'s `nodes` schema (a new, nullable `occurred_at TIMESTAMPTZ` column for existing rows, but enforced non-null at the application layer for every node this ADR's ingestion path creates). Extends, rather than amends, [ADR-0003](0003-ringmaster-ingests-mindleak-as-an-mcp-source.md)'s MCP-first architecture: that record governs Ringmaster as an MCP *client* of MindLeak; this is the first decision where Ringmaster is also an MCP *server* in its own right.
- **Tags:** architecture, data-model, api, cli, mcp, ingestion

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

A first draft of this ADR proposed a single HTTP route as the only
ingestion surface. monk-eee corrected that twice: *"i wanted an mcp for
ingestion because then i could point you at my existing huge corpus of
data and you could load it"*, then, after a revision leaned MCP-only,
*"api is fine - but we need api, cli and mcp - dont pick just one."* The
actual requirement is three thin surfaces over one shared decision, not a
choice between them: an HTTP route for programmatic/web callers, a CLI
for scripting monk-eee's own corpus through directly (no server required),
and an MCP tool so an agent already in a conversation — this one included
— can ingest a pointed-at corpus without a human relaying each call
through curl or a script.

Ringmaster already has the graph, the immutable evidence fragments
([ADR-0010](0010-transcript-ingestion-parsing-chunking-provenance.md)),
and temporally-valid edges
([ADR-0032](0032-temporal-edge-validity-supersede-on-write.md)) this needs
— but today's one ingestion path
([ADR-0034](0034-http-meeting-transcript-ingestion.md)) only accepts a
*meeting* transcript over HTTP, and even there its own date is an
optional, unvalidated free-text string buried in `nodes.attributes` JSONB,
not a real column. [VISION.md](../VISION.md) already names Outlook, Teams,
Calendar, SharePoint, and OneNote as intended future sources, and
separately flags that live access to some of those carries a real,
undecided access-control question for sensitive content — this ADR does
not resolve that; it makes ingesting already-collected text (however it
arrives) correct and reachable three ways first.

## Decision

### One shared core, three thin adapters

A single Rust function, `ingest_source` (in `transcript.rs`, alongside and
reusing `parse_transcript`), is the only place ingestion logic lives:

- Inputs: `source_type` (free text — `"meeting"`, `"email"`,
  `"teams_message"`, `"note"` are the suggested, not enforced, vocabulary,
  matching `node_type`'s own established free-text convention), a required
  `title`, a required **`occurred_at: DateTime<Utc>`** (not `Option` — the
  one new hard requirement this whole ADR exists for), optional
  `participants`, and the required raw `text`.
- Behavior, extending [ADR-0034](0034-http-meeting-transcript-ingestion.md)'s
  exact atomic pattern (one node + its ordered, hashed, immutable source
  fragments, one transaction, never triggers extraction/embedding):
  `source_type: "meeting"` keeps today's per-speaker-turn splitting via
  `parse_transcript`; any other `source_type` uses a new, simpler
  splitter — one fragment per blank-line-separated paragraph, no speaker
  field (emails and Teams messages don't carry a transcript's turn-taking
  shape). A single-paragraph submission becomes exactly one fragment.
- The created node's new `occurred_at` column is set from the argument;
  its `node_type` is set from `source_type` unchanged.
- **Every surface below calls this one function.** None of the three
  re-implements validation, splitting, or the transaction.

### Schema

`nodes` gains a nullable `occurred_at TIMESTAMPTZ` column (migration,
existing rows unaffected — `NULL` for anything ingested before this ADR).
This is the real-world time the event happened, distinct from
`created_at` (when Ringmaster stored it).

### API

`POST /api/sources/ingest` — a thin axum handler validating the JSON body
then calling `ingest_source`; `400` if `occurred_at` is missing or
unparseable, matching [ADR-0034](0034-http-meeting-transcript-ingestion.md)'s
existing blank-field-check posture. `POST /api/meetings/ingest` is
amended, not replaced: `date: Option<String>` becomes a required
`occurred_at`, validated the same way. Every other field and the exact
response shape (`meeting_id`, `fragment_ids`) stays unchanged. This is the
one behavior-breaking piece of this ADR, named explicitly: an existing
caller that omitted `date` must now supply `occurred_at`.

### CLI

A second binary in the same `ringmaster-backend` package
(`backend/src/bin/ringmaster-ingest.rs`, built via Cargo's standard
`src/bin/` convention — no new package or workspace), linking the same
library crate `main.rs` already uses. Connects directly to `DATABASE_URL`
(no running HTTP server required) and calls `ingest_source` directly —
the natural way to script monk-eee's existing "adoa" markdown corpus
through, file by file, without anything else running. Flags:
`--source-type`, `--title`, `--occurred-at`, `--participants` (repeatable),
and the text via `--text-file <path>` or stdin. Prints the created
`node_id`/`fragment_ids` as JSON on success; a non-zero exit code and a
plain-text error on failure (missing/unparseable `occurred_at` included) —
scriptable, not interactive.

### MCP

The same binary gains an `mcp-serve` subcommand: an MCP server over
**stdio** (matching how monk-eee's other local MCP servers — MindLeak,
Lodestar — are already launched in this environment, so configuring an
MCP client to add Ringmaster is the same shape of change, not a new
pattern), built with the official Rust SDK,
[`rmcp`](https://crates.io/crates/rmcp) (a new dependency, added only to
this binary's needs — justified because MCP is the explicitly-requested
surface, not decoration). It exposes exactly one tool, `ingest_source`,
with the same parameters as the CLI/API, calling the identical
`ingest_source` function. No resources, prompts, or sampling capability —
one tool, nothing this ADR wasn't asked for. This is the first ADR where
Ringmaster is an MCP *server*; [ADR-0003](0003-ringmaster-ingests-mindleak-as-an-mcp-source.md)'s
MCP-first posture already anticipated Ringmaster on both sides of that
boundary, just not built until now.

## Scope

**In scope:** the `occurred_at` column; the shared `ingest_source`
function and its paragraph-based splitter for non-meeting sources;
`POST /api/sources/ingest`; amending `POST /api/meetings/ingest` to
require `occurred_at`; the `ringmaster-ingest` CLI binary; that binary's
`mcp-serve` stdio MCP server exposing the one `ingest_source` tool.

**Out of scope, named honestly:**

- **Live Outlook/Teams/Calendar/SharePoint connectors.** All three
  surfaces accept already-collected text (pasted, scripted, piped, or
  passed as a tool argument); none of them add a live mailbox/Teams API
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
- **A bulk/batch-directory ingestion command** (e.g., "ingest every file
  in this folder"). The CLI ingests one source per invocation; looping
  over monk-eee's corpus is a shell/script concern, not this ADR's.
  Likewise the MCP tool ingests one source per call — an agent pointed at
  a corpus calls it once per file/message, it does not walk a directory
  itself.
- **MCP resources, prompts, or sampling.** Only the one `ingest_source`
  tool is exposed; reading Ringmaster's own data back out over MCP is the
  "clever ways of getting out" monk-eee already deferred.
- **Retroactively backfilling `occurred_at` on already-ingested nodes** —
  they keep `NULL`, rendered/queried as unknown, never guessed.

## Options considered

- **One shared function, three thin adapters — API, CLI, stdio MCP
  (chosen):** directly answers monk-eee's explicit correction ("dont pick
  just one"); reuses [ADR-0034](0034-http-meeting-transcript-ingestion.md)'s
  proven atomicity once, not three times; each surface stays genuinely
  thin because none contains its own copy of the validation/splitting
  logic.
- **MCP only:** rejected — explicitly overridden by monk-eee ("api is
  fine - but we need api, cli and mcp").
- **HTTP API only, no CLI or MCP:** rejected for the same reason — this
  was this ADR's own first draft, and monk-eee corrected it directly.
- **A separate Cargo workspace/package for the CLI+MCP binary, instead of
  a second binary in the existing package:** rejected as unnecessary
  process/dependency-graph complexity; Cargo's own `src/bin/` convention
  already gives a second binary sharing the existing library crate
  without a new package.
- **Mount the MCP server as Streamable HTTP on the existing axum router
  instead of stdio:** would avoid a second binary, but every other local
  MCP server monk-eee already runs (MindLeak, Lodestar) is stdio-launched;
  matching that shape is less surprising to configure than introducing
  the one HTTP-mounted exception.

## Consequences

- **Positive:** directly answers the stated priority — dated, structured,
  graph-native storage for any collected text, reachable the three ways
  monk-eee asked for — without tripling the ingestion logic itself.
- **Positive:** an agent (this one, or a future session) can be pointed
  directly at monk-eee's "adoa" corpus and call `ingest_source` file by
  file inside one conversation, the concrete capability originally asked
  for.
- **Negative / trade-off:** any existing caller of
  `POST /api/meetings/ingest` that relied on `date` being optional breaks
  until it supplies `occurred_at` — an intentional, named break, not an
  oversight. A new dependency (`rmcp`) is added, scoped to the CLI/MCP
  binary only.
- **Risk:** low. One additive nullable column; one shared, already-tested-
  shape ingestion function; three call sites that validate and delegate
  rather than reimplement.

## Exit criteria and evidence

Evidence: [EV-0040](../evidence.d/0040-dated-source-ingestion.md)

| Exit criterion | Evidence |
|---|---|
| `nodes` has a nullable `occurred_at` column | `nodes-have-occurred-at-column` |
| The shared `ingest_source` function creates a node + ordered fragments, splitting non-meeting text by paragraph, and rejects a missing `occurred_at` | `ingest-source-function-requires-occurred-at` |
| `POST /api/sources/ingest` calls the shared function, rejecting a missing/blank `occurred_at` with `400` | `sources-ingest-route-exists` |
| `POST /api/meetings/ingest` now requires `occurred_at` and rejects its absence with `400`, all other behavior unchanged | `meeting-ingest-requires-occurred-at` |
| The `ringmaster-ingest` CLI binary ingests a source by calling the same shared function, with no HTTP server required | `cli-binary-ingests-via-shared-function` |
| The `mcp-serve` subcommand exposes exactly one MCP tool, `ingest_source`, over stdio, calling the same shared function | `mcp-tool-exposes-ingest-source` |
| None of the three surfaces triggers extraction or embedding implicitly | `ingestion-never-triggers-extraction-or-embedding` |

