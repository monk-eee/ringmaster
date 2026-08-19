# EV-0089: Patch high-severity Vite/nanoid advisories

Evidence for [ADR-0089](../adr.d/0089-patch-vite-nanoid-security-advisories.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0089-patch-vite-nanoid-security-advisories"

[[check]]
id = "vite-version-patched"
invariant = "frontend/package.json pins vite to ^6.4.3 or later."
type = "present"
pattern = '"vite":\s*"\^6\.4\.3"'
paths = ["frontend/package.json"]

[[check]]
id = "no-known-vulnerabilities"
invariant = "npm audit reports zero vulnerabilities in frontend/."
type = "manual"
last_verified = "2026-08-19"
rationale = "`npm audit` in frontend/ reported 2 high-severity advisories before this change (Vite <=6.4.2's path-traversal/NTLMv2-hash-disclosure/fs.deny-bypass trio, GHSA-4w7w-66w2-5vf9/GHSA-v6wh-96g9-6wx3/GHSA-fx2h-pf6j-xcff; nanoid <3.3.18's infinite-loop-on-zero-size, GHSA-2v37-7h3g-55p8, a transitive Vite dependency). After bumping vite to ^6.4.3 (npm install vite@6.4.3) and running a plain npm audit fix (no --force, no major-version jump) for the remaining transitive nanoid advisory, `npm audit` reports 'found 0 vulnerabilities'. This is a manual check because it reflects an external tool's live advisory database at a point in time, not a literal a regex could honestly stand in for; re-run npm audit whenever dependencies change or periodically to catch newly-disclosed advisories."
```

## Notes

`frontend/Dockerfile`'s `CMD ["npx", "vite"]` means the actual running
frontend container is Vite's dev server, not a static build served by a
production HTTP server — these advisories were live in the running
application, not just a local dev-tool concern. Checked available Vite
6.x releases before accepting `npm audit fix --force`'s default choice of
`vite@8.2.1` (a two-major-version jump); `vite@6.4.3` (the latest 6.x
patch) already resolves all three Vite advisories, verified directly by
installing it and re-running `npm audit` (only the transitive `nanoid`
advisory remained, then cleared by a plain `npm audit fix`). Verified:
`npx tsc --noEmit` and `npm run build` both clean under Vite 6.4.3; the
full Playwright suite (25 passed, 5 pre-existing skips, 0 failed)
including the two ADR-0088 Career export tests, run against the actual
`npx vite` dev server this container uses, all pass unchanged.
