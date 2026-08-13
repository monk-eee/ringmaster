# ADR-0014: Replace the server-rendered front end with a React/Vite single-page app

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Direct instruction ("ringmaster should be a real interactive
  SPA"), 2026-08-14
- **Amends:** [ADR-0012](0012-minimal-http-api-and-node-web-front-end.md) (supersedes its server-rendered-only decision; the HTTP API routes it and [ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md) added are unchanged and reused as-is)
- **Depends on:** [ADR-0012](0012-minimal-http-api-and-node-web-front-end.md), [ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md)
- **Tags:** architecture, frontend, spa, testing

## Context

[ADR-0012](0012-minimal-http-api-and-node-web-front-end.md) deliberately
chose a server-rendered, no-client-JS page, and named "a full SPA
(React/Vite) front end" as a rejected alternative: "there is no interactive
behavior yet to justify a build pipeline." Since then,
[ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md)
added `/api/candidates` and an extraction trigger, so the backend now serves
two real, distinct collections (Obligations, Candidates) instead of one.
monk-eee, the sole decider, has now directly instructed that the front end
itself should be "a real interactive SPA," reversing ADR-0012's premise.

## Decision

- The front end is rebuilt as a React SPA using Vite (the same pairing
  ADR-0012 already named as *the* SPA alternative, kept for continuity
  rather than introducing a third, undiscussed option). Vite's own dev
  server (`server.host: true`, port 3000) is what `compose.yaml`'s
  `frontend` service runs; per
  [ADR-0006](0006-local-development-stack-runs-via-podman-compose.md) this
  whole stack is local development tooling, not a deployment artifact, so a
  dev server here is consistent with, not a departure from, that scope.
- Vite's dev server proxies `/api/*` to the Rust backend
  (`BACKEND_URL`, read server-side at Vite config load time — never exposed
  to client-side code), so the browser only ever talks to one same-origin
  host and no CORS configuration is needed on the backend.
- The app renders two tabs backed by the two existing read routes:
  **Obligations** ([ADR-0012](0012-minimal-http-api-and-node-web-front-end.md)'s
  `/api/obligations`) with a client-side status filter and sort, and
  **Candidates** ([ADR-0013](0013-http-endpoints-trigger-and-list-extraction-candidates.md)'s
  `/api/candidates`). A manual refresh button re-fetches both without a page
  reload. All filtering/sorting happens client-side against already-fetched
  data; no new backend query parameters are added.
- `backend/src/api.rs`'s routes are unchanged. This ADR governs the
  rendering technology only, not the API surface.
- `frontend/server.mjs` (the ADR-0012 Express server) is removed; Vite's
  dev server now serves `index.html`/`public/*` directly.

## Scope

**In scope:** adopting React + Vite; the Obligations and Candidates tabs
with client-side filter/sort/refresh; the dev-server API proxy; removing
the now-unused Express server; updated Playwright coverage for real
client-side interaction (tab switching, filtering).

**Out of scope:** any UI to trigger extraction (ADR-0013's `POST
.../extract`) — there is no route yet to list source fragments to extract
from, so a text-entry trigger form would be unusably blind; that is future,
ADR-governed work. Also out of scope: a production build/bundle step
(`vite build` for anything beyond local dev), routing beyond the two tabs,
authentication, and the full Epic E8 attention/risk-horizon view.

## Options considered

- **React + Vite dev server (chosen):** matches the alternative ADR-0012
  itself already named and reasoned about; Vite's built-in dev-server proxy
  removes any need for a hand-rolled Express proxy or CORS setup.
- **Keep server-rendering, add small islands of client JS:** cheaper, but
  does not deliver what was asked for ("a real interactive SPA"); would
  still need a bundler for anything beyond trivial inline scripts.
- **A full production build pipeline (`vite build` + static hosting) now:**
  more deployment-realistic, but this stack is explicitly local-dev-only
  ([ADR-0006](0006-local-development-stack-runs-via-podman-compose.md));
  premature until an actual deployment target is decided.

## Consequences

- **Positive:** the front end is now genuinely interactive (tab switching,
  filtering, sorting, manual refresh) against real backend data, matching
  what was asked for; Vite's proxy keeps the API same-origin with no new
  backend CORS work.
- **Negative / trade-off:** a second build toolchain (Vite/React/JSX) is now
  part of local dev; `frontend/server.mjs` and its own tests/behavior are
  retired rather than extended.
- **Risk:** `npm install` inside this repo's containers cannot reach
  `registry.npmjs.org` reliably (recorded in repo memory). Mitigated the
  same way as [ADR-0012](0012-minimal-http-api-and-node-web-front-end.md):
  install on the host, `COPY frontend/node_modules` into the image.

## Exit criteria and evidence

Evidence: [EV-0014](../evidence.d/0014-react-vite-single-page-app.md)

| Exit criterion | Evidence |
|---|---|
| The front end is a React app built with Vite | `react-dependency-declared`, `vite-config-exists` |
| The app renders both an Obligations and a Candidates view | `obligations-view-exists`, `candidates-view-exists` |
| Vite's dev server proxies `/api` to the backend | `vite-proxies-api` |
| A Playwright spec exercises real client-side interaction (tab switching) | `playwright-spec-tests-tabs` |
