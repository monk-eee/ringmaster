# ringmaster

![Ringmaster](assets/ringmaster_logo.png)

Canonical repository: [github.com/monk-eee/ringmaster](https://github.com/monk-eee/ringmaster) ([ADR-0016](docs/adr.d/0016-publish-repository-publicly-on-github.md)).

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
podman compose logs -f backend frontend
podman compose down   # stop; add -v to also drop the local Postgres volume
```

This brings up Postgres, the Rust backend
([ADR-0005](docs/adr.d/0005-adopt-rust-event-sourced-postgres-commitment-graph.md),
serving its HTTP API on `:8080`) and the React/Vite front end
([ADR-0014](docs/adr.d/0014-react-vite-single-page-app.md), at
`http://localhost:3000`) locally; it is dev tooling only, not a deployment
artifact. Copy `.env.example` to `.env` to override the default dev
credentials.

### Front-end tests

The front end's Playwright suite assumes Postgres and the backend are
already running (`podman compose up -d`):

```bash
cd frontend
npm install
npx playwright install --with-deps chromium   # first time only
BACKEND_URL=http://localhost:8080 npx playwright test
```

## Contributing

Read [AGENTS.md](AGENTS.md) and the [contributor guide](docs/CONTRIBUTING.md)
before changing the repository. Engineering decisions are indexed in
[`docs/adr.d/`](docs/adr.d/README.md), with current proof in
[`docs/evidence.d/`](docs/evidence.d/).

Validate decision evidence with:

```bash
node scripts/check-evidence.mjs
```