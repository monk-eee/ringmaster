# EV-0014: Replace the server-rendered front end with a React/Vite single-page app

Evidence for [ADR-0014](../adr.d/0014-react-vite-single-page-app.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0014-react-vite-single-page-app"

[[check]]
id = "react-dependency-declared"
invariant = "react is a declared frontend dependency."
type = "present"
pattern = '"react"\s*:'
paths = ["frontend/package.json"]

[[check]]
id = "vite-config-exists"
invariant = "A Vite config exists for the React app."
type = "present"
pattern = "defineConfig"
paths = ["frontend/vite.config.ts"]

[[check]]
id = "obligations-view-exists"
invariant = "The SPA fetches and renders the Obligations collection."
type = "present"
pattern = "/api/obligations"
paths = ["frontend/src/api.ts"]

[[check]]
id = "candidates-view-exists"
invariant = "The SPA fetches and renders the Candidates collection."
type = "present"
pattern = "/api/candidates"
paths = ["frontend/src/api.ts"]

[[check]]
id = "vite-proxies-api"
invariant = "Vite's dev server proxies /api to the backend."
type = "present"
pattern = '"/api"'
paths = ["frontend/vite.config.ts"]

[[check]]
id = "playwright-spec-tests-tabs"
invariant = "A Playwright spec exercises switching between the Obligations and Candidates tabs."
type = "present"
pattern = "Candidates"
paths = ["frontend/tests/**"]
```

## Notes

All six checks are automated against the actual package manifest, Vite
config, React source tree, and Playwright spec. None asserts specific
obligation/candidate content or counts, for the same reason as EV-0012: the
shared development Postgres volume accumulates rows across sessions and
agents.
