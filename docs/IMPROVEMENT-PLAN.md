# Ringmaster — Improvement plan

**Status:** Draft candidate backlog. This is not an ADR and authorizes no
implementation. Per [AGENTS.md](../AGENTS.md), every item below still needs
its own bounded ADR (or an explicit statement that an existing accepted ADR's
scope already covers it) and explicit acceptance from the named decider
before any code changes.

**Basis:** a real audit of the running app, the database, and the 62-ADR
index as of 2026-08-18 (see [current-status.md](current-status.md)), plus a
gap analysis against a text-file-based management workflow (Learn.ADOA) that
Ringmaster is meant to replace. Ordered by leverage: fix trust first, then
close named-but-unbuilt gaps, then the UX reframe VISION.md already
describes, then new source coverage, then new domain coverage.

**Before starting anything:** re-read `current-status.md` fresh. Two
concurrent AI sessions build this repo continuously; any specific claim in
this plan may already be stale by the time it's read.

---

## Priority 0 — Data hygiene and trust (no new design, low risk)

**Status: done.** 0.1 ([ADR-0056](adr.d/0056-local-test-database-isolation-and-dev-data-cleanup.md)),
0.2 ([ADR-0078](adr.d/0078-log-build-provenance-to-detect-stale-containers.md)), and
0.3 (re-affirmed, [EV-0004](../evidence.d/0004-defer-multi-user-access-control-single-user-v1.md),
2026-08-19) have all landed.

### 0.1 Clean up the 392 stray Person nodes

`ringmaster_test` isolation is now enforced for both `cargo test`
([ADR-0057](adr.d/0057-enforce-test-database-isolation-with-a-runtime-guard.md))
and Playwright
([ADR-0073](adr.d/0073-isolate-playwright-from-dev-database.md)), but the
392 fixture Person nodes already written into the long-lived `ringmaster`
database before that fix landed are still there, named and counted in
`current-status.md`, and explicitly not authorized for cleanup by either
ADR. A dashboard a manager actually trusts cannot have 357 rows named
`Pagination Test Person` sitting in it. This needs its own small ADR: a
reviewable, non-destructive-by-default cleanup (matching the caution already
shown in
[ADR-0056](adr.d/0056-local-test-database-isolation-and-dev-data-cleanup.md)'s
"reviewable, not auto-run" pattern), scoped to the exact fixture name list
already identified.

### 0.2 Detect stale containers automatically

`current-status.md` documents a real incident: the running
`ringmaster-backend-1`/`ringmaster-frontend-1` containers served an image
built before the latest commit, and it looked like a capping regression
until `podman inspect --format '{{.Created}}'` proved otherwise. This has
already happened once and cost investigation time. A cheap fix: a
`podman compose` healthcheck or a startup log line comparing the running
image's build timestamp against the current git HEAD commit timestamp, so
the mismatch is visible on `podman compose logs` instead of requiring a
manual `podman inspect` comparison next time.

### 0.3 Re-affirm ADR-0004

`current-status.md` names ADR-0004 (defer multi-user access control) as
the one `ASSERTED`-not-`PROVEN` record among 62 accepted ADRs — a
long-standing manual claim rather than checker-derived evidence. It may be
correctly un-provable by design (single-user, local, unshared, policy-only)
rather than stale. No code change either way; just re-read it and confirm
the position still holds before it becomes stale-by-assumption.

---

## Priority 1 — Close the named-but-unbuilt gaps

**Status: done.** 1.1 ([ADR-0082](adr.d/0082-repeated-concern-risk-signal.md)),
1.2 ([ADR-0083](adr.d/0083-meeting-brief-generation.md)), and
1.3 ([ADR-0079](adr.d/0079-timeline-surfaces-source-occurred-at.md)) have all landed.

These are already stated as intent in `VISION.md`/`PRODUCT-SPEC.md` or
directly answer a workflow Learn.ADOA still does by hand. None appear in the
62-ADR index.

### 1.1 Repeated-concern risk signal (Congruence Engine v2)

Four risk signals are live today: `stale`, `date_compression`, `unowned`,
`isolated`
([ADR-0041](adr.d/0041-risk-engine-v1-staleness-and-date-compression-signals.md),
[0046](adr.d/0046-unowned-obligation-risk-signal.md),
[0054](adr.d/0054-congruence-engine-v1-isolated-commitment-signal.md)).
`PRODUCT-SPEC.md` §7.1 also names "Repeated concern — the same risk appears
in multiple meetings without mitigation," but no ADR implements it. This is
the direct replacement for Learn.ADOA's manual `THEMES.md` promotion rule
(single source → Watch list, second independent source → numbered theme) —
here it becomes a query over distinct source Meetings linked to
semantically-similar Risk/Obligation nodes, not a human remembering a rule.
Reuses the existing risk-signal composition pattern from
[ADR-0053](adr.d/0053-what-am-i-forgetting.md); does not need a new subsystem.

### 1.2 Meeting-brief generation

`PRODUCT-SPEC.md` §8.3 lists "Prepare a factual brief for my next management
1:1, with sources" as an example agent query. No ADR implements it. Today
the evidence exists (Person detail's Recent Interactions, linked
Obligations, risk signals — [ADR-0071](adr.d/0071-person-detail-recent-interactions.md),
[0028](adr.d/0028-person-relationship-view.md)) but nothing composes it into
the artifact a manager would actually take into a 1:1. Scope this narrowly
first: a read-only endpoint/MCP tool that, given a person, returns their
open commitments, recent asks, and outstanding risks with source citations
— composition over existing data, not new extraction.

### 1.3 Timeline awareness of `occurred_at`

Named out of scope in [ADR-0042](adr.d/0042-occurred-at-retrieval-and-recall-sources-mcp-tool.md)
and confirmed still true two audits later. The Timeline view is still
bucket-based and does not reflect when a linked source actually happened.
Lowest-novelty item in this plan — the field already exists and is
retrievable; this is wiring, not design.

---

## Priority 2 — The UX reframe `VISION.md` already describes

**Status: done.** 2.1 ([ADR-0084](adr.d/0084-today-narrative-summary.md)),
2.2 ([ADR-0085](adr.d/0085-focus-blocks-people-filter.md)), and
2.3 ([ADR-0086](adr.d/0086-workbench-three-pane-view.md)) have all landed.

`VISION.md`'s "The UX is the product" section (2026-08-14) explicitly says
it "sharpens" the current panel-based Today/Focus Blocks implementation
into something not yet built. Three concrete gaps between what's live and
what's described:

### 2.1 Daily Brief as narrative, not a panel list

Today currently renders greeting → capped ranked list → "What am I
forgetting?" → "Do these together" → a capped "coming soon" strip
(`current-status.md`, confirmed live). `VISION.md`'s own mockup is a single
ranked prose brief ("4 things need attention today. 2 will become risks
this week...") — a narrative summary of the same underlying data, not an
additional panel. This is a presentation change over existing Today data,
not a new signal.

### 2.2 Focus Sessions clustered by attention-type, not just shared node

[ADR-0031](adr.d/0031-suggested-focus-blocks.md)/[0052](adr.d/0052-context-derived-focus-sessions.md)
group Obligations sharing a linked node and time bucket. `VISION.md`
describes a different, complementary grouping: by *kind of attention*
(People / Delivery / Leadership / Operations), so a manager can run a
"People Focus Session" end to end instead of context-switching between
unrelated obligation types. Additive to the existing Focus Blocks feature,
not a replacement.

### 2.3 Three-pane workbench (Attention / Current focus / Relationship context)

Current navigation is tab-based (`Today / Timeline / People / Inbox`,
[ADR-0039](adr.d/0039-product-re-steer-primary-navigation.md)). `VISION.md`
describes a workbench layout where selecting an item in an attention list
fills a centre pane with its full context and a right pane with the
relevant person's relationship state, without a page navigation. This is
the largest frontend change in this plan and should be scoped as its own
ADR rather than folded into 2.1 or 2.2.

---

## Priority 3 — Source coverage (needs a prerequisite access-control ADR)

`current-status.md` states Live Outlook/Teams/Calendar/SharePoint
connectors are "deliberately deferred pending an access-control decision for
sensitive data" (`VISION.md` open questions), and that the full Congruence
Engine (checking a commitment against real delivery work) "needs ADO
ingestion that doesn't exist yet." Recommend ADO first: `PRODUCT-SPEC.md`
already scopes it as a "Read first" provider, Ringmaster's own philosophy is
"ADO remains the work system" (so this is linking, not replacing
`ado-board-planner`), and it directly unblocks 1.1's full Congruence Engine
scope. The access-control ADR itself — not any specific connector — is the
actual first decision needed here.

---

## Priority 4 — Domain coverage not yet modeled as a first-class feature

Lower priority: these map to real Learn.ADOA workflows but have no
confirmed usage pressure on Ringmaster yet, and the underlying domain types
already exist.

- **Career/Connect evidence export. Done** ([ADR-0088](adr.d/0088-career-connect-export.md),
  2026-08-19): a new `GET /api/people/:id/career-export` read plus a
  Person-detail section render every closed Obligation linked to a person
  as plain, copy-to-clipboard text with evidence citations. Honestly
  unfiltered by People/Delivery/Leadership/Operational category, since no
  such classification is stored anywhere in this schema.
- **BCDR/compliance dedicated view.** The "Operational" obligation type
  named in `PRODUCT-SPEC.md` §5.1 (compliance actions, service-ownership
  transitions) is, like "People"/"Delivery"/"Leadership", not a stored
  classification — [ADR-0082](adr.d/0082-repeated-concern-risk-signal.md)/[ADR-0085](adr.d/0085-focus-blocks-people-filter.md)
  already confirmed `obligation_projection` has no `kind`/type column, and
  a follow-up audit (2026-08-19) confirmed no node type resembling
  "service"/"compliance" is actually created anywhere in this codebase
  outside `PRODUCT-SPEC.md`'s own aspirational node-type table (only
  `person`/`meeting`/`risk` are real in practice). Unlike Career/Connect
  export above — which had a genuine technical gap (closed Obligations
  simply weren't queryable per-person) independent of any category — a
  BCDR/compliance *view* has no honest signal to filter on at all today.
  Building it now would mean inventing a classification with no real
  basis, exactly the fabrication this repo's conventions refuse. Blocked
  on the same real decision [ADR-0085](adr.d/0085-focus-blocks-people-filter.md)
  already deferred: a real obligation-category concept, set at
  extraction/promotion time, does not exist yet.

---

## Suggested order

1. **0.1, 0.2, 0.3** — done.
2. **1.3** — done.
3. **1.1** — done.
4. **1.2** — done.
5. **2.1–2.3** — done.
6. **3, 4** — 4's Career/Connect export is done ([ADR-0088](adr.d/0088-career-connect-export.md));
   its BCDR/compliance item is honestly blocked (no stored obligation
   category exists to build a dedicated view on, same gap as 2.2's full
   attention-type taxonomy). All of 3 remains. Largest and most
   speculative; needs its own
   access-control ADR and real usage pressure before committing effort.
   This is a genuine policy decision (what data-classification/access model
   governs a future connector), not a bounded implementation choice, and
   should not be drafted speculatively without the decider's direction.

Every numbered item above still requires its own ADR acceptance before
implementation begins, per [AGENTS.md](../AGENTS.md). This document orders
and justifies the backlog; it does not authorize any of it.
