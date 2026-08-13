# ADR-0012: Add a minimal HTTP read API and a Node web front end, tested with Playwright

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Direct instruction ("accept the adrs build the front end - the
  product is useless without it - dont forget playwright tests"), 2026-08-14
- **Depends on:** [ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md), [ADR-0006](0006-local-development-stack-runs-via-podman-compose.md)
- **Tags:** architecture, api, frontend, testing

## Context

[docs/VISION.md](../VISION.md) and the repository uplift report name "Rust
core with a Node front end" as the intended toolchain, but no HTTP surface
or front end has existed until now: the Rust backend only migrates, rebuilds
the Obligation projection once, and idles
([ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md)).
[docs/PRODUCT-SPEC.md § 16](../PRODUCT-SPEC.md#16-mvp-scope-and-epics) Epic
E8 ("Web home") describes a rich attention/risk-horizon view, but that
depends on Epic E7 (attention engine) and Epic E4 (extraction), neither of
which is built. Without any UI at all, nothing built so far is visible or
usable by a person, which defeats the point of a tool a manager is meant to
look at. monk-eee, the sole decider, has now directly instructed building a
front end immediately rather than waiting for the full Epic E8 feature set.

## Decision

- The Rust backend gains a minimal HTTP API (using `axum`, chosen for being
  tokio-native and consistent with the existing `tokio`/`sqlx` stack):
  - `GET /health` returns `200 OK`.
  - `GET /api/obligations` returns the current `obligation_projection` rows
    as JSON (`obligation_id`, `status`, `updated_at`). This is a read-only
    projection read; the API does not write, matching
    [ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md)'s
    rule that projections are derived and never additionally authoritative.
  - `main.rs` now serves this API after migrating and rebuilding the
    projection, replacing the previous placeholder infinite-sleep loop.
- A new top-level `frontend/` Node/Express app renders one server-rendered
  page ("Obligations") listing those same rows, fetched from the backend's
  `/api/obligations` inside the Express route handler (no client-side
  JavaScript, no bundler, no SPA framework yet — there is no interactivity
  requirement to justify that complexity today). All interpolated values are
  HTML-escaped before rendering.
- Playwright (`@playwright/test`, Chromium only) is adopted as the front
  end's test tool. One spec loads the page against a running stack and
  asserts DOM structure (title, table headers, non-empty cells if any rows
  exist) rather than exact row counts or content, since the shared
  development Postgres volume accumulates obligations across sessions and
  agents.
- `compose.yaml` gains a `frontend` service (new `frontend/Dockerfile`),
  depending on `backend`; `backend` now exposes its port to the host so a
  host-run front end or Playwright process can reach it directly. This is an
  additional service within [ADR-0006](0006-local-development-stack-runs-via-podman-compose.md)'s
  already-accepted "at minimum a postgres and backend service" scope, not a
  new infrastructure decision.

## Scope

**In scope:** the two-route read-only HTTP API; the single server-rendered
Obligations page; Playwright as the e2e test tool with one structural spec;
the `frontend` Compose service and backend port exposure.

**Out of scope:** the full Epic E8 attention/risk-horizon UI (needs Epic
E7), any write/mutation UI, authentication or session UI (single operator
already fixed by [ADR-0004](0004-defer-multi-user-access-control-single-user-v1.md)),
any client-side framework, bundler, or build pipeline, and Ringmaster's own
outward-facing MCP server (Epic E9).

## Options considered

- **axum for the API, plain server-rendered HTML for the front end (chosen):**
  smallest surface that makes the product genuinely visible end-to-end today,
  with no new architectural commitments (client framework, auth, styling)
  the product doesn't yet need.
- **A full SPA (React/Vite) front end:** matches the vision's eventual
  richness, but there is no interactive behavior yet to justify a build
  pipeline; premature given Epics E4/E6/E7 (the data a rich UI would need)
  remain unbuilt.
- **actix-web instead of axum:** a credible alternative, but axum's tighter
  fit with the already-adopted `tokio` runtime avoids introducing a second
  async runtime model.
- **Skip Playwright, test the front end with unit tests only:** cheaper, but
  would not exercise a real browser rendering real HTML from a real HTTP
  response chain, which is the actual risk this ADR is closing.

## Consequences

- **Positive:** for the first time, a person can open a browser and see real
  Obligation data flow end-to-end from Postgres through the Rust backend to
  a rendered page; Playwright proves that chain works, not just that Rust
  functions compile.
- **Negative / trade-off:** introduces a second language/runtime (Node)
  operationally, and `axum` as a new, previously unused Rust dependency.
- **Risk:** the page currently shows only `obligation_id`/`status`, since no
  extraction feature yet attaches human-readable titles to obligations.
  Mitigated by treating this as an honest reflection of current backend
  capability, not a gap in this ADR's own scope.

## Exit criteria and evidence

Evidence: [EV-0012](../evidence.d/0012-minimal-http-api-and-node-web-front-end.md)

| Exit criterion | Evidence |
|---|---|
| The Rust backend exposes a read-only `/api/obligations` HTTP route | `api-route-exists`, `axum-dependency-declared` |
| The front end renders an Obligations page backed by that route | `frontend-renders-obligations-page` |
| A Playwright spec exercises the rendered page | `playwright-spec-exists` |
| The Compose stack defines a frontend service | `compose-defines-frontend` |
