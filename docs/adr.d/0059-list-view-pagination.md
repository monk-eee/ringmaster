# ADR-0059: List-view pagination for Obligations, Candidates, and People

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Depends on:** [ADR-0025](0025-node-edge-write-api-and-traversal.md), [ADR-0051](0051-relationship-workspace.md)
- **Tags:** frontend, backend, performance, product

## Context

`docs/current-status.md`'s audit has now twice named the same gap: Today's
sections are all capped (Focus Blocks at 3, "What am I forgetting?" at 5,
the ranked list at 10, [ADR-0050](0050-today-attention-budget.md)/[ADR-0053](0053-what-am-i-forgetting.md)),
but the three full list views — Obligations tab, Candidates/Inbox tab, and
People — still fetch and render every matching row with no limit:

- `GET /api/obligations` (`list_obligations`, [backend/src/api.rs](../../backend/src/api.rs))
  has no `LIMIT`, `ORDER BY op.updated_at DESC` over the entire table.
- `GET /api/candidates` (`list_candidates`) is the same shape.
- `GET /api/nodes` (`list_nodes_route`, [ADR-0025](0025-node-edge-write-api-and-traversal.md)/[ADR-0051](0051-relationship-workspace.md))
  has no `LIMIT` either, `?needs_attention=true` narrows *which* rows
  match but not *how many* come back.

This was invisible while the dev database held a handful of rows and
returned to invisibility once [ADR-0056](0056-local-test-database-isolation-and-dev-data-cleanup.md)/[ADR-0057](0057-enforce-test-database-isolation-with-a-runtime-guard.md)
emptied it back out — but the underlying routes are unchanged, and nothing
stops the same unbounded growth (the ~2,025/1,007/1,008 the first audit
found) from recurring and making these three views slow to fetch and
overwhelming to scroll again, independent of the dev-data-hygiene work.

## Decision

Add optional, additive `?limit=`/`?offset=` query parameters to all three
routes, applied as real SQL `LIMIT`/`OFFSET` (not fetch-everything-then-
slice), and have the frontend request bounded pages with an explicit
"Load more" affordance — the same "cap, then an honest way to see more"
philosophy Today's sections already established, applied here via the
network layer instead of a client-side array slice, since the point is to
stop over-fetching, not just to stop over-rendering.

### Backend: `limit`/`offset` on all three list routes

- `GET /api/obligations`, `GET /api/candidates`, and `GET /api/nodes` each
  gain optional `limit`/`offset` query params. Omitting both preserves
  each route's exact current behavior — every matching row, no
  truncation — matching the precedent already set when `occurred_from`/
  `occurred_to`/`needs_attention` were added to `GET /api/nodes` ([ADR-0042](0042-occurred-at-retrieval-and-recall-sources-mcp-tool.md)/[ADR-0051](0051-relationship-workspace.md)).
  Existing callers (tests, the MCP server, any script) that never pass
  these params see zero behavior change.
- When `limit` is present, the query applies `LIMIT`/`OFFSET` after the
  existing `ORDER BY`, so paging is stable across pages for an unchanging
  dataset. `limit` is clamped to a sane maximum server-side (matching the
  clamp-don't-reject precedent in [ADR-0049](0049-audit-trail-read-api.md)'s
  `limit-is-clamped-not-rejected`), `offset` defaults to `0`.
- No response-envelope change — each route keeps returning a bare JSON
  array. "Is there more" is inferred the cheap way: the frontend requests
  `limit` rows and treats a full page (exactly `limit` rows returned) as
  "there may be more, offer Load more"; a short page means "this is
  everything." This trades one rare, harmless false-positive (an empty
  extra page when the true count is an exact multiple of `limit`) for
  never needing a separate `COUNT(*)` query.

### Frontend: a default page size plus "Load more"

- Obligations, Candidates/Inbox, and People (both the needs-attention and
  "show everyone" modes) each request `limit=50` by default and keep an
  offset in component state. A "Load more" button, shown only when the
  last page came back full, appends the next page to the rendered list.
- No new component: this is the existing fetch-and-render logic in each
  view gaining a page-size argument and a button, the same shape as the
  existing `needs_attention` toggle already added to People ([ADR-0051](0051-relationship-workspace.md)).

## Scope

**In scope:** `limit`/`offset` query params on `GET /api/obligations`,
`GET /api/candidates`, `GET /api/nodes`; a default page size and "Load
more" affordance in the Obligations tab, Candidates/Inbox tab, and People.

**Out of scope, named honestly:**

- **An exact total count or "page N of M" display.** "Load more" needs no
  count; a separate `COUNT(*)` query is unnecessary machinery for the
  problem named here.
- **Cursor/keyset pagination.** Offset paging can skip or repeat a row if
  the underlying set changes between page loads. Acceptable, named
  trade-off for a single-user internal tool ([ADR-0004](0004-defer-multi-user-access-control-single-user-v1.md)'s
  precedent for deferring robustness the actual v1 shape doesn't need).
- **Search/filter within these list views** beyond the existing
  `needs_attention` toggle — an unrelated, separate concern.
- **Today's sections** (Focus Blocks, "What am I forgetting?", the ranked
  list) — already capped client-side over an intentionally small,
  pre-filtered fetch ([ADR-0050](0050-today-attention-budget.md)/[ADR-0053](0053-what-am-i-forgetting.md)); this ADR does not touch them.

## Options considered

- **Additive `limit`/`offset`, SQL-level, "Load more" (chosen):** small,
  backward-compatible, fixes the actual over-fetch cost, matches this
  codebase's established pattern for adding optional query params.
- **Client-side-only cap (fetch everything, slice in the frontend):**
  matches Today's existing pattern exactly, but does nothing about the
  named problem — the network fetch and the JSON parse still cover every
  row; rejected because the audit specifically flagged fetching, not just
  rendering.
- **Cursor/keyset pagination:** more robust under concurrent writes, but
  materially more complex (opaque cursor encoding, per-route sort-key
  plumbing) than a single-user tool currently needs; deferred.
- **A generic paginated-response envelope (`{items, total, has_more}`)
  applied everywhere:** would give exact counts, but changes response
  shape for every existing consumer of these three routes (tests, MCP
  tools) for a benefit not named in the gap this ADR addresses; rejected
  as more invasive than the problem warrants.

## Consequences

- **Positive:** these three list views no longer transfer or render an
  unbounded row count; the fix holds even if the dev database (or a real
  future deployment) grows large again, unlike a client-side-only cap.
- **Positive:** fully backward compatible — no existing caller of these
  three routes changes behavior unless it opts in to the new params.
- **Negative / trade-off:** offset paging can show a duplicate or skip a
  row across two "Load more" clicks if rows are inserted or deleted
  between them — named and accepted above, not hidden.
- **Risk:** low — additive query params and a button; no schema change,
  no change to any existing route's default response.

## Exit criteria and evidence

Evidence: [EV-0059](../evidence.d/0059-list-view-pagination.md)

| Exit criterion | Evidence |
|---|---|
| `GET /api/obligations` accepts `limit`/`offset` and applies them in SQL | `obligations-route-accepts-limit-and-offset` |
| `GET /api/candidates` accepts `limit`/`offset` and applies them in SQL | `candidates-route-accepts-limit-and-offset` |
| `GET /api/nodes` accepts `limit`/`offset`, omitting them preserves current behavior | `nodes-route-limit-offset-is-additive` |
| Obligations, Candidates/Inbox, and People each offer a "Load more" affordance | `list-views-offer-load-more` |
| A Playwright test proves "Load more" appends a further page | `playwright-proves-load-more` |
