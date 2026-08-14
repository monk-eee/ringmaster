# EV-0021: Ratify surfacing semantic search in the frontend SPA (retroactive)

Evidence for [ADR-0021](../adr.d/0021-ratify-search-tab-surfaced-without-its-own-adr.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0021-ratify-search-tab-surfaced-without-its-own-adr"

[[check]]
id = "search-results-component-exists"
invariant = "The Search tab's presentational component exists as shipped."
type = "present"
pattern = 'function SearchResults'
paths = ["frontend/src/components/SearchResults.tsx"]

[[check]]
id = "no-new-frontend-dependency"
invariant = "The Search tab introduced no new dependency, confirmed by diffing the shipping commit against package.json/package-lock.json."
type = "manual"
last_verified = "2026-08-14"
```

## Notes

`search-results-component-exists` is automated against the actual
component file. `no-new-frontend-dependency` is `manual`: verified by
`git show <search-ui-commit> --stat -- frontend/package.json
frontend/package-lock.json` returning no changes, which isn't a pattern a
declarative file check can express — a dependency diff needs a specific
commit range, not a static file state.
