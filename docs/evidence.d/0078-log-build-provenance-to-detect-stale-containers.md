# EV-0078: Log build provenance so stale containers are visible on startup

Evidence for [ADR-0078](../adr.d/0078-log-build-provenance-to-detect-stale-containers.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0078-log-build-provenance-to-detect-stale-containers"

[[check]]
id = "backend-logs-build-provenance"
invariant = "The backend logs its build commit SHA and commit timestamp once at startup."
type = "present"
pattern = 'ringmaster-backend: built from'
paths = ["backend/src/main.rs"]

[[check]]
id = "build-provenance-degrades-gracefully"
invariant = "A missing or unavailable git command falls back to \"unknown\" rather than failing the build."
type = "present"
pattern = 'unwrap_or_else\(\|\| "unknown"\.to_string\(\)\)'
paths = ["backend/build.rs"]

[[check]]
id = "frontend-logs-build-provenance"
invariant = "The frontend dev server logs the same build provenance once at startup."
type = "present"
pattern = "built from"
paths = ["frontend/vite.config.ts"]

[[check]]
id = "dockerignore-permits-git-metadata"
invariant = ".git is available in both Dockerfiles' build contexts."
type = "absent"
pattern = '^\.git$'
paths = [".dockerignore"]
```

## Notes

Implemented: `backend/build.rs` runs `git rev-parse --short=12 HEAD` and
`git log -1 --format=%cI` at compile time, embedding both via
`cargo:rustc-env`; either falls back to `"unknown"` if git is unavailable
or the commands fail, never failing the build. `backend/src/main.rs`'s
startup log gains `ringmaster-backend: built from <sha> (<commit
time>)`. `frontend/vite.config.ts` computes the same values at
config-load time via `execFileSync` (same `"unknown"` fallback) and logs
them once via a `configureServer` Vite plugin hook, visible in
`podman compose logs frontend`/`docker compose logs frontend`.
`.dockerignore` no longer excludes `.git`; `frontend/Dockerfile` gains
`COPY .git ./.git` alongside its existing copies (backend/Dockerfile
already does `COPY . .`, so no change was needed there).

Verified: `cargo build --workspace`, `cargo clippy --all-targets
--all-features -- -D warnings` (zero warnings), `cargo fmt --all --
--check` (clean), the full backend suite via Unit Test MCP against an
isolated `ringmaster_test` database (all passing), `npx tsc --noEmit`,
and `npm run build` all passed. Ran the built binary directly and
confirmed the exact startup log line renders with a real commit SHA and
timestamp (not the `"unknown"` fallback), since `.git` is present on this
development machine.

**Gap found and fixed, 2026-08-19:** the verification above only ran a
host `cargo build`/binary, never the actual `docker compose build`
path this ADR exists to fix. Rebuilding for real showed both containers
logging `built from unknown (unknown)` -- `rust:1-slim` and `node:20-slim`
do not include `git`, so `build.rs`'s and `vite.config.ts`'s `git`
invocations failed and hit the designed fallback every time, silently
defeating the feature in exactly the scenario it targets.
`backend/Dockerfile`'s build stage and `frontend/Dockerfile` now each
`apt-get install` `git` before compiling/running. Re-verified live:
`docker compose build backend frontend && docker compose up -d
--force-recreate backend frontend` then `docker compose logs
backend`/`frontend` both now show `built from 6d15fefc11a5
(2026-08-18T20:24:46+10:00)`, matching `git log -1 --format=%cI HEAD`
exactly. `cargo check`, `cargo clippy -- -D warnings`, `npx tsc --noEmit`,
`npm run build`, and `git diff --check` all still pass.
