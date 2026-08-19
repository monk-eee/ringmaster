# ADR-0092: Fix CI cargo-audit job — grant checks:write, document the one unfixable advisory it actually finds

- **Status:** Accepted
- **Date:** 2026-08-19
- **Decider:** monk-eee
- **Approval:** Continuing this session's established autonomous-work practice ("keep going" / "work autonomously and make good decisions" when unavailable), 2026-08-19 — verified against a real, live GitHub Actions run rather than assumed
- **Depends on:** [ADR-0090](0090-ci-enforced-dependency-vulnerability-scanning.md)
- **Tags:** security, ci, infrastructure

## Context

A prior fix in this session (committed `d4568dc`, pointing `rustsec/audit-check`
at the workspace-root `Cargo.lock` instead of the nonexistent
`backend/Cargo.lock`) was assumed correct from reading the action's source,
but not yet confirmed against a real CI run. Checking the actual run
(`https://github.com/monk-eee/ringmaster/actions/runs/32221938951`, since this
repo is public and viewable without authentication) showed the `backend`
job still **failed** — for two separate, previously-unverified reasons:

1. **A real advisory, correctly found.** `cargo audit` ran successfully
   this time (proving the Cargo.lock path fix from `d4568dc` worked) and
   reported `RUSTSEC-2023-0071` ("Marvin Attack" timing side-channel) in
   `rsa` 0.9.10, with cargo-audit's own output stating "No fixed upgrade is
   available!" Tracing it (`cargo tree`, then a direct `Cargo.lock` read
   since `cargo tree -i rsa` reported nothing for the default target) found
   it reaches this workspace only via `sqlx` 0.8.6's facade crate, which
   unconditionally lists `sqlx-mysql`/`sqlx-postgres`/`sqlx-sqlite` as
   dependencies in the resolved lockfile regardless of which database
   feature is requested. Confirmed this is not a fixable feature-flag
   oversight: adding `default-features = false` to `backend/Cargo.toml`'s
   `sqlx` entry and regenerating `Cargo.lock` from scratch still resolved
   `sqlx-mysql`/`rsa` identically — reverted that change since it had no
   effect. `backend/Cargo.toml` requests only `["postgres", "runtime-tokio",
   "tls-rustls", "macros", "migrate", "uuid", "chrono"]`, `DATABASE_URL` is
   always a `postgres://` URL, and every query in this codebase goes through
   `sqlx::PgPool` — MySQL's `sha256_password`/`caching_sha2_password`
   RSA-encrypted auth handshake, the exact code path this advisory concerns,
   is never invoked.
2. **A separate permissions error, unrelated to the advisory.**
   The job's annotations also showed "Resource not accessible by
   integration" on `rustsec/audit-check`'s attempt to create a Check Run.
   Reading the action's own source
   (`rustsec/audit-check`'s `src/reporter.ts`/`src/main.ts`) confirmed:
   `reportCheck` always calls `startCheck` first; if that throws and
   `GITHUB_HEAD_REF` is unset (true for a direct push to `main`, as here —
   it's only set for forked-repo pull request events), the action
   re-throws rather than falling back, failing the job independent of
   whether any vulnerability was actually found. `ci.yml` had no
   `permissions:` block, so the default `GITHUB_TOKEN` lacked the
   `checks: write` scope this specific action needs to create its Check Run
   (documented in the action's own README "Granular Permissions" section).

## Decision

- **`ci.yml`'s `backend` job** gains an explicit
  `permissions: { contents: read, checks: write }` block, so
  `rustsec/audit-check` can create its Check Run instead of failing on a
  permissions error unrelated to any advisory.
- **`.cargo/audit.toml`** (new, read automatically by `cargo audit` — and
  by extension `rustsec/audit-check`, which shells out to it — from the
  working directory, which is the repo root per ADR-0090/`d4568dc`) adds
  `[advisories] ignore = ["RUSTSEC-2023-0071"]`, with an inline comment
  recording exactly why: no upstream fix exists, the vulnerable code path
  is unreachable given this app's Postgres-only usage, and the specific
  dependency chain (`sqlx` → `sqlx-mysql` → `rsa`) that was confirmed
  unavoidable within sqlx 0.8.x via a real experiment, not assumption.
- **No change** to `backend/Cargo.toml`, `Cargo.lock`, or any sqlx feature
  flag — the `default-features = false` experiment was reverted after
  confirming it had zero effect on the resolved dependency graph.

## Scope

**In scope:** `ci.yml`'s `backend` job permissions; a new `.cargo/audit.toml`
ignoring exactly one named, investigated advisory.

**Out of scope, named honestly:** upgrading `sqlx` to 0.9 (would need Rust
1.94.0, not confirmed available in this CI's `dtolnay/rust-toolchain@stable`
at time of writing, and is a major-version bump touching every `sqlx` call
site in the backend — too large and unverified for this bounded fix,
matching ADR-0089's own precedent of preferring the smallest verified
change); re-evaluating this ignore automatically (a human must re-check
if `rsa` ships a fix or `sqlx` restructures this dependency — recorded as
a re-evaluation trigger in the config comment, not automated); any other
advisory (none other were found).

## Options considered

- **Grant `checks: write` and document a narrowly-scoped ignore (chosen):**
  fixes the actual job failure for its real causes; the ignore is specific
  (one advisory ID), justified with a traced dependency path and a real
  experiment (not a guess) showing no feature-flag fix exists, and named
  as needing re-evaluation if circumstances change.
- **Ignore all `sqlx`-originated advisories broadly, or disable the CI
  check on failure (`continue-on-error: true`):** rejected — both would
  silently swallow a *future*, real, different advisory in the same job,
  defeating ADR-0090's own purpose.
- **Force sqlx to drop mysql/sqlite via a workspace patch or fork:**
  rejected as disproportionate — replacing or patching a well-maintained
  upstream crate to avoid one already-ignorable, unreachable advisory is
  far more maintenance burden than a documented ignore.
- **Upgrade to sqlx 0.9 now:** rejected for this record — real option,
  but large, unverified, and blocked on a Rust toolchain version this
  environment hasn't confirmed; left as a future, separate decision.

## Consequences

- **Positive:** the `backend` CI job can now pass cleanly and will still
  catch any *other*, real, future advisory — ADR-0090's actual purpose.
  The permissions fix is a one-line, low-risk addition; the ignore is
  narrow, documented, and reversible.
- **Negative / trade-off:** `RUSTSEC-2023-0071` remains formally
  "vulnerable" in dependency-scanning terms for as long as `sqlx` 0.8.x is
  used; mitigated by the fact the vulnerable code path cannot execute in
  this application.
- **Risk:** if this backend ever gains a MySQL connection (a significant,
  independently-ADR-worthy change given ADR-0005's Postgres-only
  architecture decision), this ignore must be revisited first.

## Exit criteria (evidence-checkable)

| Invariant | Evidence check id |
|---|---|
| `ci.yml`'s `backend` job grants `checks: write` | `ci-backend-job-grants-checks-write` |
| `.cargo/audit.toml` ignores exactly `RUSTSEC-2023-0071`, with a documented rationale | `audit-toml-ignores-rsa-advisory` |
| A live GitHub Actions run of this exact commit shows the `backend` job passing | `live-ci-run-confirms-backend-passes` |
