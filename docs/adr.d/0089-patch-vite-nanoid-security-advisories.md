# ADR-0089: Patch high-severity Vite/nanoid advisories — the frontend container runs Vite's dev server, not a static build

- **Status:** Accepted
- **Date:** 2026-08-19
- **Decider:** monk-eee
- **Approval:** Continuing this session's established autonomous-work practice ("keep working" / "work autonomously and make good decisions" when unavailable), 2026-08-19
- **Depends on:** [ADR-0014](0014-react-vite-single-page-app.md), [ADR-0078](0078-log-build-provenance-to-detect-stale-containers.md)
- **Tags:** security, infrastructure, dependencies

## Context

`npm audit` on `frontend/` reported 2 high-severity advisories:

- **Vite (`<=6.4.2`):** path traversal in optimized-deps `.map` handling
  (GHSA-4w7w-66w2-5vf9); `launch-editor`'s NTLMv2 hash disclosure via UNC
  path handling on Windows (GHSA-v6wh-96g9-6wx3); `server.fs.deny` bypass
  on Windows alternate paths (GHSA-fx2h-pf6j-xcff).
- **`nanoid` (`<3.3.18`):** custom generators can loop indefinitely when
  size is zero (GHSA-2v37-7h3g-55p8), a transitive dependency of Vite.

These are not merely dev-tooling concerns to defer: `frontend/Dockerfile`'s
`CMD ["npx", "vite"]` means the container this repo actually runs — in
`compose.yaml`, exposed on the host — *is* Vite's dev server, not a built
static bundle served by a production HTTP server. A vulnerability in the
Vite dev server is therefore a vulnerability in the running application,
not just a local build-tool risk, and two of the three Vite advisories
name Windows path handling specifically — this repo's own documented
development environment.

`npm audit`'s suggested fix (`npm audit fix --force`) jumps to
`vite@8.2.1`, a two-major-version bump this repo has reason to be
cautious about: `package.json` already carries `rollup`/`esbuild`
overrides to WASM variants because "this container's network cannot
reach registry.npmjs.org reliably" (documented directly in
`frontend/Dockerfile`) — a environment constraint a large, unverified
major-version jump risks interacting with badly. Checking available
versions found `vite@6.4.3` (the latest 6.x patch, no major bump from the
existing `^5.4.8` range's next logical step) already contains the fix for
all three Vite advisories, verified directly: installing it and re-running
`npm audit` left only the transitive `nanoid` advisory, which `npm audit
fix` (no `--force`, no major bump) then resolved completely.

## Decision

- `frontend/package.json`'s `vite` dependency moves from `^5.4.8` to
  `^6.4.3` — the minimal version that resolves all three Vite advisories,
  not the major-version-8 jump `npm audit fix --force` would have applied.
- The transitive `nanoid` advisory is resolved via a plain `npm audit fix`
  (a lockfile-only change, no `package.json` edit).
- No other dependency, override, build script, or Dockerfile line changes.

## Scope

**In scope:** `frontend/package.json`'s `vite` version constraint;
`frontend/package-lock.json`'s resulting lockfile updates (`vite`,
`nanoid`, and their own transitive dependencies).

**Out of scope, named honestly:** upgrading to Vite 7 or 8 (not needed —
6.4.3 already resolves every reported advisory, and a larger jump carries
more unverified risk than this record needs to take on); replacing the
frontend container's `npx vite` runtime with a built-static-bundle-plus-
real-HTTP-server architecture (a separate, larger infrastructure decision
this record does not reopen — `CMD ["npx", "vite"]` itself is unchanged);
any other npm audit findings in devDependencies not reported as
vulnerable by this run.

## Options considered

- **Minimal-version bump to the latest patched 6.x release (chosen):**
  resolves every reported advisory with the smallest possible version
  delta, verified directly rather than trusting `npm audit fix --force`'s
  default choice of the newest major version.
- **`npm audit fix --force` to Vite 8.2.1:** would also resolve the
  advisories, but is a larger, unverified jump (two major versions) that
  risks breaking the existing Rollup/esbuild WASM overrides this
  environment already needs; not attempted since the smaller fix already
  works.
- **Defer, since this is "just a dev server" (rejected):** the container
  this repo actually runs *is* the Vite dev server (`CMD ["npx", "vite"]`),
  so these are live vulnerabilities in the running application, not a
  theoretical dev-only concern.

## Consequences

- **Positive:** closes 2 high-severity advisories in the code path the
  running frontend container actually executes, with the smallest
  possible dependency delta; `npm audit` now reports 0 vulnerabilities.
- **Negative / trade-off:** none identified — `npx tsc --noEmit`,
  `npm run build`, and the full Playwright suite (25 passed, 5 pre-existing
  skips, 0 failed) all pass unchanged under Vite 6.4.3.
- **Risk:** future Vite/nanoid advisories will need the same minimal-bump-
  first check repeated; this record does not establish an automated
  dependency-audit gate (a separate, larger CI decision).

## Exit criteria (evidence-checkable)

| Invariant | Evidence check id |
|---|---|
| `frontend/package.json` pins `vite` to `^6.4.3` or later | `vite-version-patched` |
| `npm audit` reports zero vulnerabilities in `frontend/` | `no-known-vulnerabilities` |
