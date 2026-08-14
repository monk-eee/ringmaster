# Architecture Decision Records

This directory contains bounded decisions that govern changes to ringmaster.
Records begin as `Proposed`. The named decider must explicitly accept a record
before implementation begins. Accepted records are immutable; later changes
must amend or supersede them with a new ADR.

The proposed bootstrap policy requires ADR coverage for codebase mutations, not
one ADR per mutation. Reuse an accepted ADR only while its scope and trigger
cover the work. Purely editorial corrections are exempt.

Each accepted ADR has an exact-name companion under `docs/evidence.d/`. The ADR
records intent; the evidence record describes current, rerunnable proof. Evidence
state must be derived by the repository checker after the evidence policy is
accepted and implemented.

## Records

| ADR | Status | Evidence |
|---|---|---|
| [ADR-0001: Require governing ADR coverage before implementation](0001-require-governing-adr-coverage-before-implementation.md) | Accepted | [EV-0001](../evidence.d/0001-require-governing-adr-coverage-before-implementation.md) |
| [ADR-0002: Keep current evidence separate from accepted decisions](0002-keep-current-evidence-separate-from-accepted-decisions.md) | Accepted | [EV-0002](../evidence.d/0002-keep-current-evidence-separate-from-accepted-decisions.md) |
| [ADR-0003: Ringmaster ingests MindLeak as an MCP source, not a shared graph](0003-ringmaster-ingests-mindleak-as-an-mcp-source.md) | Accepted | [EV-0003](../evidence.d/0003-ringmaster-ingests-mindleak-as-an-mcp-source.md) |
| [ADR-0004: Defer multi-user access control; keep sensitive commitment data local and unshared for v1](0004-defer-multi-user-access-control-single-user-v1.md) | Accepted | [EV-0004](../evidence.d/0004-defer-multi-user-access-control-single-user-v1.md) |
| [ADR-0005: Adopt a Rust service with an event-sourced Postgres commitment graph](0005-adopt-rust-event-sourced-postgres-commitment-graph.md) | Accepted | [EV-0005](../evidence.d/0005-adopt-rust-event-sourced-postgres-commitment-graph.md) |
| [ADR-0006: Local development stack runs via Podman Compose](0006-local-development-stack-runs-via-podman-compose.md) | Accepted | [EV-0006](../evidence.d/0006-local-development-stack-runs-via-podman-compose.md) |
| [ADR-0007: Generalize the event-sourced aggregate to Obligation and require pgvector](0007-generalize-obligation-and-require-pgvector.md) | Accepted | [EV-0007](../evidence.d/0007-generalize-obligation-and-require-pgvector.md) |
| [ADR-0008: Add an append-only audit_events table for security-relevant actions](0008-add-append-only-audit-events-table.md) | Accepted | [EV-0008](../evidence.d/0008-add-append-only-audit-events-table.md) |
| [ADR-0009: Add a generic node/edge graph substrate and source fragments table](0009-add-graph-nodes-edges-and-source-fragments.md) | Accepted | [EV-0009](../evidence.d/0009-add-graph-nodes-edges-and-source-fragments.md) |
| [ADR-0010: Transcript ingestion — parsing, chunking, and immutable source fragments](0010-transcript-ingestion-parsing-chunking-provenance.md) | Accepted | [EV-0010](../evidence.d/0010-transcript-ingestion-parsing-chunking-provenance.md) |
| [ADR-0011: Extraction pipeline — candidate schema, deterministic validation, and an optional model adapter](0011-extraction-pipeline-candidate-schema-and-model-adapter.md) | Accepted | [EV-0011](../evidence.d/0011-extraction-pipeline-candidate-schema-and-model-adapter.md) |
| [ADR-0012: Add a minimal HTTP read API and a Node web front end, tested with Playwright](0012-minimal-http-api-and-node-web-front-end.md) | Accepted | [EV-0012](../evidence.d/0012-minimal-http-api-and-node-web-front-end.md) |
| [ADR-0013: HTTP endpoints trigger and list model-based extraction candidates](0013-http-endpoints-trigger-and-list-extraction-candidates.md) | Accepted | [EV-0013](../evidence.d/0013-http-endpoints-trigger-and-list-extraction-candidates.md) |
| [ADR-0014: Replace the server-rendered front end with a React/Vite single-page app](0014-react-vite-single-page-app.md) | Accepted | [EV-0014](../evidence.d/0014-react-vite-single-page-app.md) |
| [ADR-0015: Expose source-fragment traceability on candidates](0015-expose-source-fragment-traceability-on-candidates.md) | Accepted | [EV-0015](../evidence.d/0015-expose-source-fragment-traceability-on-candidates.md) |
| [ADR-0016: Publish the ringmaster repository publicly on GitHub](0016-publish-repository-publicly-on-github.md) | Proposed | [EV-0016](../evidence.d/0016-publish-repository-publicly-on-github.md) |
| [ADR-0017: Add a GitHub Actions CI pipeline for backend, frontend, and governance](0017-add-github-actions-ci-pipeline.md) | Proposed | [EV-0017](../evidence.d/0017-add-github-actions-ci-pipeline.md) |