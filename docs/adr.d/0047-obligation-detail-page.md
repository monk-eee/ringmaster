# ADR-0047: Obligation detail page — a first-class read view over existing data

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Direct instruction ("yes"), confirming the gap-check's proposal to scope this ADR, 2026-08-17
- **Depends on:** [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md), [ADR-0022](0022-daily-brief-endpoint.md), [ADR-0023](0023-evidence-backed-daily-brief-reasons.md), [ADR-0025](0025-node-edge-write-api-and-traversal.md), [ADR-0039](0039-product-re-steer-primary-navigation.md), [ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md), [ADR-0044](0044-today-attention-items-management-meaning.md), [ADR-0046](0046-unowned-obligation-risk-signal.md)
- **Tags:** architecture, api, frontend, data-model

## Context

Three separate, already-accepted ADRs each named the same missing piece as
their reason for deferring something:

- [ADR-0044](0044-today-attention-items-management-meaning.md): *"A real
  primary action... 'review' needs a first-class obligation-detail surface
  (none exists yet)."*
- The same ADR, on snooze/dismiss: needs *"a persisted state and route
  that do not exist"* — which in turn needs somewhere to put those
  controls.
- Correcting an Obligation's owner or date would need the same detail
  surface to host the control, mirroring
  [ADR-0045](0045-correct-candidate-before-accepting.md)'s Candidate
  correction form.

No route reads one Obligation today — `GET /api/obligations` only lists
all of them. Every field this page needs already exists and is already
returned by `GET /api/daily-brief`/`GET /api/time-horizon`
(`status`, `updated_at`, `hard_due_at`, `soft_due_at`, `source_fragment_id`,
`source_text`, `risk_signals`); the one genuinely new capability is reading
a single Obligation by id, plus its linked graph context (owner, related
meeting, etc.) via the same polymorphic `edges` table
[ADR-0028](0028-person-relationship-view.md) already reads in the other
direction (Person → Obligation). This ADR is deliberately read-only,
matching this repository's established sequencing (a read surface before
any write/action surface — [ADR-0025](0025-node-edge-write-api-and-traversal.md)
before [ADR-0026](0026-graph-explorer-frontend.md);
[ADR-0036](0036-meeting-detail-read.md) before
[ADR-0043](0043-meeting-review-page.md)). Snooze/dismiss, correcting
owner/date, and a "What changed" audit view remain named, separate,
future decisions — this ADR gives them a page to eventually live on, it
does not build them.

## Decision

- `GET /api/obligations/:id` reads one `obligation_projection` row and
  returns the same shape `GET /api/daily-brief` already returns per item
  (`status`, `updated_at`, `hard_due_at`, `soft_due_at`,
  `source_fragment_id`, `source_text`, `risk_signals` — computed by the
  exact existing `risk_signals()` function and `has_owner` subquery,
  zero new reasoning), plus a new `linked_nodes` array: every edge whose
  `from_id` or `to_id` equals this Obligation's id, with the *other* end
  resolved against `nodes` (mirroring `GET /api/nodes/:id`'s neighbor
  query, [ADR-0025](0025-node-edge-write-api-and-traversal.md), just
  anchored on an Obligation id instead of a node id). An edge whose other
  end isn't a `nodes` row reports a `null` neighbor, the same honest
  fallback ADR-0025 already established. `404` for an unknown id.
- **Frontend:** a new `ObligationDetail` component renders the title
  (evidence quote or honest status label,
  [ADR-0044](0044-today-attention-items-management-meaning.md)'s
  `itemTitle`), status badge, due-date phrase (`duePhrase`, same ADR),
  evidence status line, the `risk_signals` list (same `.risk-signals`
  presentation Today/Timeline already use), and a `linked_nodes` list
  (icon + type + canonical text + edge type, matching the existing
  Graph Explorer/People neighbor presentation). `itemTitle`/`duePhrase`
  are exported from `DailyBrief.tsx` instead of duplicated.
- **Entry points:** a Today row and an Obligations-table row are each a
  button that opens this same detail view in place of the tab's current
  content, with a "Back" control returning to the prior list — no new
  tab, no client-side router, matching the List+Detail-in-one-surface
  pattern `GraphExplorer`/`People`/`MeetingReview` already use.

## Scope

**In scope:** `GET /api/obligations/:id` (fields, risk signals,
linked-nodes, 404); the `ObligationDetail` component; exporting
`itemTitle`/`duePhrase`; making Today and Obligations-table rows
selectable into this view.

**Out of scope, named honestly (real, separate future decisions):**

- **Snooze/dismiss controls.** Obligation has no such state or transition
  today; adding one is its own schema/event decision.
- **Correcting an Obligation's owner or date from this page.** No write
  route exists for Obligation fields beyond status transitions
  ([ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md)'s
  event vocabulary); this page reads, it does not add a new mutation.
- **A "What changed" / event-history view on this page.**
  [ADR-0008](0008-add-append-only-audit-events-table.md) explicitly
  deferred an audit read API; this ADR does not build one.
- **Creating a new edge from this page** (e.g., "assign an owner"). The
  Graph Explorer's existing "Add relationship" form
  ([ADR-0026](0026-graph-explorer-frontend.md)) already does this against
  any node/Obligation id; this ADR does not duplicate it here.
- **Deep-linkable URLs** for a specific Obligation (e.g.,
  `/obligations/:id`). This app has no client-side router
  ([ADR-0014](0014-react-vite-single-page-app.md)); selection is
  in-memory component state, the same posture
  [ADR-0033](0033-progressive-graph-traversal-trail.md) already chose for
  the graph traversal trail.

## Options considered

- **A dedicated read route plus a shared detail component (chosen):**
  reuses every existing field, function, and presentation pattern
  verbatim; the only new code is one route and one component wiring
  them together.
- **Reuse `GET /api/nodes/:id` instead of a new Obligation route:** that
  route's contract is a graph node, not an Obligation
  ([ADR-0025](0025-node-edge-write-api-and-traversal.md)); an Obligation
  is not a `nodes` row, so it would need special-casing inside an
  already-generic route rather than its own honestly-named endpoint —
  rejected for the same reasoning
  [ADR-0036](0036-meeting-detail-read.md) used for a dedicated meetings
  route over overloading `/api/nodes/:id`.
- **Build snooze/dismiss/owner-correction in the same ADR:** rejected —
  each needs its own schema/event decision this record does not make;
  bundling them would repeat the exact overreach
  [ADR-0044](0044-today-attention-items-management-meaning.md) already
  declined for the same reason.
- **A dedicated client-side route/URL for Obligation detail:** rejected —
  this app deliberately has no router; in-memory selection state is
  simpler and consistent with three existing surfaces that already work
  this way.

## Consequences

- **Positive:** closes the specific, three-times-named blocker ("no
  obligation-detail surface exists") without guessing at the unrelated
  decisions that still block snooze/dismiss/correction/What-changed.
- **Positive:** zero new backend reasoning — `risk_signals()`, `has_owner`,
  `itemTitle`, and `duePhrase` are reused verbatim from already-accepted,
  already-tested code.
- **Negative / trade-off:** the page is read-only; "Review" now navigates
  somewhere real, but no action beyond looking is possible from it yet.
- **Risk:** low. One new read-only route; one new frontend component;
  no schema change, no new dependency.

## Exit criteria and evidence

Evidence: [EV-0047](../evidence.d/0047-obligation-detail-page.md)

| Exit criterion | Evidence |
|---|---|
| `GET /api/obligations/:id` returns the same fields Daily Brief returns, plus risk_signals | `obligation-detail-route-returns-risk-signals` |
| The route resolves linked edges against `nodes`, honestly reporting null for an unresolvable neighbor | `obligation-detail-route-resolves-linked-nodes` |
| The route 404s for an unknown id | `obligation-detail-route-404s-for-unknown-id` |
| `itemTitle`/`duePhrase` are exported and reused, not duplicated | `title-and-due-phrase-are-exported-and-reused` |
| Selecting a Today row or an Obligations-table row opens the shared detail view with a Back control | `rows-are-selectable-into-shared-detail-view` |
| Focused browser coverage proves opening an Obligation's detail and returning | `playwright-proves-obligation-detail-flow` |
