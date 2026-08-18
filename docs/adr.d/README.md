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
| [ADR-0016: Publish the ringmaster repository publicly on GitHub](0016-publish-repository-publicly-on-github.md) | Accepted | [EV-0016](../evidence.d/0016-publish-repository-publicly-on-github.md) |
| [ADR-0017: Add a GitHub Actions CI pipeline for backend, frontend, and governance](0017-add-github-actions-ci-pipeline.md) | Accepted | [EV-0017](../evidence.d/0017-add-github-actions-ci-pipeline.md) |
| [ADR-0018: Generate and store embeddings for source fragments](0018-generate-and-store-source-fragment-embeddings.md) | Accepted | [EV-0018](../evidence.d/0018-generate-and-store-source-fragment-embeddings.md) |
| [ADR-0019: Semantic search over embedded source fragments](0019-semantic-search-over-source-fragments.md) | Accepted | [EV-0019](../evidence.d/0019-semantic-search-over-source-fragments.md) |
| [ADR-0020: Add due-date fields to Obligation, the schema prerequisite for Epic E7](0020-obligation-due-date-fields.md) | Accepted | [EV-0020](../evidence.d/0020-obligation-due-date-fields.md) |
| [ADR-0021: Ratify surfacing semantic search in the frontend SPA (retroactive)](0021-ratify-search-tab-surfaced-without-its-own-adr.md) | Accepted | [EV-0021](../evidence.d/0021-ratify-search-tab-surfaced-without-its-own-adr.md) |
| [ADR-0022: A read-only Daily Brief endpoint — Obligations ranked by urgency](0022-daily-brief-endpoint.md) | Accepted | [EV-0022](../evidence.d/0022-daily-brief-endpoint.md) |
| [ADR-0023: Evidence-backed Daily Brief reasons — source-fragment traceability on Obligation](0023-evidence-backed-daily-brief-reasons.md) | Accepted | [EV-0023](../evidence.d/0023-evidence-backed-daily-brief-reasons.md) |
| [ADR-0024: Accept/reject buttons for candidates — Epic E5's first interactive slice](0024-candidate-accept-reject-buttons.md) | Accepted | [EV-0024](../evidence.d/0024-candidate-accept-reject-buttons.md) |
| [ADR-0025: Node/edge write API and neighborhood traversal](0025-node-edge-write-api-and-traversal.md) | Accepted | [EV-0025](../evidence.d/0025-node-edge-write-api-and-traversal.md) |
| [ADR-0026: Graph explorer frontend — data entry, drill-down, and relationship visualization](0026-graph-explorer-frontend.md) | Accepted | [EV-0026](../evidence.d/0026-graph-explorer-frontend.md) |
| [ADR-0027: Promote an accepted candidate into an Obligation](0027-promote-accepted-candidate-to-obligation.md) | Accepted | [EV-0027](../evidence.d/0027-promote-accepted-candidate-to-obligation.md) |
| [ADR-0028: Person relationship view — resolve linked Obligations into a per-person page](0028-person-relationship-view.md) | Accepted | [EV-0028](../evidence.d/0028-person-relationship-view.md) |
| [ADR-0029: Time Horizon view — Obligations bucketed by due-date window](0029-time-horizon-view.md) | Accepted | [EV-0029](../evidence.d/0029-time-horizon-view.md) |
| [ADR-0030: Human-readable titles and type iconography across the UI](0030-human-readable-titles-and-type-iconography.md) | Accepted | [EV-0030](../evidence.d/0030-human-readable-titles-and-type-iconography.md) |
| [ADR-0031: Suggested Focus Blocks — group Obligations sharing a linked node](0031-suggested-focus-blocks.md) | Accepted | [EV-0031](../evidence.d/0031-suggested-focus-blocks.md) |
| [ADR-0032: Wire up edge temporal validity — supersede-on-write and relationship history in the Graph Explorer](0032-temporal-edge-validity-supersede-on-write.md) | Accepted | [EV-0032](../evidence.d/0032-temporal-edge-validity-supersede-on-write.md) |
| [ADR-0033: Progressive graph traversal trail over one-hop neighborhoods](0033-progressive-graph-traversal-trail.md) | Accepted | [EV-0033](../evidence.d/0033-progressive-graph-traversal-trail.md) |
| [ADR-0034: Expose atomic meeting-transcript ingestion over HTTP](0034-http-meeting-transcript-ingestion.md) | Accepted | [EV-0034](../evidence.d/0034-http-meeting-transcript-ingestion.md) |
| [ADR-0035: Time Horizon timeline view — an alternative, zoomable presentation of the existing bucketed data](0035-time-horizon-timeline-view.md) | Accepted | [EV-0035](../evidence.d/0035-time-horizon-timeline-view.md) |
| [ADR-0036: Meeting detail read — one meeting with its ordered transcript fragments](0036-meeting-detail-read.md) | Accepted | [EV-0036](../evidence.d/0036-meeting-detail-read.md) |
| [ADR-0037: Meeting-scoped candidate listing and extraction progress](0037-meeting-scoped-candidate-listing.md) | Accepted | [EV-0037](../evidence.d/0037-meeting-scoped-candidate-listing.md) |
| [ADR-0038: Wire up audit_events for candidate validation actions](0038-wire-up-audit-events-for-candidate-validation.md) | Accepted | [EV-0038](../evidence.d/0038-wire-up-audit-events-for-candidate-validation.md) |
| [ADR-0039: Product re-steer — Today/Timeline/People/Inbox as primary navigation](0039-product-re-steer-primary-navigation.md) | Accepted | [EV-0039](../evidence.d/0039-product-re-steer-primary-navigation.md) |
| [ADR-0040: Dated source ingestion — occurred_at becomes a required, structured field across every ingested source](0040-dated-source-ingestion.md) | Accepted | [EV-0040](../evidence.d/0040-dated-source-ingestion.md) |
| [ADR-0041: Risk Engine v1 — staleness and date-compression signals](0041-risk-engine-v1-staleness-and-date-compression-signals.md) | Accepted | [EV-0041](../evidence.d/0041-risk-engine-v1-staleness-and-date-compression-signals.md) |
| [ADR-0042: Surface occurred_at on nodes, with date-range retrieval and a second MCP tool](0042-occurred-at-retrieval-and-recall-sources-mcp-tool.md) | Accepted | [EV-0042](../evidence.d/0042-occurred-at-retrieval-and-recall-sources-mcp-tool.md) |
| [ADR-0043: Meeting Review page — transcript fragments with inline extracted candidates](0043-meeting-review-page.md) | Accepted | [EV-0043](../evidence.d/0043-meeting-review-page.md) |
| [ADR-0044: Today attention items show management meaning, not identifiers](0044-today-attention-items-management-meaning.md) | Accepted | [EV-0044](../evidence.d/0044-today-attention-items-management-meaning.md) |
| [ADR-0045: Correct a candidate before accepting it](0045-correct-candidate-before-accepting.md) | Accepted | [EV-0045](../evidence.d/0045-correct-candidate-before-accepting.md) |
| [ADR-0046: Unowned-obligation risk signal via existing owns edges](0046-unowned-obligation-risk-signal.md) | Accepted | [EV-0046](../evidence.d/0046-unowned-obligation-risk-signal.md) |
| [ADR-0047: Obligation detail page — a first-class read view over existing data](0047-obligation-detail-page.md) | Accepted | [EV-0047](../evidence.d/0047-obligation-detail-page.md) |
| [ADR-0049: Audit trail read API — a chronological activity feed](0049-audit-trail-read-api.md) | Accepted | [EV-0049](../evidence.d/0049-audit-trail-read-api.md) |
| [ADR-0050: Today attention budget — cap Focus Blocks, remove their raw id, honest "show all"](0050-today-attention-budget.md) | Accepted | [EV-0050](../evidence.d/0050-today-attention-budget.md) |
| [ADR-0051: Relationship workspace — People shows who needs something from you, not every person node](0051-relationship-workspace.md) | Accepted | [EV-0051](../evidence.d/0051-relationship-workspace.md) |
| [ADR-0052: Context-derived Focus Sessions — group by shared node *and* similar timeframe, not node alone](0052-context-derived-focus-sessions.md) | Accepted | [EV-0052](../evidence.d/0052-context-derived-focus-sessions.md) |
| [ADR-0053: "What am I forgetting?" — compose existing risk signals into one capped, prominent list](0053-what-am-i-forgetting.md) | Accepted | [EV-0053](../evidence.d/0053-what-am-i-forgetting.md) |
| [ADR-0054: Congruence Engine v1 — flag a commitment with no linked node at all](0054-congruence-engine-v1-isolated-commitment-signal.md) | Accepted | [EV-0054](../evidence.d/0054-congruence-engine-v1-isolated-commitment-signal.md) |
| [ADR-0056: Local test-database isolation, plus a reviewable (not auto-run) dev-data cleanup](0056-local-test-database-isolation-and-dev-data-cleanup.md) | Accepted | [EV-0056](../evidence.d/0056-local-test-database-isolation-and-dev-data-cleanup.md) |
| [ADR-0057: Enforce test-database isolation with a runtime guard](0057-enforce-test-database-isolation-with-a-runtime-guard.md) | Accepted | [EV-0057](../evidence.d/0057-enforce-test-database-isolation-with-a-runtime-guard.md) |
| [ADR-0058: Extract a due date from a candidate and carry it to the promoted obligation](0058-extract-candidate-due-date-to-obligation.md) | Accepted | [EV-0058](../evidence.d/0058-extract-candidate-due-date-to-obligation.md) |
| [ADR-0059: List-view pagination for Obligations, Candidates, and People](0059-list-view-pagination.md) | Accepted | [EV-0059](../evidence.d/0059-list-view-pagination.md) |
| [ADR-0060: Extract an owner name from a candidate and link it at promotion](0060-extract-candidate-owner-and-link-at-promotion.md) | Accepted | [EV-0060](../evidence.d/0060-extract-candidate-owner-and-link-at-promotion.md) |
| [ADR-0061: A derived Obligation health label — composing existing status and signals, not a new score](0061-obligation-health-label.md) | Accepted | [EV-0061](../evidence.d/0061-obligation-health-label.md) |
| [ADR-0062: Auto-embed fragments on ingest (best-effort), so search has data](0062-auto-embed-fragments-on-ingest.md) | Accepted | [EV-0062](../evidence.d/0062-auto-embed-fragments-on-ingest.md) |
| [ADR-0066: Expose non-destructive graph management over MCP](0066-non-destructive-graph-management-over-mcp.md) | Accepted | [EV-0066](../evidence.d/0066-non-destructive-graph-management-over-mcp.md) |
| [ADR-0067: Pin the local frontend to port 3001](0067-pin-local-frontend-to-port-3001.md) | Accepted | [EV-0067](../evidence.d/0067-pin-local-frontend-to-port-3001.md) |
| [ADR-0068: Add an optional API key to the embedding adapter](0068-embedding-adapter-optional-api-key.md) | Accepted | [EV-0068](../evidence.d/0068-embedding-adapter-optional-api-key.md) |
| [ADR-0069: Resolve participant/speaker names to existing Person nodes at ingestion](0069-resolve-participants-to-person-nodes-at-ingestion.md) | Accepted | [EV-0069](../evidence.d/0069-resolve-participants-to-person-nodes-at-ingestion.md) |
| [ADR-0070: Derive Person interaction recency from identity edges with a legacy fallback](0070-edge-backed-person-interaction-recency.md) | Accepted | [EV-0070](../evidence.d/0070-edge-backed-person-interaction-recency.md) |
| [ADR-0071: Surface recent interaction sources on Person detail](0071-person-detail-recent-interactions.md) | Accepted | [EV-0071](../evidence.d/0071-person-detail-recent-interactions.md) |
| [ADR-0072: Split oversized, low-cohesion backend modules with no behavior change](0072-split-oversized-low-cohesion-backend-modules.md) | Accepted | [EV-0072](../evidence.d/0072-split-oversized-low-cohesion-backend-modules.md) |
| [ADR-0073: Isolate Playwright from the development database](0073-isolate-playwright-from-dev-database.md) | Accepted | [EV-0073](../evidence.d/0073-isolate-playwright-from-dev-database.md) |
| [ADR-0074: Visual design system refresh — a considered look, zero behavior change](0074-visual-design-system-refresh.md) | Accepted | [EV-0074](../evidence.d/0074-visual-design-system-refresh.md) |
| [ADR-0075: Restore the mascot logo in the app header](0075-restore-mascot-logo-in-header.md) | Accepted | [EV-0075](../evidence.d/0075-restore-mascot-logo-in-header.md) |
| [ADR-0076: Bulk candidate triage — multi-select accept/reject, confidence-first ordering](0076-bulk-candidate-triage.md) | Accepted | [EV-0076](../evidence.d/0076-bulk-candidate-triage.md) |
| [ADR-0077: Bulk candidate promotion — complete the triage loop ADR-0076 started](0077-bulk-candidate-promotion.md) | Accepted | [EV-0077](../evidence.d/0077-bulk-candidate-promotion.md) |
| [ADR-0078: Log build provenance so stale containers are visible on startup](0078-log-build-provenance-to-detect-stale-containers.md) | Accepted | [EV-0078](../evidence.d/0078-log-build-provenance-to-detect-stale-containers.md) |
| [ADR-0079: Timeline surfaces a linked source's own occurred_at](0079-timeline-surfaces-source-occurred-at.md) | Accepted | [EV-0079](../evidence.d/0079-timeline-surfaces-source-occurred-at.md) |
| [ADR-0080: Promote Graph Explorer to primary navigation](0080-promote-graph-explorer-to-primary-navigation.md) | Accepted | [EV-0080](../evidence.d/0080-promote-graph-explorer-to-primary-navigation.md) |
| [ADR-0081: Add an Actions lens to Graph Explorer's neighbourhood view](0081-graph-explorer-actions-lens.md) | Accepted | [EV-0081](../evidence.d/0081-graph-explorer-actions-lens.md) |
| [ADR-0082: Repeated-concern signal — the same risk raised in multiple meetings, still unpromoted](0082-repeated-concern-risk-signal.md) | Accepted | [EV-0082](../evidence.d/0082-repeated-concern-risk-signal.md) |
| [ADR-0083: Meeting-brief generation — a person's open commitments, recent asks, and risks in one call](0083-meeting-brief-generation.md) | Accepted | [EV-0083](../evidence.d/0083-meeting-brief-generation.md) |
| [ADR-0084: Today's narrative summary — the ranked count line VISION.md describes](0084-today-narrative-summary.md) | Accepted | [EV-0084](../evidence.d/0084-today-narrative-summary.md) |
| [ADR-0085: Focus Sessions filter to People-linked blocks — the one honestly-groundable attention-type slice](0085-focus-blocks-people-filter.md) | Accepted | [EV-0085](../evidence.d/0085-focus-blocks-people-filter.md) |