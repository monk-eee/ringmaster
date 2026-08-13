# EV-0012: Add a minimal HTTP read API and a Node web front end, tested with Playwright

Evidence for [ADR-0012](../adr.d/0012-minimal-http-api-and-node-web-front-end.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0012-minimal-http-api-and-node-web-front-end"

[[check]]
id = "api-route-exists"
invariant = "The Rust backend declares a GET /api/obligations route."
type = "present"
pattern = '"/api/obligations"'
paths = ["backend/src/api.rs"]

[[check]]
id = "axum-dependency-declared"
invariant = "axum is a declared backend dependency."
type = "present"
pattern = '^axum\s*='
paths = ["backend/Cargo.toml"]

[[check]]
id = "frontend-renders-obligations-page"
invariant = "The front end fetches from the backend's obligations API and renders a table."
type = "present"
pattern = '/api/obligations'
paths = ["frontend/src/api.ts"]

[[check]]
id = "playwright-spec-exists"
invariant = "A Playwright spec exercises the rendered Obligations page."
type = "present"
pattern = "@playwright/test"
paths = ["frontend/tests/**"]

[[check]]
id = "compose-defines-frontend"
invariant = "The Compose stack defines a frontend service."
type = "present"
pattern = '^\s*frontend:'
paths = ["compose.yaml"]
```

## Notes

All five checks are automated against the actual API module, Cargo manifest,
front-end source, Playwright spec, and Compose file. None asserts specific
obligation content, since the shared development Postgres volume accumulates
rows across sessions and agents; the Playwright spec itself asserts DOM
structure only, for the same reason.

`frontend-renders-obligations-page` points at `frontend/src/**`, not
`frontend/server.mjs`: [ADR-0014](../adr.d/0014-react-vite-single-page-app.md)
superseded ADR-0012's server-rendered implementation and removed that file.
ADR-0012's own text is unchanged and still holds; only this evidence pointer
follows the guarantee to where it now lives, the same way ADR-0005's evidence
was repointed when ADR-0007 renamed its underlying files.
