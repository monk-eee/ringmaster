# EV-0087: Graph Explorer create-node reliability under concurrent Playwright load

Evidence for [ADR-0087](../adr.d/0087-graph-explorer-reliability-under-concurrent-load.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0087-graph-explorer-reliability-under-concurrent-load"

[[check]]
id = "handle-create-runs-concurrently"
invariant = "GraphExplorer.tsx's handleCreate runs the list refresh and root-node selection concurrently (Promise.all) rather than sequentially."
type = "present"
pattern = 'Promise\.all\(\[loadNodes\(\), selectRootNode\(node\)\]\)'
paths = ["frontend/src/components/GraphExplorer.tsx"]

[[check]]
id = "playwright-expect-timeout-raised"
invariant = "playwright.config.ts sets an explicit expect.timeout of at least 8000ms."
type = "present"
pattern = 'timeout:\s*10_000'
paths = ["frontend/playwright.config.ts"]

[[check]]
id = "graph-explorer-tests-pass-under-concurrency"
invariant = "The two previously-flaky Graph Explorer tests pass under the default (multi-worker) Playwright configuration."
type = "manual"
last_verified = "2026-08-19"
rationale = "Before the fix: running just these two tests with the default worker count (6, matching this machine's CPU count) via `npx playwright test -g \"graph trail: traversing two edges|Actions lens filters neighbours\" --repeat-each=3` failed 6/6, each timing out at 5000ms around 8.6-9.3s wall time on the first createNode() call. Running the identical two tests with --workers=1 passed 2/2 (10.6s, 8.3s), confirming concurrent-worker contention against the one shared Playwright backend/Vite pair (ADR-0073) as the dominant cause, not accumulated database size alone. After applying both fixes (Promise.all in handleCreate, expect.timeout raised to 10s): the same --repeat-each=3 run under default 6-worker concurrency passed 6/6 (17.9-18.7s each). A subsequent full-suite run (`npx playwright test --project=chromium`, 8 workers, 28 tests) passed 23/23 non-skipped tests including both of these, 0 failures. This is a manual check because it is a flakiness-elimination claim under real concurrent load, not a single literal a regex could honestly stand in for; re-verify if worker count, machine load, or ringmaster_test's accumulated row count changes materially enough to warrant re-tuning the timeout."
```

## Notes

Root-caused, not just re-documented: `GraphExplorer.tsx`'s `handleCreate`
unnecessarily serialized an unbounded `GET /api/nodes` list refresh ahead of
the just-created node's own single-row detail fetch, and Playwright's
default 5000ms `expect` timeout left no headroom for genuine (not buggy)
contention when every CPU-core worker drives a separate Chromium instance
against the one shared dev-mode backend/Vite pair ADR-0073 intentionally
uses. Both fixes are additive/config-only — no product behavior, route, or
decision changed. Verified via a before/after `--workers=1` vs. default-worker
comparison (see `graph-explorer-tests-pass-under-concurrency` above) and a
full-suite run. This replaces the "known, pre-existing, out of scope"
caveat carried in [EV-0085](0085-focus-blocks-people-filter.md) and
[EV-0086](0086-workbench-three-pane-view.md) with an actual fix.
