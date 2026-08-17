# Ringmaster — Current status (a real audit, not a decision log)

Updated 2026-08-17 (second pass — supersedes the first version of this
file, which is quoted below where useful) by clicking through the running
app again, querying the live dev database again, and reading the code
behind what's there — not by summarizing ADR titles.
[`ARCHITECTURE.md`](ARCHITECTURE.md) is the formal, decision-level
snapshot; this is "what's really there right now, warts included." As of
this pass: **56 ADRs, 54 `PROVEN`, 1 `ASSERTED` (ADR-0004, a long-standing
manual claim), 0 `BROKEN`/`STALE`/`DEADHEADED`**, CI green at `26ab431`.

**A honest caveat before anything else**: this repo is being built by two
concurrent AI sessions against the same working directory, right now,
continuously. HEAD moved twice *while this exact audit was being written*
(commits landed mid-investigation). Any specific row count below is
already stale by the time you read it — the structural findings (what
works, what doesn't, what's missing) are the durable part.

## What changed since the first audit

The first pass of this document found the single biggest problem was
**~99% test-fixture noise in the dev database** (~2,025 fake obligations,
1,007 fake person nodes) making the Today page unusable, plus two
frontend gaps (an uncapped, still-raw-id-leaking "Do these together"
section, and a database-browser-style People tab). All three are now
fixed and verified live:

- **Data volume**: down from ~2,025/1,007/1,008 (obligations/people/
  candidates) to single/low-double-digits, via
  [ADR-0056](adr.d/0056-local-test-database-isolation-and-dev-data-cleanup.md)
  (a separate `ringmaster_test` database, now genuinely created and
  confirmed present on the running Postgres). **Caveat, found this pass**:
  the fix is a *documented convention*
  ([CONTRIBUTING.md](CONTRIBUTING.md)), not an enforced one — nothing stops
  a `cargo test` invocation from still pointing at the dev database by
  habit or mistake. I personally ran `cargo test` against the dev database
  for this entire session before noticing the new convention existed;
  several of the "real-looking" fixtures currently on the live Today page
  (e.g., a person node named "Bucket Split Test Person") are residue from
  my own test runs, not monk-eee's work. The prevention is real; the
  discipline to actually use it isn't automatic yet.
- **FocusBlocks cap + no raw id** ([ADR-0050](adr.d/0050-today-attention-budget.md)):
  confirmed live — "Do these together" now shows at most 3 blocks with a
  "Show all N" control, zero raw ids anywhere.
- **People tab** ([ADR-0051](adr.d/0051-relationship-workspace.md)):
  confirmed live — defaults to a `needs_attention` filter with a "Show
  everyone" toggle; person detail now carries `last_interaction_at` and
  per-obligation `risk_signals`.
- **New this pass, also confirmed live**:
  [ADR-0052](adr.d/0052-context-derived-focus-sessions.md) (Focus Blocks
  split by node *and* Time Horizon bucket — watched the same test person
  appear in two separate blocks, "Next 7 Days" and "Next 90 Days", exactly
  as designed) and
  [ADR-0053](adr.d/0053-what-am-i-forgetting.md) ("What am I forgetting?"
  — a capped, ranked section between the main list and "Do these
  together", showing exactly 5 flagged items with real signal text) and
  [ADR-0054](adr.d/0054-congruence-engine-v1-isolated-commitment-signal.md)
  (a new `isolated` risk signal — saw "Not linked to anyone or anything."
  rendering live, alongside `stale`/`unowned`/`date_compression`).
- **CI had a rocky patch**: four consecutive commits failed CI's
  `governance` job (not backend/frontend — a `check-evidence.mjs`/
  `git diff --check` gate, not a logic bug) during the ADR-0056 rollout,
  now resolved as of the latest commit.
- **Stale containers, found and fixed this pass**: the running
  `ringmaster-backend-1`/`ringmaster-frontend-1` containers were still
  serving an image built *before* the latest commit landed (confirmed via
  `podman inspect --format '{{.Created}}'` on both the image and the
  commit timestamp) — Today briefly rendered "120 things need your
  attention" with mostly generic, evidence-less text, which looked like a
  capping regression but wasn't one. `podman compose build backend
  frontend && podman compose up -d --force-recreate backend frontend`
  fixed it; reloading afterward showed the true, small, evidence-backed
  list. Matches a gotcha already in repo memory ("running compose
  containers go stale silently") — worth checking *first*, before
  assuming a code regression, whenever the live app disagrees with what
  the source/evidence says it should do.

## What's actually real and working

- **Graph substrate** (nodes/edges/source_fragments, [ADR-0009](adr.d/0009-add-graph-nodes-edges-and-source-fragments.md)):
  real, immutable evidence fragments (DB trigger rejects UPDATE/DELETE),
  polymorphic edges with temporal validity (supersede-on-write,
  [ADR-0032](adr.d/0032-temporal-edge-validity-supersede-on-write.md)).
- **Obligations** are event-sourced: append-only `obligation_events` +
  a rebuilt-from-scratch `obligation_projection`, never patched in place.
- **Ingestion — three real surfaces, one function** ([ADR-0040](adr.d/0040-dated-source-ingestion.md)):
  `POST /api/sources/ingest`, a `ringmaster-ingest` CLI binary, and that
  binary's `mcp-serve` stdio MCP server exposing `ingest_source` — all
  three call the identical Rust function, `occurred_at` required at every
  surface.
- **Retrieval** ([ADR-0042](adr.d/0042-occurred-at-retrieval-and-recall-sources-mcp-tool.md)):
  `occurred_at` is readable and date-range filterable on `GET /api/nodes`;
  the MCP server's `recall_sources` tool needs no embedding model.
- **Semantic search** ([ADR-0018](adr.d/0018-generate-and-store-source-fragment-embeddings.md)/[ADR-0019](adr.d/0019-semantic-search-over-source-fragments.md)):
  infrastructurally real (Ollama + `nomic-embed-text` configured,
  `/api/search` responds `200`), but **currently 0 embeddings exist** (the
  database reset wiped the 25 that existed at the last audit; embedding is
  a deliberate manual step, never automatic) — right now, search would
  return nothing for any query, worth knowing before demoing it.
- **Risk signals, now four**: `stale`/`date_compression`
  ([ADR-0041](adr.d/0041-risk-engine-v1-staleness-and-date-compression-signals.md)),
  `unowned` ([ADR-0046](adr.d/0046-unowned-obligation-risk-signal.md)),
  `isolated` ([ADR-0054](adr.d/0054-congruence-engine-v1-isolated-commitment-signal.md))
  — all four confirmed rendering together on real rows this pass.
- **Candidate lifecycle** ([ADR-0024](adr.d/0024-candidate-accept-reject-buttons.md)/[ADR-0045](adr.d/0045-correct-candidate-before-accepting.md)):
  accept/reject/promote plus correcting a candidate before accepting it,
  full audit trail ([ADR-0038](adr.d/0038-wire-up-audit-events-for-candidate-validation.md)),
  a flat Activity feed of recent audit rows ([ADR-0049](adr.d/0049-audit-trail-read-api.md)).
- **Obligation detail page** ([ADR-0047](adr.d/0047-obligation-detail-page.md)):
  new since the last audit — clicking any Today row now opens a full
  detail view (confirmed: rows render as real `<button>` elements, not
  static text).
- **Governance**: 55 ADRs, 54 `PROVEN`, 1 `ASSERTED` (ADR-0004,
  policy-only by design), 0 broken. CI green.

## The frontend, tab by tab (what I actually saw, this pass)

Primary nav: `Today / Timeline / People / Inbox`, secondary/"Developer":
`Obligations / Search / Graph / Meetings / Activity` (Activity is new).

- **Today**: greeting → ranked list (capped 10, "31 more in Timeline →") →
  **"What am I forgetting?"** (capped 5, ranked by signal count — watched
  it correctly surface the most-flagged items first) → **"Do these
  together"** (capped 3, urgency-ordered, bucket-labeled, "Show all N") →
  "Coming soon" strip (still capped 3/window, unchanged). Every row is
  clickable into detail. This is now a genuinely coherent attention
  budget, not a data dump — the single biggest change since the last
  audit.
- **People**: defaults to who-needs-attention (verified via direct API
  call: `?needs_attention=true` returns a strict subset of the unfiltered
  list). Detail view shows `last_interaction_at` as a relative phrase and
  risk signals per linked obligation.
- **Timeline**: unchanged since last audit — still bucket-based, still not
  aware of a linked source's `occurred_at` (named out of scope in
  [ADR-0042](adr.d/0042-occurred-at-retrieval-and-recall-sources-mcp-tool.md),
  still true).
- **Inbox / Meetings / Obligations / Search / Graph / Activity**: all
  present and functional per prior verification; not re-clicked through
  this pass (no changes landed there since the last audit that I'm aware
  of).

## The database, right now (a snapshot that's already changing)

At the moment of writing: single-digit-to-thirties counts across
obligations/people/candidates (9 obligations / 21 nodes / 24 candidates
on this pass) — down from the ~2,025/1,007/1,008 the first audit found,
and now staying down. The reduction is real, the isolation mechanism
(`ringmaster_test`) genuinely exists on the running Postgres, and as of
[ADR-0057](adr.d/0057-enforce-test-database-isolation-with-a-runtime-guard.md)
it is now *enforced*, not merely documented: every backend `test_pool()`
calls a runtime guard (`backend/src/lib.rs`) that panics unless
`DATABASE_URL` targets `ringmaster_test`, so a stray `cargo test` against
the long-lived `ringmaster` database a person reads at `localhost:3000`
now fails loudly instead of silently polluting it. The residue the first
audit found has been cleared and the guard stops it recurring.

## Why it's built this way

Unchanged from the first audit: the product thesis (monk-eee's own words,
[VISION.md](VISION.md#reframed-priority-order)) — *"Ringmaster isn't
helping managers manage work. It's helping managers maintain a coherent
mental model of reality."* Two concurrent AI sessions have built this
under a shared ADR-governance process the whole way; this pass's own
mid-audit HEAD movements are just the latest instance of that.

## What's explicitly not built (named, not hidden)

- Live Outlook/Teams/Calendar/SharePoint connectors — deliberately deferred
  pending an access-control decision for sensitive data
  ([VISION.md](VISION.md#open-questions-for-future-adrs)).
- Person/participant-to-Person-node linking at ingestion time — participant
  names, and `last_interaction_at`'s speaker match, are still plain-string
  based, not resolved graph edges.
- Natural-language date parsing ("last week") anywhere — every date
  boundary is RFC3339, supplied by the caller.
- "Upcoming conversation" on a person's page — no calendar/future-meeting
  source exists; explicitly refused rather than fabricated
  ([ADR-0051](adr.d/0051-relationship-workspace.md)).
- The full "commitment exists, no supporting work exists" Congruence
  Engine — [ADR-0054](adr.d/0054-congruence-engine-v1-isolated-commitment-signal.md)
  only ships the narrow, honest slice (zero linked edges at all); checking
  against real delivery work needs ADO ingestion that doesn't exist yet.
- A pagination/cap policy for the People/Obligations/Candidates *list*
  views (as opposed to Today's sections, which are now all capped) —
  still fetch-all, per the first audit's finding.

