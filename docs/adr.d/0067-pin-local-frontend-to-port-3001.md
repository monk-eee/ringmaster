# ADR-0067: Pin the local frontend to port 3001

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Direct instruction ("please ensure ringmaster frontedn server stays on 3001"), 2026-08-17
- **Amends:** [ADR-0014](0014-react-vite-single-page-app.md)'s local Vite port from 3000 to 3001.
- **Depends on:** [ADR-0006](0006-local-development-stack-runs-via-podman-compose.md), [ADR-0014](0014-react-vite-single-page-app.md)
- **Tags:** frontend, vite, local-development, configuration

## Context

Ringmaster is currently being used at `http://127.0.0.1:3001`, but that port
comes from a one-off `vite --port 3001` launch argument. The checked-in Vite,
Compose, Playwright, and documentation configuration still names port 3000.
A normal restart would therefore move the frontend back to 3000, and Vite's
default behavior could silently choose another port when the configured port
is occupied.

## Decision

- Ringmaster's local frontend port is 3001 everywhere: Vite, Compose host and
  container mapping, Playwright's base URL and managed web server URL, and
  current setup documentation.
- Vite sets `strictPort: true`. If 3001 is unavailable, startup fails clearly
  instead of silently moving Ringmaster to 3002 or another port.
- The already-running Vite process on 3001 is left untouched while the
  checked-in restart configuration is corrected.

## Scope

**In scope:** local frontend port configuration and current documentation.

**Out of scope:** backend/Postgres ports; production deployment; changing
Vite's API proxy; rewriting historical accepted ADRs that accurately record
the earlier port-3000 decision.

## Options considered

- **Pin 3001 with strict-port behavior (chosen):** preserves the active user
  URL and makes future restarts deterministic.
- **Rely on a manual `--port 3001` argument:** rejected because an ordinary
  `npm run dev` or Compose restart returns to checked-in port 3000.
- **Configure 3001 without `strictPort`:** rejected because Vite can silently
  drift to another port when 3001 is occupied.

## Consequences

- **Positive:** Ringmaster remains at `http://127.0.0.1:3001` across direct
  Vite, Compose, and Playwright-managed launches.
- **Negative / trade-off:** a second process already using 3001 now causes a
  clear startup failure rather than an automatic fallback.
- **Risk:** low. This changes only local-development addressing.

## Exit criteria and evidence

Evidence: [EV-0067](../evidence.d/0067-pin-local-frontend-to-port-3001.md)

| Exit criterion | Evidence |
|---|---|
| Vite is pinned to port 3001 and refuses automatic fallback | `vite-pins-strict-port-3001` |
| Compose and Playwright use port 3001 | `local-launchers-use-port-3001` |
| The active frontend responds on port 3001 | `frontend-live-on-3001` |
