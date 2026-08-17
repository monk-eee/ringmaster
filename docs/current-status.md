# Ringmaster — Current status (a real audit, not a decision log)

Written 2026-08-17 by actually clicking through the running app, querying the
live dev database, and reading the code behind what rendered — not by
summarizing ADR titles. [`ARCHITECTURE.md`](ARCHITECTURE.md) is the formal,
decision-level snapshot; this is "what's really there right now, warts
included."

## The one-paragraph version

The backend is genuinely solid: an event-sourced Postgres graph (people,
obligations, meetings, evidence fragments, edges), three real ingestion
surfaces (HTTP, CLI, MCP) sharing one function, working semantic search
infrastructure, and 46 ADRs with automated evidence checks, almost all
`PROVEN`. The frontend re-steer (Today/Timeline/People/Inbox as primary)
is real and live. But the dev database is currently **~99% test-fixture
noise** — thousands of fake "at risk" obligations from months of test runs
that were never cleaned up — which makes the flagship Today page currently
unusable as a demo of the actual product, and exposed two real frontend gaps
(below) that were invisible until the data volume got this large.

## What's actually real and working

- **Graph substrate** (nodes/edges/source_fragments, [ADR-0009](adr.d/0009-add-graph-nodes-edges-and-source-fragments.md)):
  real, immutable evidence fragments (DB trigger rejects UPDATE/DELETE),
  polymorphic edges with temporal validity (supersede-on-write,
  [ADR-0032](adr.d/0032-temporal-edge-validity-supersede-on-write.md)).
- **Obligations** are event-sourced: append-only `obligation_events` +
  a rebuilt-from-scratch `obligation_projection`, never patched in place.
  This is the one piece of architecture I'd call unambiguously good — no
  event has ever been mutated in this entire project's history.
- **Ingestion — three real surfaces, one function** ([ADR-0040](adr.d/0040-dated-source-ingestion.md)):
  `POST /api/sources/ingest`, a `ringmaster-ingest` CLI binary, and that
  binary's `mcp-serve` stdio MCP server exposing `ingest_source` — all
  three call the identical Rust function. Verified this session via a raw
  MCP `initialize`/`tools/list`/`tools/call` handshake, not just tests.
  `occurred_at` is required at every surface.
- **Retrieval** ([ADR-0042](adr.d/0042-occurred-at-retrieval-and-recall-sources-mcp-tool.md)):
  `occurred_at` is now readable (it was write-only for a while), date-range
  filtering exists on `GET /api/nodes`, and the MCP server has a second
  tool, `recall_sources`, so an agent can pull dated evidence back out
  without a configured embedding model.
- **Semantic search** ([ADR-0018](adr.d/0018-generate-and-store-source-fragment-embeddings.md)/[ADR-0019](adr.d/0019-semantic-search-over-source-fragments.md)):
  infrastructurally real — Ollama + `nomic-embed-text` is configured and
  `/api/search` responds `200`. **But only 25 of 1,241 source fragments
  (2%) have actually been embedded** — embedding is a deliberate manual
  step, never automatic, so in practice search only covers a sliver of
  what's been ingested today.
- **Risk signals** ([ADR-0041](adr.d/0041-risk-engine-v1-staleness-and-date-compression-signals.md)/[ADR-0046](adr.d/0046-unowned-obligation-risk-signal.md)):
  staleness, date-compression, and unowned-obligation signals are real,
  computed fields on Daily Brief/Time Horizon rows — confirmed directly in
  API responses, not just claimed.
- **Candidate lifecycle** ([ADR-0024](adr.d/0024-candidate-accept-reject-buttons.md)/[ADR-0045](adr.d/0045-correct-candidate-before-accepting.md)):
  accept/reject/promote plus, as of today, correcting a candidate's
  statement or type before accepting it — full audit trail on every action
  ([ADR-0038](adr.d/0038-wire-up-audit-events-for-candidate-validation.md)).
- **Governance**: 46 ADRs, paired evidence records, `check-evidence.mjs`
  reports no `BROKEN`/`STALE`/`DEADHEADED` invariant as of this commit. CI
  (frontend/backend/governance) green.

## The frontend, tab by tab (what I actually saw)

Primary nav is genuinely `Today / Timeline / People / Inbox`, with
`Obligations / Search / Graph / Meetings` demoted under a "Developer" label
— [ADR-0039](adr.d/0039-product-re-steer-primary-navigation.md)'s re-steer
is real, not just decided on paper.

- **Today**: the main ranked list *is* correctly capped at 10 items with an
  honest "N more in Timeline" link ([ADR-0039](adr.d/0039-product-re-steer-primary-navigation.md)/[ADR-0044](adr.d/0044-today-attention-items-management-meaning.md)).
  Rows lead with a plain-language title/evidence quote and a human due-date
  phrase, not a raw id — that part of ADR-0044 is real. **But the "Do these
  together" section right below it (`FocusBlocks`) has no cap at all** —
  right now it renders all 110 focus blocks unfiltered, and it *still*
  shows a raw truncated-UUID chip per item
  (`frontend/src/components/FocusBlocks.tsx`), which is exactly what
  ADR-0044 says it removed from Today. It removed it from the ranked list,
  not from this section — the same page still leaks an id chip a few
  inches lower.
- **Timeline**: the zoomable Time Horizon presentation
  ([ADR-0035](adr.d/0035-time-horizon-timeline-view.md)) — bucket-based,
  not yet aware of a source's `occurred_at` (named explicitly out of scope
  in [ADR-0042](adr.d/0042-occurred-at-retrieval-and-recall-sources-mcp-tool.md);
  still un-done).
- **People**: real — lists person nodes, opens each into existing
  relationship data ([ADR-0028](adr.d/0028-person-relationship-view.md)/[ADR-0039](adr.d/0039-product-re-steer-primary-navigation.md)).
  Fetches *all* person nodes with no pagination, the same pattern as
  FocusBlocks — with 1,007 person nodes currently in the dev DB (see
  below), this tab would currently render an equally enormous, mostly-fake
  list. Not yet hit in practice because I couldn't get a click through
  before the Today page's weight made the tab bar unresponsive.
- **Inbox**: the relabeled Candidates view, unchanged behavior, now with
  the ADR-0045 correction flow.
- **Meetings** (secondary): the new Meeting Review page
  ([ADR-0043](adr.d/0043-meeting-review-page.md)) — lists meetings, opens
  ordered transcript fragments, shows extraction candidates inline. Exists
  and compiles/tests pass; I did not personally click through it this pass.
- **Obligations / Search / Graph** (secondary/developer): the original,
  longest-standing surfaces — table view, semantic search box, and the
  graph explorer with traversal trail
  ([ADR-0033](adr.d/0033-progressive-graph-traversal-trail.md)). Still
  fully functional, just demoted, per ADR-0039's explicit intent.

## The uncomfortable part: what's actually in the database

This is the real finding of this pass. Right now, live:

| Table | Count | Real? |
|---|---|---|
| Obligations | ~2,025 (1,698 flagged "needs attention") | No — reason text is verbatim test fixtures ("Marked at risk. No evidence recorded.", "Due in 1232 day(s)...") |
| Person nodes | 1,007 | No — names like "Filter Test Person", "Relationship Test Person", "Node Route Test Person" |
| Meeting nodes | 505 | No — same story |
| Candidates | 1,008 | No |

None of this is monk-eee's real work. It's the accumulated residue of
**months of `cargo test` runs against a dev Postgres that's never been
reset**, sitting in the exact same database the running app reads from.
This was already suspected (a `daily_brief_reason_cites_linked_evidence`
test flakes intermittently under full-suite load — noted in memory
earlier) but seeing "1656 things need your attention" on the actual Today
page made the scale concrete for the first time. There is currently no
reset/seed/isolation mechanism between test runs and the dev environment a
person would actually look at.

This isn't a backend logic bug — the code is correct, event-sourced, and
does exactly what it's told. It's an operational gap: nothing has ever
needed to distinguish "test wrote this" from "a real event happened,"
because until ingestion (ADR-0040) existed, there was no path for a person
to put real data in at all.

## Why it's built this way

The product thesis (monk-eee's own words, in
[VISION.md](VISION.md#reframed-priority-order)): *"Ringmaster isn't helping
managers manage work. It's helping managers maintain a coherent mental
model of reality."* The build order followed that: event-sourced graph
first (so nothing is ever silently lost or overwritten), Daily
Brief/Relationship View/Time Horizon next (so the model is visible), then
ingestion (so real data can get in at all), then retrieval (so it can come
back out, including to an agent via MCP). Two concurrent AI sessions have
been building against the same repo under a shared ADR-governance
process the whole way — which is why history shows some renumbering
collisions and bundled commits, but also why review/verification (this
document included) keeps catching up honestly rather than assuming green
CI means "done."

## What's explicitly not built (named, not hidden)

- Live Outlook/Teams/Calendar/SharePoint connectors — deliberately deferred
  pending an access-control decision for sensitive data
  ([VISION.md](VISION.md#open-questions-for-future-adrs)).
- Person/participant-to-Person-node linking at ingestion time — participant
  names are still plain strings, not resolved graph edges.
- Natural-language date parsing ("last week") anywhere — every date
  boundary is RFC3339, supplied by the caller.
- The "What am I forgetting?" one-button experience from VISION.md — the
  larger, later product vision; today's Today page is a ranked list, not
  that.
- A test-data reset/seed strategy for the dev database — the gap this
  audit surfaced, not yet an ADR.
- A pagination/cap policy for FocusBlocks and the People/Obligations/
  Candidates list views — everything except the new Today ranked list
  fetches and renders its full table.
