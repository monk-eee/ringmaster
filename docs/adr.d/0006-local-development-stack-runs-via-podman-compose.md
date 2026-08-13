# ADR-0006: Local development stack runs via Podman Compose

- **Status:** Accepted
- **Date:** 2026-08-13
- **Decider:** monk-eee
- **Approval:** Explicitly accepted by monk-eee on 2026-08-13
- **Depends on:** [ADR-0004](0004-defer-multi-user-access-control-single-user-v1.md), [ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md)
- **Tags:** infrastructure, local-dev, containers, podman

## Context

[ADR-0004](0004-defer-multi-user-access-control-single-user-v1.md) fixes v1
to a single local Postgres instance. [ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md)
fixes the backend to Rust with an event-sourced Postgres commitment graph,
but explicitly leaves "hosting or deployment beyond the single local
instance" out of scope. No infrastructure configuration exists yet in this
repository, and running Postgres and the eventual Rust service natively on
every contributor machine is not currently reproducible.

monk-eee wants a reproducible local development environment using
containers, covering both the Rust backend service and Postgres, using
Podman as the container runtime. This is local developer convenience only —
it does not change the single-instance, single-operator scope already fixed
by ADR-0004, and it does not address production or deployment packaging.

## Decision

- Local development must run the Rust backend service and Postgres as
  containers defined in a Compose-format file (`compose.yaml`) at the
  repository root.
- Podman (via `podman compose` or `podman-compose`) is the primary supported
  local container runtime. The Compose file must use the standard Compose
  file format so it is not exclusively tied to one runtime's proprietary
  syntax, but Podman is what this repository's tooling and documentation
  target and validate against.
- The Compose file must define, at minimum, a `postgres` service and a
  backend service for the Rust application, with the backend depending on
  Postgres being available.
- This decision governs local development only. It does not define a
  production/deployment container image, CI container usage, or a VS Code
  devcontainer — each would need its own governing ADR before implementation.

## Scope

**In scope:** the local development Compose file, the services it must
define (Postgres, Rust backend), and Podman as the primary supported
runtime.

**Out of scope:** production/deployment container images, CI pipeline
container usage, `.devcontainer/` configuration, and the specific Postgres
schema or Rust crate layout (governed by [ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md)).

## Options considered

- **Podman Compose for the full local stack (chosen):** matches monk-eee's
  stated runtime and scope; gives one reproducible command to bring up both
  services without requiring a native Postgres or Rust toolchain install.
- **Containerize Postgres only, run Rust natively:** simpler, but leaves the
  backend's build/runtime environment unreproducible across machines, and
  monk-eee explicitly wants the full stack containerized.
- **Docker Desktop instead of Podman:** a common default, but not what was
  requested; Podman avoids Docker Desktop's licensing model for this kind of
  use.
- **VS Code devcontainer:** would containerize the whole dev environment
  including editor tooling, not just the runtime services; broader in scope
  than what was requested and would still need its own ADR.

## Consequences

- **Positive:** local development becomes reproducible across machines
  without a native Postgres or Rust install; stays consistent with ADR-0004's
  single local instance.
- **Negative / trade-off:** the repository now carries container definitions
  and a Rust build/runtime image to maintain before any product code exists.
- **Risk:** Podman's Compose compatibility has occasional gaps versus Docker
  Compose. Mitigated by keeping the Compose file in the standard format and
  treating any Podman-specific workaround as an implementation detail
  documented alongside the file, not a reason to reopen this decision.

## Exit criteria and evidence

Evidence: [EV-0006](../evidence.d/0006-local-development-stack-runs-via-podman-compose.md)

| Exit criterion | Evidence |
|---|---|
| A Compose file at the repository root defines both a Postgres service and a Rust backend service | `compose-defines-postgres`, `compose-defines-backend` |
| Local development instructions name Podman as the primary supported runtime | `docs-name-podman-as-runtime` |
