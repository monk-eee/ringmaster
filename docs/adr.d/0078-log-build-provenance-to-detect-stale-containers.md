# ADR-0078: Log build provenance so stale containers are visible on startup

- **Status:** Accepted
- **Date:** 2026-08-18
- **Decider:** monk-eee
- **Approval:** Direct instruction ("Work autonomously and make good decisions") selecting this item from `docs/IMPROVEMENT-PLAN.md`'s Priority 0.2, 2026-08-18
- **Depends on:** [ADR-0006](0006-local-development-stack-runs-via-podman-compose.md)
- **Tags:** infrastructure, developer-experience, observability

## Context

`docs/current-status.md` documents a real incident: the running
`ringmaster-backend-1`/`ringmaster-frontend-1` containers kept serving an
image built before the latest commit landed. Today briefly rendered "120
things need your attention" with mostly generic, evidence-less text — it
looked like a capping regression but wasn't one. The only way to confirm
the real cause was `podman inspect --format '{{.Created}}'` on the image,
compared by hand against the commit timestamp. That comparison is a
mechanical, cheap check; nothing today does it automatically, so a person
has to remember to run it and interpret it correctly every time the live
app disagrees with what the source says it should do.

## Decision

- `backend/build.rs` captures the short commit SHA (`git rev-parse
  --short=12 HEAD`) and that commit's ISO 8601 timestamp (`git log -1
  --format=%cI`) at compile time, embedding both as compile-time
  constants via `cargo:rustc-env`. If `git` is unavailable or the command
  fails (for example, a build context with no `.git` directory), both
  fall back to the literal string `"unknown"` rather than failing the
  build — build provenance is a diagnostic aid, never a build
  requirement.
- The backend's existing startup log line gains this provenance:
  `ringmaster-backend: built from <sha> (<commit timestamp>)`, printed
  once at startup, so it is visible in `podman compose logs backend` (or
  `docker compose logs backend`) without any extra command.
- `frontend/vite.config.ts` computes the same short SHA and commit
  timestamp at config-load time (which runs every time the dev server
  starts) using the same fallback-to-`"unknown"` posture, and logs it
  once via a small Vite plugin's `configureServer` hook, visible in
  `podman compose logs frontend`.
- `.dockerignore` stops excluding `.git` so both Dockerfiles' build
  contexts can read it; `frontend/Dockerfile` gains a `COPY .git ./.git`
  step (read-only metadata, not a runtime dependency) alongside its
  existing copies. This is accepted as a reasonable trade-off for a
  stack explicitly scoped as "local dev only, not a deployment artifact"
  (ADR-0006), which already copies the full repository context.
- Detection remains manual-but-cheap: a person compares the logged SHA
  against `git log` on the host. No script parses container logs or
  polls for drift automatically.

## Scope

**In scope:** `backend/build.rs` (new), the startup log line in
`backend/src/main.rs`, `frontend/vite.config.ts`'s startup log,
`.dockerignore`'s `.git` exclusion, and `frontend/Dockerfile`'s copy of
`.git`.

**Out of scope, named honestly:** an automated healthcheck or CI job that
diffs the logged SHA against the current commit and fails/alerts on
mismatch (the plan named this as an alternative; a log line is the
cheaper of the two and is what this ADR implements); any change to
`compose.yaml`'s existing `postgres` healthcheck; any change to what
`podman compose build` actually rebuilds (this ADR makes staleness
*visible*, it does not prevent or auto-correct it).

## Options considered

- **Startup log line comparing build provenance to what's logged
  (chosen):** the cheaper of the two options named in the improvement
  plan; requires no new tooling, script, or CI job, and surfaces in the
  exact place (`podman compose logs`) a person already looks first.
- **A `podman compose` healthcheck that fails when stale:** rejected for
  now — meaningfully more complex (the healthcheck script itself would
  need the same git-comparison logic, plus a policy for what "unhealthy"
  should do to a already-running container), and the log-line approach
  already solves the concrete, documented pain point (manual `podman
  inspect` comparison) at a fraction of the cost.

## Exit criteria and evidence

| Exit criterion | Evidence |
|---|---|
| The backend logs its build commit SHA and commit timestamp once at startup | `backend-logs-build-provenance` |
| A missing/unavailable git falls back to `"unknown"` rather than failing the build or the run | `build-provenance-degrades-gracefully` |
| The frontend dev server logs the same build provenance once at startup | `frontend-logs-build-provenance` |
| `.git` is available in both Dockerfiles' build contexts | `dockerignore-permits-git-metadata` |
