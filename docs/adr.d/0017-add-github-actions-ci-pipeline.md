# ADR-0017: Add a GitHub Actions CI pipeline for backend, frontend, and governance

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Explicitly accepted by monk-eee on 2026-08-14
- **Depends on:** [ADR-0016](0016-publish-repository-publicly-on-github.md), [ADR-0001](0001-require-governing-adr-coverage-before-implementation.md), [ADR-0006](0006-local-development-stack-runs-via-podman-compose.md)
- **Tags:** infrastructure, ci, automation, governance

## Context

[ADR-0001](0001-require-governing-adr-coverage-before-implementation.md)
explicitly listed "blocking CI enforcement" as out of scope and recorded, as
a risk, that "GitHub review wiring will be added only after this ADR is
accepted." [ADR-0006](0006-local-development-stack-runs-via-podman-compose.md)
separately listed "CI pipeline container usage" as needing its own governing
ADR before implementation. Neither has been superseded; today, `cargo test`,
the frontend build, and AGENTS.md's own mandated validation gate
(`node scripts/check-evidence.mjs` and `git diff --check`) only ever run
manually, by whichever agent or human happens to run them. Now that the
repository is hosted on GitHub ([ADR-0016](0016-publish-repository-publicly-on-github.md)),
monk-eee wants this automated ("build pipelines and everything").

## Decision

Add GitHub Actions workflow(s) under `.github/workflows/` that run
automatically on `push` and `pull_request` targeting `main`:

1. **backend job:** provisions Postgres with the pgvector extension (the
   same `pgvector/pgvector:pg16` image `compose.yaml` already uses per
   [ADR-0007](0007-generalize-obligation-and-require-pgvector.md)) as a
   service container, applies `backend/migrations/` (via `sqlx-cli`), then
   runs `cargo test --manifest-path backend/Cargo.toml`. The live-model
   round-trip test already self-skips without `RINGMASTER_LLM_URL`
   ([ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md));
   no Ollama or other model server is provisioned in CI.
2. **frontend job:** `npm ci`, then `npm run build` (Vite) and a TypeScript
   typecheck (`tsc --noEmit`).
3. **governance job:** runs `node scripts/check-evidence.mjs` and
   `git diff --check` — the exact gate AGENTS.md already mandates locally,
   now enforced automatically on every push and pull request.

These jobs report status (visible check runs on commits and pull requests);
turning them into a hard, merge-blocking branch-protection requirement is a
separate GitHub repository-settings action that this ADR does not itself
perform, and is reasonable to enable once the pipeline has run green at
least once.

## Scope

**In scope:** a backend test job, a frontend build/typecheck job, and a
governance (evidence + doc-link) job, each a GitHub Actions workflow
triggered on `push`/`pull_request` to `main`.

**Out of scope:** end-to-end Playwright execution in CI (needs multi-service
orchestration and depends on the in-flight
[ADR-0014](0014-react-vite-single-page-app.md) React/Vite migration settling
first — a natural follow-on, not blocked forever, and could reuse
`compose.yaml` via `docker compose` to validate ADR-0006's runtime-agnostic
design); deployment/CD to any hosting target (none exists yet); branch
protection / required-status-check configuration in GitHub's repository
settings; container image publishing/registries.

## Options considered

- **GitHub Actions, native runners + a Postgres service container (chosen):**
  no extra tooling to install, keeps CI fast, matches the now-public GitHub
  hosting from [ADR-0016](0016-publish-repository-publicly-on-github.md).
- **Reuse `podman compose` inside CI, mirroring local dev exactly:** rejected
  for now — GitHub-hosted runners don't ship Podman by default, and
  rootless-in-CI quirks add fragility for no benefit over native service
  containers; `compose.yaml`'s standard-format design
  ([ADR-0006](0006-local-development-stack-runs-via-podman-compose.md)) is
  still exercised by the deferred e2e follow-on using `docker compose`
  instead.
- **Skip CI, stay manual:** rejected — explicitly what monk-eee asked to
  change.

## Consequences

- **Positive:** every push and pull request automatically proves backend
  tests, the frontend build, and the ADR/evidence governance gate, closing
  the "blocking CI enforcement" gap
  [ADR-0001](0001-require-governing-adr-coverage-before-implementation.md)
  explicitly deferred.
- **Negative / trade-off:** CI run minutes; a new workflow file to maintain;
  migrations must stay runnable standalone via `sqlx-cli`, not only via the
  backend binary's own startup `sqlx::migrate!` call.
- **Risk:** CI can go red during legitimate in-flight work (for example, the
  ongoing ADR-0014 migration). That is expected, correct signal, not a
  defect to suppress or a reason to weaken a check.

## Exit criteria and evidence

Evidence: [EV-0017](../evidence.d/0017-add-github-actions-ci-pipeline.md)

| Exit criterion | Evidence |
|---|---|
| A GitHub Actions workflow defines a backend test job | `workflow-defines-backend-job` |
| A GitHub Actions workflow defines a frontend build job | `workflow-defines-frontend-job` |
| A GitHub Actions workflow defines a governance/evidence job | `workflow-defines-governance-job` |
| The pipeline has run successfully on GitHub | `pipeline-observed-green` |
