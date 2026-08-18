# ADR-0087: Graph Explorer create-node reliability under concurrent Playwright load

- **Status:** Accepted
- **Date:** 2026-08-19
- **Decider:** monk-eee
- **Approval:** Continuing this session's established autonomous-work practice ("keep working" / "work autonomously and make good decisions" when unavailable), 2026-08-19
- **Depends on:** [ADR-0026](0026-graph-explorer-frontend.md), [ADR-0033](0033-progressive-graph-traversal-trail.md), [ADR-0073](0073-isolate-playwright-from-dev-database.md), [ADR-0081](0081-graph-explorer-actions-lens.md)
- **Tags:** frontend, testing, reliability

## Context

Two Graph Explorer Playwright tests — `graph trail: traversing two edges
and returning to the root` ([ADR-0033](0033-progressive-graph-traversal-trail.md))
and `graph explorer: the Actions lens filters neighbours to what needs
doing` ([ADR-0081](0081-graph-explorer-actions-lens.md)) — have failed
intermittently across this session, always the same way: their shared
`createNode()` test helper times out after 5000ms waiting for
`.node-detail h3` to populate after clicking "Create node". This was
documented as pre-existing, unrelated flakiness in
[EV-0085](../evidence.d/0085-focus-blocks-people-filter.md) and
[EV-0086](../evidence.d/0086-workbench-three-pane-view.md) and repeatedly
deferred as out of scope.

This record root-causes it instead of deferring it again, because it
reproduces deterministically enough to fix:

1. **`GraphExplorer.tsx`'s `handleCreate` unnecessarily serializes two
   independent effects.** After creating a node it `await`s the full,
   unbounded node-list refresh (`GET /api/nodes` — genuinely unbounded by
   design, per `backend/src/graph/node.rs`'s `list_nodes` docs: "`limit`/
   `offset` of `None` fetch every matching row unchanged (ADR-0059)")
   *before* fetching the just-created node's own detail
   (`GET /api/nodes/:id`, a single-row lookup). The detail pane the test
   waits on has no reason to wait behind the slower, ever-growing list
   fetch — the two calls write to disjoint state (`nodes` vs.
   `trail`/`detail`).
2. **The shared Playwright backend/frontend pair
   ([ADR-0073](0073-isolate-playwright-from-dev-database.md)) is hit by
   every worker at once.** `playwright.config.ts` sets `fullyParallel:
   true` with no explicit `workers` cap, so Playwright defaults to one
   worker per CPU core, all driving separate Chromium instances against
   the *same* single backend/Vite pair. Reproduced directly: running just
   these two tests with the default worker count (6 on this machine)
   failed 6/6 times, each timing out around 8.6–9.3s wall time on the
   very first `createNode()` call; running the identical two tests with
   `--workers=1` passed 2/2 times (10.6s and 8.3s total, no timeout hit).
   This confirms genuine concurrent-worker contention — not accumulated
   `ringmaster_test` row count in isolation — is the dominant cause: six
   simultaneous browser-driven round trips against one shared dev-mode
   Rust/Vite pair on shared local hardware routinely exceed a 5000ms
   assertion budget, even though any single flow completes in well under
   that budget alone.

Neither cause is a product bug in the Graph Explorer feature itself —
manual verification in every prior pass confirmed create-then-select
works correctly. Both causes are fixable without touching product
behavior: remove the unnecessary serialization, and give the assertion
budget enough headroom to tolerate legitimate (not buggy) concurrent load
on shared hardware.

## Decision

- **`GraphExplorer.tsx`'s `handleCreate`** now runs the node-list refresh
  and the new node's own root-selection (which fetches its detail)
  concurrently via `Promise.all`, instead of sequentially `await`ing the
  list refresh first. Final state is identical either way; only the
  ordering/latency changes.
- **`playwright.config.ts`** gains an explicit `expect: { timeout: 10_000
  }`, up from Playwright's implicit 5000ms default, sized to comfortably
  cover the worst observed full-concurrency round trip (~9.3s) with
  headroom, applied uniformly (no per-test special-casing that would mask
  a future genuine regression elsewhere).
- No change to `list_nodes`'s unbounded-by-design semantics
  ([ADR-0059](0059-list-view-pagination.md)) — pagination there is a
  separate, already-decided question this record does not reopen.

## Scope

**In scope:** `GraphExplorer.tsx`'s `handleCreate` ordering; the global
Playwright `expect.timeout` default.

**Out of scope, named honestly:** reducing Playwright's worker count
(would slow the whole suite down as a broad trade-off, not attempted
here); paginating or capping `GET /api/nodes` (a separate, already-settled
decision); cleaning up accumulated `ringmaster_test` row growth across
sessions (a data-hygiene question, not this record's concern); any other
Graph Explorer behavior.

## Options considered

- **Fix the real serialization bug and add reasonable timeout headroom
  (chosen):** addresses both contributing causes without changing any
  product decision or masking a real regression class (a 10s budget still
  fails fast on an actually-broken feature).
- **Just raise the timeout further, ignore the serialization bug:**
  simpler, but leaves a genuine (if minor) unnecessary latency source in
  the create-node flow and risks needing another bump later as the shared
  test database keeps growing.
- **Cap Playwright's worker count instead:** would reduce contention at
  the cost of slowing every test run, a broader trade-off than this
  narrowly-scoped reliability fix needs to make.
- **Do nothing, keep documenting it as known flakiness (rejected):**
  matches this session's prior passes, but the failure reproduced
  deterministically enough (6/6 vs. 2/2) that continuing to defer it is
  no longer honest once the cause is understood.

## Consequences

- **Positive:** removes a genuine, reproducible source of CI/local test
  flakiness without touching any accepted product decision; the two
  previously-flaky tests should now pass reliably even under full-suite
  concurrency.
- **Negative / trade-off:** a 10s default `expect` timeout means a truly
  broken assertion now takes up to 2x as long to fail as before, slightly
  slowing feedback on genuine regressions.
- **Risk:** if worker count or database growth increases further, the same
  timeout could eventually need re-tuning; this is a tuned constant, not a
  guarantee.

## Exit criteria (evidence-checkable)

| Invariant | Evidence check id |
|---|---|
| `GraphExplorer.tsx`'s `handleCreate` runs the list refresh and root-node selection concurrently (`Promise.all`) rather than sequentially | `handle-create-runs-concurrently` |
| `playwright.config.ts` sets an explicit `expect.timeout` of at least 8000ms | `playwright-expect-timeout-raised` |
| The two previously-flaky Graph Explorer tests pass under the default (multi-worker) Playwright configuration | `graph-explorer-tests-pass-under-concurrency` |
