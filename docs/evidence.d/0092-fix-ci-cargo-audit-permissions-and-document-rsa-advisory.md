# EV-0092: Fix CI cargo-audit job — grant checks:write, document the one unfixable advisory it actually finds

Evidence for [ADR-0092](../adr.d/0092-fix-ci-cargo-audit-permissions-and-document-rsa-advisory.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0092-fix-ci-cargo-audit-permissions-and-document-rsa-advisory"

[[check]]
id = "ci-backend-job-grants-checks-write"
invariant = "ci.yml's backend job grants checks: write."
type = "present"
pattern = 'checks:\s*write'
paths = [".github/workflows/ci.yml"]

[[check]]
id = "audit-toml-ignores-rsa-advisory"
invariant = "cargo audit config ignores exactly RUSTSEC-2023-0071, with a documented rationale."
type = "present"
pattern = 'ignore = \["RUSTSEC-2023-0071"\]'
paths = [".cargo/audit.toml"]

[[check]]
id = "live-ci-run-confirms-backend-passes"
invariant = "A live GitHub Actions run of this exact commit shows the backend job passing."
type = "manual"
last_verified = "2026-08-19"
rationale = "Verified locally first: `cargo audit` (v0.22.2, installed fresh since this session's earlier attempt to install it had been abandoned for slowness) against the real workspace Cargo.lock exits 0 with .cargo/audit.toml's ignore in place (previously: 'Crate: rsa ... error: 1 vulnerability found!', exit 1). Confirming the live GitHub Actions run of the commit containing both this fix and the permissions grant is the actual proof for CI, since the local cargo-audit invocation and the CI rustsec/audit-check invocation are two different code paths (a local binary vs. a Node-based Action) that could plausibly diverge; re-check the Actions run for this repo's latest push (https://github.com/monk-eee/ringmaster/actions/workflows/ci.yml, viewable without authentication since this repo is public) whenever this file's last_verified date is stale relative to the latest commit touching ci.yml or .cargo/audit.toml."
```

## Notes

Root-caused via a real, live CI run rather than assuming the earlier
`d4568dc` fix was sufficient: `https://github.com/monk-eee/ringmaster/actions/runs/32221938951`
showed `frontend`/`governance` passing (confirming `d4568dc`'s Cargo.lock
path fix worked) but `backend` still failing, for two independent reasons
traced from the run's annotations and `rustsec/audit-check`'s own source
(`src/reporter.ts`, `src/main.ts`, `src/input.ts`) — see ADR-0092's Context.

Investigated the advisory itself rather than reflexively ignoring it:
installed `cargo-audit` locally (a ~6-minute from-source compile, the
same cost ADR-0090 named and chose to avoid in CI by using the action's
pre-built binary instead) specifically to confirm which advisory, in
which package, via which dependency path. Traced `rsa` 0.9.10 to `sqlx`
0.8.6's facade crate via `Cargo.lock` (not `cargo tree -i rsa`, which
reported nothing for the default target). Tested whether
`default-features = false` on the `sqlx` dependency would remove
`sqlx-mysql`/`rsa` from the graph: added it, ran `cargo check` (passed),
then regenerated `Cargo.lock` completely from scratch to rule out stale
lockfile entries — `sqlx-mysql`/`rsa` still resolved identically either
way. Reverted the `default-features = false` change (zero measured
effect, not worth the added surface) and restored the original
`Cargo.lock` from a backup taken before regenerating it, keeping this
fix's diff minimal and focused on the two things that actually needed to
change: CI permissions and a documented ignore.
