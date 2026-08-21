# Ringmaster — Current status (a real audit, not a decision log)

Updated 2026-08-21 (sixth pass — supersedes the fifth version of this
file, which is quoted below where useful) by re-running
`node scripts/check-evidence.mjs` against a clean, fully-pushed working
tree and reading the ADRs that landed since. [`ARCHITECTURE.md`](ARCHITECTURE.md)
is the formal, decision-level snapshot; this is "what's really there right
now, warts included." As of this pass: **97 ADR records exist on disk, all
97 `PROVEN`, 0 broken/stale/deadheaded** — the fifth audit's two
`DEADHEADED` records (ADR-0094 and the ADR-0002 parity check it tripped)
are resolved: ADR-0094 landed, its evidence companion exists, and the
checker now passes clean end to end.

**Update to the "two concurrent sessions" caveat below**: as of this pass
there is again only **one** active session. The concurrent second session
referenced throughout this document (and responsible for ADR-0094/0096
and the ADR-0092 numbering collision) has ended. The structural findings
below (what works, what doesn't, what's missing) remain the durable part;
the specific "stale by the time you read it" risk from simultaneous edits
no longer applies until a second session starts again.

## What changed since the fifth audit

Three more ADRs landed (0094, 0096, 0097) plus a follow-up commit wiring
0094 into the UI:

- **Candidate synthesis pass** ([ADR-0094](adr.d/0094-candidate-synthesis-pass.md))
  — a live diagnosis against the real (non-test) database found the actual
  cause of "extractions feel ungranular": per-fragment extraction with no
  sibling awareness turned one coherent multi-goal Connect document into
  ten disconnected candidates, and 175 of 227 real candidates sat
  `accepted`-but-never-promoted. Shipped as an insert-only synthesis pass
  that groups same-source candidates before they reach review, backend-only
  in its initial slice, then wired into the Sources review UI in a same-day
  follow-up commit (`db3b82b`).
- **Source review generalized beyond `node_type='meeting'`**
  ([ADR-0096](adr.d/0096-generalize-source-review-beyond-meeting.md)) —
  a live audit found zero `meeting`-type nodes exist while 46 real dated
  sources exist across five other ingested types (`1on1`/`note`/`comms`/
  `perspective`/`connect`), all invisible to the one UI built to browse a
  source's transcript alongside its candidates side by side. The Meetings
  tab's routes and component now accept any source type with a
  `has_source_fragments` filter, relabeled honestly as "Sources."
- **Candidate accept/reject over MCP, and a Node identity/lifecycle edit
  form** ([ADR-0097](adr.d/0097-candidate-mcp-and-node-edit-form.md)) —
  closes two gaps ADR-0093 explicitly named out of scope: an MCP agent
  could ingest and read candidates but not triage one, and Graph
  Explorer's only edit affordance for a Node was a raw "Enrich attributes
  (JSON)" textarea. Both now share the existing per-surface functions
  (ADR-0024's accept/reject, ADR-0025/0066's `canonical_text`/
  `lifecycle_state` PATCH fields) rather than adding new logic.

## What changed since the fourth audit

Five more ADRs landed (0090–0093, 0095 — 0092 and 0094 in the sequence
belong to a concurrent session's own work, not this one):

- **CI enforces `npm audit`/`cargo audit`** ([ADR-0090](adr.d/0090-ci-enforced-dependency-vulnerability-scanning.md))
  — closes the loop ADR-0089's Vite/nanoid patch opened: a future
  dependency bump can no longer silently reintroduce a known-vulnerable
  package without CI catching it. Verified locally (`npm audit`: 0
  vulnerabilities) before landing, then a concurrent session found and
  fixed a real CI-only gap this pass ([ADR-0092](adr.d/0092-fix-ci-cargo-audit-permissions-and-document-rsa-advisory.md)):
  `rustsec/audit-check` needs `checks: write`, which the default
  `GITHUB_TOKEN` permissions didn't grant, so the job failed in live CI
  even though `cargo audit` itself ran clean locally — confirmed fixed in
  a real CI run (`32223737027`).
- **People view redesign** ([ADR-0091](adr.d/0091-people-view-avatar-and-badge-redesign.md))
  — direct response to "i fucking hate the ui... especially the people
  view" plus a reference screenshot. Colored initials avatars and
  pill-shaped status badges, reusing the existing status color tokens
  (ADR-0074), scoped to the People tab only.
- **Obligation editing, closing a real 100%-read-only gap**
  ([ADR-0093](adr.d/0093-obligation-editing-across-surfaces.md)) — an
  audit prompted directly by "we should be able to edit via the api the
  cli or the mcp and the ui" found Obligations had literally no edit
  surface anywhere (status could only ever be set at creation). One
  shared `obligation::update_status` function now backs a `PATCH
  /api/obligations/:id` route, a new `update-obligation` CLI subcommand,
  a new `update_obligation` MCP tool, and an Edit form on
  `ObligationDetail.tsx` — verified with new Rust unit tests, a live
  PATCH call against the running app, and a UI edit/revert cycle.
- **Shared row typography decluttered** ([ADR-0095](adr.d/0095-daily-brief-row-decluttering.md),
  renumbered from a colliding 0092 — see below) — a real CSS bug
  (`.daily-brief-list li`, no child combinator, was leaking the outer
  row's padding/border onto its own nested risk-signal `<li>`s) plus
  un-rendered literal `**bold**` markdown in real evidence quotes were
  the two concrete, evidenced causes of "everything feels crowded."
  Fixed across every component sharing this one pattern (Today, Timeline,
  Focus Blocks, "What am I forgetting?", Workbench, Graph's person view,
  People's relationship groups) via a new `renderBoldSegments` helper
  (never `dangerouslySetInnerHTML`) and reusing ADR-0091's pill-badge
  visual language for risk signals.
- **A real ADR numbering collision, caught and fixed**: two sessions
  independently checked "highest ADR is 0091" within minutes of each
  other and both claimed 0092 for genuinely different topics. Caught via
  a post-push `git log --oneline` sanity check surfacing an unfamiliar
  commit message; fixed with a `git mv` renumber to the next free number
  (0095 — 0094 was *also* already claimed by the same concurrent session's
  candidate-synthesis-pass ADR) and every internal cross-reference
  updated, landed as its own follow-up commit rather than an amend+force-push
  of the already-pushed commit.

## What changed since the second audit

Eight more ADRs landed (0080–0087), closing every bounded item in
`docs/IMPROVEMENT-PLAN.md`'s Priority 0–2 backlog:

- **Graph Explorer promoted to primary navigation**
  ([ADR-0080](adr.d/0080-promote-graph-explorer-to-primary-navigation.md)),
  plus an **Actions lens** that filters the radial neighbourhood down to
  what needs doing ([ADR-0081](adr.d/0081-graph-explorer-actions-lens.md)).
  Primary nav is now confirmed live as `Today / Timeline / People / Inbox /
  Graph`, not the four-tab set the second audit recorded.
- **Repeated-concern risk signal**
  ([ADR-0082](adr.d/0082-repeated-concern-risk-signal.md)) — the same
  concern appearing across independent sources now surfaces as a fifth
  risk signal. Its test flakiness was root-caused twice: an initial
  one-hot-vector fix (`cd8f804`) still collided as the shared
  `ringmaster_test` database grew; the real fix
  (`1b1842c`) generates genuinely random continuous embedding vectors, no
  discrete collision space, verified via two clean full-suite runs.
- **Meeting-brief generation**
  ([ADR-0083](adr.d/0083-meeting-brief-generation.md)) — a
  `GET /api/people/:id/brief` route and a `prepare_meeting_brief` MCP tool
  compose a person's open commitments and recent asks with source
  citations, verified live over both HTTP and a raw MCP stdio handshake.
- **Today's narrative summary**
  ([ADR-0084](adr.d/0084-today-narrative-summary.md)) — a time-of-day
  greeting plus honest date-compression/stale counts (zero counts
  omitted, not shown as "0").
- **Focus Blocks People/All filter**
  ([ADR-0085](adr.d/0085-focus-blocks-people-filter.md)) — deliberately
  scoped down from `VISION.md`'s full People/Delivery/Leadership/
  Operations taxonomy (no `kind` field exists on `Obligation` to build
  that honestly) to the one real, groundable split: person-linked vs.
  everything else.
- **Workbench, a three-pane view**
  ([ADR-0086](adr.d/0086-workbench-three-pane-view.md)) — a new secondary
  tab composing three already-proven reads (`DailyBrief`,
  `ObligationDetail`, and ADR-0083's person brief) with zero backend
  changes; ships additive, does not replace Today.
- **Graph Explorer create-node reliability under concurrent Playwright
  load** ([ADR-0087](adr.d/0087-graph-explorer-reliability-under-concurrent-load.md))
  — root-caused a session-long "pre-existing flaky test" caveat
  (mis-attributed to shared-database growth) to real worker-concurrency
  contention against one shared Playwright backend/Vite pair, proven via
  a `--workers=1` vs. default-worker A/B (0/6 pass → 6/6 pass after the
  fix), plus an unrelated but genuine latency bug (an unbounded node-list
  refresh unnecessarily blocking the new node's own detail fetch).
- **ADR-0004 re-affirmed twice this pass** (`834cb96`, then `e67e2d3`
  with more detail) — its `no-sensitive-data-sharing-path` manual check
  now checker-confirms clean through ADR-0086, moving it from `ASSERTED`
  to `PROVEN` for the first time since it was accepted.

The second audit's specific findings (data volume, FocusBlocks cap,
People tab, container staleness) are unchanged and still hold; quoted
below where still useful.

## What changed since the third audit

Four more ADRs landed (0086–0089):

- **Workbench, a three-pane view** ([ADR-0086](adr.d/0086-workbench-three-pane-view.md))
  and **Graph Explorer reliability under concurrent load**
  ([ADR-0087](adr.d/0087-graph-explorer-reliability-under-concurrent-load.md))
  were already recorded in the third audit's own "since second audit"
  section above (both landed the same pass).
- **Career/Connect export** ([ADR-0088](adr.d/0088-career-connect-export.md))
  — closes `docs/IMPROVEMENT-PLAN.md` Priority 4's first item. A new
  `person_career_history` read returns every *closed* Obligation linked to
  a person with an evidence citation — the honest opposite of
  `person_brief`'s open-only filter and `get_node_detail`'s relationship
  grouping, both of which explicitly exclude closed rows. Deliberately
  unfiltered by People/Delivery/Leadership/Operational category, since no
  such classification exists anywhere in this schema (confirmed again:
  `obligation_projection` has no `kind` column). Rendered as a
  copy-to-clipboard "Career export" section on Person detail.
- **BCDR/compliance dedicated view named as honestly blocked, not
  attempted**: a fresh audit (2026-08-19) confirmed no `service`/
  `compliance`-style node type is actually created anywhere in this
  codebase outside `PRODUCT-SPEC.md`'s own aspirational node-type table
  (only `person`/`meeting`/`risk` are real in practice) — there is no
  honest signal to build a dedicated view on without inventing a
  classification, so `docs/IMPROVEMENT-PLAN.md` now names this blocked
  rather than silently dropped.
- **Stale container, found and fixed again this pass**: `ringmaster-backend-1`
  was still serving `bb59994` (a commit from *before* ADR-0082), six ADRs
  behind `2d548b9` — the exact recurring gotcha ADR-0078 exists to
  surface. Confirmed live before the fix: opening a real person's detail
  page showed the Relationship section but no Career export section at
  all. `docker compose build backend frontend && docker compose up -d
  --force-recreate backend frontend` fixed it (backend now logs `built
  from 2d548b9c57ed`); reloading the same person's detail page afterward
  showed the Career export section rendering its honest empty state
  ("Nothing completed recorded yet.") exactly as designed.
- **Security: two high-severity Vite/nanoid advisories patched**
  ([ADR-0089](adr.d/0089-patch-vite-nanoid-security-advisories.md)) —
  landed concurrently with this pass, worth flagging precisely because
  `frontend/Dockerfile`'s `CMD ["npx", "vite"]` means the container this
  repo actually runs *is* Vite's dev server, not a built static bundle —
  a path-traversal/NTLM-hash-disclosure advisory in it is a live
  vulnerability in the running app, not a deferred dev-tooling concern.
  Fixed via a minimal `vite@^5.4.8` → `^6.4.3` bump (not the major-version-8
  jump `npm audit fix --force` would have applied) plus a lockfile-only
  `npm audit fix` for the transitive `nanoid` advisory.

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
  confirmed present on the running Postgres), then enforced for backend tests
  by [ADR-0057](adr.d/0057-enforce-test-database-isolation-with-a-runtime-guard.md).
  [ADR-0073](adr.d/0073-isolate-playwright-from-dev-database.md) closed the
  browser-test path: Playwright starts its own backend/Vite pair on
  dedicated ports against `ringmaster_test`, so it can no longer write to
  this database. **Update, 2026-08-18 (later pass):** the ~392 pre-fix
  Person-node fixtures (plus 1 stray `meeting` fixture and 3
  `trailtest*`-typed nodes the original heuristic didn't cover) were
  cleaned via [ADR-0056](adr.d/0056-local-test-database-isolation-and-dev-data-cleanup.md)'s
  already-drafted `scripts/dev-data-cleanup.sql`, after a `pg_dump` backup,
  a read-only `dev-data-report.sql` review, and a concurrency check
  (`pg_stat_activity`). 383 fixture nodes and 7 referencing edges removed;
  13 real Person nodes remain. Candidates/Obligations/source_fragments were
  untouched (out of this script's scope by design).
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
- **Graph management over MCP** ([ADR-0066](adr.d/0066-non-destructive-graph-management-over-mcp.md)):
  entity list/get/create/update, atomic exact-match batch upsert, and
  relationship list/create are first-class tools. Attributes shallow-merge;
  ambiguous identities fail rather than being silently combined.
- **Semantic search** ([ADR-0018](adr.d/0018-generate-and-store-source-fragment-embeddings.md)/[ADR-0019](adr.d/0019-semantic-search-over-source-fragments.md)):
  live and populated: the accepted reindex operation appended embeddings for
  all 358 source fragments using local `nomic-embed-text`, and
  `/api/search?q=deployment%20risk&limit=3` returned ranked, relevant source
  fragments. New ingests auto-embed best-effort; reindex remains available to
  backfill fragments created while no embedding model was configured.
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
- **Governance**: 97 ADRs, all 97 `PROVEN`, 0 broken. CI green.

## The frontend, tab by tab (what I actually saw, this pass)

Primary nav: `Today / Timeline / People / Inbox / Graph` (Graph promoted
from secondary, ADR-0080), secondary/"Developer": `Obligations / Search /
Sources / Activity / Workbench` (Workbench is new, ADR-0086; the tab
labeled "Meetings" is now labeled "Sources" and browses any ingested
source type, not just `node_type='meeting'`, per ADR-0096).

- **Today**: greeting → narrative summary (date_compression/stale counts,
  ADR-0084) → ranked list (capped 10, "31 more in Timeline →") →
  **"What am I forgetting?"** (capped 5, ranked by signal count — watched
  it correctly surface the most-flagged items first) → **"Do these
  together"** (capped 3, urgency-ordered, bucket-labeled, "Show all N",
  now with a People/All filter toggle when both kinds of block are
  present — ADR-0085) → "Coming soon" strip (still capped 3/window,
  unchanged). Every row is clickable into detail. This is now a genuinely
  coherent attention budget, not a data dump — the single biggest change
  since the first audit.
- **People**: defaults to who-needs-attention (verified via direct API
  call: `?needs_attention=true` returns a strict subset of the unfiltered
  list). Detail view shows `last_interaction_at` as a relative phrase,
  risk signals per linked obligation, and a capped, source-cited Recent
  interactions section covering both identity edges and the legacy speaker
  fallback ([ADR-0071](adr.d/0071-person-detail-recent-interactions.md)).
- **Navigation on narrow screens**: all ten destinations remain reachable
  through horizontal scrolling contained inside the tab list; the document
  itself stays viewport-width at 390px.
- **Timeline**: now surfaces a linked source's own `occurred_at` as
  display-only detail ("Source occurred ...") on expanded items
  ([ADR-0079](adr.d/0079-timeline-surfaces-source-occurred-at.md)) — the
  second audit's "still not aware of occurred_at" finding is no longer
  current. Bucket placement itself is still due-date-only, unchanged by
  design.
- **Graph**: now primary nav, not secondary (ADR-0080). An Actions lens
  filters the radial neighbourhood to Obligations/risk nodes only,
  stating the shown/filtered count honestly (ADR-0081). Create-node →
  select flow is now measurably more reliable under concurrent load
  (ADR-0087).
- **Workbench** (new, ADR-0086): a three-pane layout — Attention (reuses
  `DailyBrief`), Current focus (reuses `ObligationDetail`), Relationship
  context (new `PersonBriefPanel`, composing ADR-0083's person-brief
  read). Confirmed live: selecting an item fills the centre and right
  panes; an honest empty state renders when nothing is selected or no
  person is linked.
- **Sources** (renamed from Meetings, ADR-0096): now shows every ingested
  source type (`1on1`/`note`/`comms`/`perspective`/`connect`, not just the
  never-populated `meeting` type), each with its transcript, extracted
  candidates, and — since the ADR-0094 follow-up commit (`db3b82b`) — a
  synthesis-groups view composing same-source candidates before review.
- **Inbox / Obligations / Search / Activity**: all present and functional
  per prior verification; not re-clicked through this pass (no changes
  landed there since the last audit that I'm aware of).

## The database, right now (a snapshot that's already changing)

The first audit's reduction was real, and backend test isolation remains
enforced. On 2026-08-18 (later pass) the dev-data cleanup was actually run
for the first time: 383 fixture nodes (person-fixture prefixes, one stray
`meeting` fixture, and 3 `trailtest*`-typed nodes a heuristic gap missed)
and their 7 referencing edges were removed after a backup, a read-only
report review, and a concurrency check. 13 real Person nodes remain. The
`ringmaster_test` database genuinely exists on the running Postgres, and as of
[ADR-0057](adr.d/0057-enforce-test-database-isolation-with-a-runtime-guard.md)
it is now *enforced*, not merely documented: every backend `test_pool()`
calls a runtime guard (`backend/src/lib.rs`) that panics unless
`DATABASE_URL` targets `ringmaster_test`, so a stray `cargo test` against
the long-lived `ringmaster` database a person reads at `localhost:3001`
now fails loudly instead of silently polluting it. Browser-test prevention is
now closed by [ADR-0073](adr.d/0073-isolate-playwright-from-dev-database.md):
Playwright's backend and Vite instance run on dedicated ports (18080/13001)
against `ringmaster_test`, with the backend refusing to start against any
other database when `RINGMASTER_REQUIRE_TEST_DATABASE=true`.

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
- Playwright previously ran against the development backend/database and
  repopulated the People list with hundreds of fixtures.
  [ADR-0073](adr.d/0073-isolate-playwright-from-dev-database.md) moved
  browser tests to dedicated ports over `ringmaster_test`; the existing
  polluted rows were cleaned separately on 2026-08-18 (see "The database,
  right now" above) — no longer an open item.
- Participant/speaker names now resolve to existing Person nodes by exact,
  case-insensitive name during new ingestion and create `participated_in`
  edges. `last_interaction_at` now uses those identity edges while retaining
  the legacy speaker fallback for older sources. Fuzzy matching, Person-node
  creation for unknown names, and backfill of older sources remain deliberately
  out of scope. Person detail now exposes the readable Past interaction list
  through [ADR-0071](adr.d/0071-person-detail-recent-interactions.md).
- Natural-language date parsing ("last week") anywhere — every date
  boundary is RFC3339, supplied by the caller.
- "Upcoming conversation" on a person's page — no calendar/future-meeting
  source exists; explicitly refused rather than fabricated
  ([ADR-0051](adr.d/0051-relationship-workspace.md)).
- The full "commitment exists, no supporting work exists" Congruence
  Engine — [ADR-0054](adr.d/0054-congruence-engine-v1-isolated-commitment-signal.md)
  only ships the narrow, honest slice (zero linked edges at all); checking
  against real delivery work needs ADO ingestion that doesn't exist yet.
- People, Obligations, and Candidates list views now page in batches of 50
  with an explicit Load more control ([ADR-0059](adr.d/0059-list-view-pagination.md));
  the earlier fetch-all audit finding is no longer current.

