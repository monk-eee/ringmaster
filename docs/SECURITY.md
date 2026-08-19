# Security policy

Ringmaster is a single-user, local-only tool
([ADR-0004](adr.d/0004-defer-multi-user-access-control-single-user-v1.md)):
one operator, one local Postgres instance, no shared deployment, no
multi-tenant access model. That scope shapes what "security" means here.

## Reporting a vulnerability

Open an issue on this repository, or contact the repository owner
([monk-eee](https://github.com/monk-eee)) directly for anything sensitive
enough that it shouldn't go in a public issue. There is no dedicated
security email or bug bounty program.

## Supported versions

Only the `main` branch is supported. There are no released/tagged
versions to backport fixes to.

## Scope and known boundaries

- **Not a hardened multi-tenant service.** Per ADR-0004, no multi-user
  authorization model is implemented, and People-commitment data must not
  be synced, exported, or shared beyond the single local operator without
  a new or amending ADR.
- **Dependency scanning is enforced in CI**
  ([ADR-0090](adr.d/0090-ci-enforced-dependency-vulnerability-scanning.md)):
  `npm audit --omit=dev --audit-level=high` (frontend) and `cargo audit`
  via `rustsec/audit-check` (backend) run on every push/PR to `main` and
  fail the build on a new high-severity finding. Any advisory judged
  unreachable in this codebase's actual usage is documented, not silently
  suppressed — see `.cargo/audit.toml`
  ([ADR-0092](adr.d/0092-fix-ci-cargo-audit-permissions-and-document-rsa-advisory.md))
  for the one currently-ignored, upstream-unfixable advisory and why.
- **Local development defaults are intentionally weak**
  (`compose.yaml`'s `ringmaster-dev` Postgres password, etc.) and are not a
  vulnerability report — this stack is documented as local-development-only
  ([ADR-0006](adr.d/0006-local-development-stack-runs-via-podman-compose.md)).
