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
paths = ["frontend/server.mjs"]

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
front-end server, Playwright spec, and Compose file. None asserts specific
obligation content, since the shared development Postgres volume accumulates
rows across sessions and agents; the Playwright spec itself asserts DOM
structure only, for the same reason.
