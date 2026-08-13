# ringmaster

![Ringmaster](assets/ringmaster_logo.png)

Ringmaster is a management operating system: it maintains a living model of
commitments, people, work, outcomes, risks, and time so managers spend less
effort reconstructing reality and more effort making decisions. See
[docs/VISION.md](docs/VISION.md) for the product vision and
[docs/PRODUCT-SPEC.md](docs/PRODUCT-SPEC.md) for the detailed, versioned
product specification.

Product behavior, build commands, and detailed architecture will be recorded
as those decisions are made and accepted; this repository does not invent
them in advance.

## Local development

Podman is the primary supported container runtime for local development
([ADR-0006](docs/adr.d/0006-local-development-stack-runs-via-podman-compose.md)).

```bash
podman machine init   # first time only
podman machine start  # if not already running
podman compose up -d
podman compose logs -f backend
podman compose down   # stop; add -v to also drop the local Postgres volume
```

This brings up Postgres and the Rust backend
([ADR-0005](docs/adr.d/0005-adopt-rust-event-sourced-postgres-commitment-graph.md))
locally; it is dev tooling only, not a deployment artifact. Copy
`.env.example` to `.env` to override the default dev credentials.

## Contributing

Read [AGENTS.md](AGENTS.md) and the [contributor guide](docs/CONTRIBUTING.md)
before changing the repository. Engineering decisions are indexed in
[`docs/adr.d/`](docs/adr.d/README.md), with current proof in
[`docs/evidence.d/`](docs/evidence.d/).

Validate decision evidence with:

```bash
node scripts/check-evidence.mjs
```