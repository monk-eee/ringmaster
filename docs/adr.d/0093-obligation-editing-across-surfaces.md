# ADR-0093: Obligation editing — status and due dates, across the API, CLI, MCP server, and UI

- **Status:** Accepted
- **Date:** 2026-08-19
- **Decider:** monk-eee
- **Approval:** "plus we should be able to edit via the api the cli or the mcp and the
  ui" — direct decider instruction, same standing pattern ADR-0074/ADR-0091/
  ADR-0095 already treated as approval for this kind of change.
- **Depends on:** [ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md)/[ADR-0007](0007-generalize-obligation-and-require-pgvector.md)
  (the event-sourced Obligation this ADR's edit still respects — appends an
  event, never patches the projection directly), [ADR-0020](0020-obligation-due-date-fields.md)
  (the due-date fields this ADR can now edit), [ADR-0038](0038-wire-up-audit-events-for-candidate-validation.md)
  (the atomic event+audit-row transaction pattern this ADR reuses), [ADR-0040](0040-dated-source-ingestion.md)
  (the CLI/MCP dual-surface precedent this ADR follows for a second command)
- **Tags:** backend, cli, mcp, frontend, obligations

## Context

An audit of every surface (prompted directly by the decider) found
Obligations were **100% read-only everywhere**: no HTTP route, no CLI
subcommand, no MCP tool, and no UI form could change an Obligation's status
or due dates after creation — the only way status ever changed was a
`status_changed` event appended internally by code that has since never
been wired to any caller-facing surface. `ObligationDetail.tsx` even said so
explicitly in its own doc comment: `"Read-only -- snooze/dismiss/
correct-owner are separate, future decisions."` This is the single largest
edit-capability gap in the app (Candidates already had `correct`/accept/
reject/promote; Nodes already had PATCH via API+MCP) and the entity most
central to the whole product.

The decider's ask was explicit about wanting edit parity across **all
four** surfaces, not just the UI. This ADR closes that gap for the one
entity that had literally nothing on any surface, using the exact
minimal edit that's actually meaningful for an Obligation: status
(open/at_risk/closed) and the two existing due-date fields. It does not
attempt to make every entity (Node, Candidate, edge) uniformly editable
everywhere in one pass — that remains a larger, separate effort if wanted.

## Decision

- Add `obligation::update_status()` (`backend/src/obligation.rs`): appends a
  `status_changed` event (the event log stays authoritative, exactly like
  every other write in this module — never patches `obligation_projection`
  directly), records an audit row in the same transaction (ADR-0038's
  pattern), then rebuilds the projection and returns the fresh row. Takes
  an `actor`/`source` pair so the audit trail honestly distinguishes which
  surface made the change.
  - `hard_due_at`/`soft_due_at` contract: `None` leaves the field
    unchanged (omitted from the event payload, so the projection rebuild
    carries the prior value forward); `Some("")` explicitly clears it (an
    empty string fails RFC3339 parsing — the same "present but
    unparseable clears it" contract `payload_timestamp` already uses
    internally); `Some(<rfc3339>)` sets it.
  - Rejects with `NoChange` if status and both due dates are all omitted —
    an edit that changes nothing is rejected, not silently accepted as a
    no-op event, matching ADR-0045's own guard on Candidate correction.
- **HTTP API:** `PATCH /api/obligations/:id` (`backend/src/api/obligations.rs`),
  calling `update_status` with `actor="local-operator"`, `source="http_api"`.
  `400` for an unrecognized status or a no-op request; `404` for an unknown
  id.
- **CLI:** a new `update-obligation` subcommand on the `ringmaster-ingest`
  binary (`--id`, `--status`, `--hard-due`, `--soft-due`), calling the
  identical `update_status` directly against `DATABASE_URL` with
  `source="cli"` — no running HTTP server required, matching the existing
  `ingest`/`reindex-embeddings` subcommands' own pattern.
- **MCP:** a new `update_obligation` tool (`backend/src/bin/ringmaster-ingest/mcp.rs`),
  same fields, calling the identical `update_status` with `source="mcp"`.
- **UI:** `ObligationDetail.tsx` gains an "Edit" button revealing a status
  `<select>` and two `<input type="date">` fields; Save only sends the
  fields that actually changed (via `updateObligation()` in `api.ts`,
  `PATCH`), then re-fetches the full detail (risk signals/health are
  derived from status and due dates, so a fresh read is required, not a
  local patch of stale derived fields).
- All four surfaces call the **same** `obligation::update_status` function.
  None can drift from the others' validation, event shape, or audit
  behavior.

## Scope

**In scope:** `backend/src/obligation.rs` (`update_status`,
`UpdateObligationError`), `backend/src/api/obligations.rs` (`PATCH` route),
`backend/src/api/mod.rs` (route wiring), `backend/src/bin/ringmaster-ingest/main.rs`
(`update-obligation` subcommand), `backend/src/bin/ringmaster-ingest/mcp.rs`
(`update_obligation` tool), `frontend/src/api.ts` (`updateObligation`),
`frontend/src/components/ObligationDetail.tsx` (edit form),
`frontend/public/style.css` (edit-form styling).

**Out of scope, named honestly:**

- **Editing an Obligation's `source_fragment_id`, or its evidence quote.**
  Evidence traceability (ADR-0023) stays exactly as ingested/promoted;
  this ADR edits only status and due dates.
- **Deleting or closing-with-a-reason.** `closed` is just another status
  value here; a richer "why was this closed" flow is a separate decision.
- **Node/Edge/Candidate edit-parity gaps.** Nodes already have API+MCP
  PATCH (UI lacks a `canonical_text`/`lifecycle_state` form); Candidates
  already have `correct` on API+UI (no MCP tool, no CLI). Bringing every
  entity to full four-surface parity in one ADR would be a much larger,
  unbounded change; this ADR closes the *worst* gap (Obligations had
  nothing) and leaves the others as named, tracked follow-ups.
- **Bulk obligation editing** (the batch pattern ADR-0076/ADR-0077 gave
  Candidates). Single-obligation edit only, matching the scope actually
  requested.
- **Any new obligation_events event type.** Reuses the existing
  `status_changed` type (ADR-0005/ADR-0007) unchanged.

## Options considered

- **One shared function, four thin callers (chosen):** the CLI/MCP dual
  surface precedent (`ingest_source`, ADR-0040) already established this
  pattern for this exact binary; extending it to a fourth caller (the HTTP
  route) and a UI form on top is the natural, lowest-risk way to guarantee
  the four surfaces cannot drift.
- **A generic "PATCH any Obligation field" endpoint** (mirroring Node's
  open-ended attribute merge): rejected — an Obligation's fields
  (`source_fragment_id`, evidence) are deliberately immutable per ADR-0023;
  a generic merge would make it too easy to silently corrupt evidence
  provenance. Naming exactly which fields are editable (status, due dates)
  keeps the invariant explicit and enforced by the type system, not just
  convention.
- **Full four-surface parity for every entity in one ADR:** rejected as
  disproportionate for one message's worth of work — scoped instead to the
  entity with the actual zero-coverage gap, with the others explicitly
  named as follow-ups above.

## Consequences

- **Positive:** an Obligation can now be corrected/closed/re-dated from
  whichever surface is convenient — a script (CLI), an agent (MCP), a
  direct API call, or the UI — with one shared, audited code path.
- **Positive:** the audit trail (`audit_events`) now honestly distinguishes
  `http_api`/`cli`/`mcp` as the source of an obligation update, extending
  the same provenance discipline ADR-0038 gave candidate corrections.
- **Negative / trade-off:** `hard_due_at`/`soft_due_at`'s "empty string
  clears it" contract is a slightly unusual API shape (not a nullable
  JSON field); chosen to exactly match the existing internal
  `payload_timestamp` contract rather than introduce a second, different
  absent-vs-null convention.
- **Risk:** low-medium. New write path on the core aggregate, mitigated by
  reusing the identical transaction+audit+rebuild pattern every other
  Obligation/Candidate write already uses, and by every one of the four
  callers sharing one function rather than four independent
  implementations.

## Exit criteria and evidence

Evidence: [EV-0093](../evidence.d/0093-obligation-editing-across-surfaces.md)

| Exit criterion | Evidence |
|---|---|
| `obligation::update_status` exists and is called by all four surfaces | `update-status-shared-by-four-surfaces` |
| `PATCH /api/obligations/:id` is wired and rejects an unrecognized status | `http-patch-route-wired` |
| `update-obligation` CLI subcommand exists | `cli-subcommand-wired` |
| `update_obligation` MCP tool exists | `mcp-tool-wired` |
| `ObligationDetail.tsx` has a working edit form calling the PATCH route | `ui-edit-form-wired` |
| Backend compiles cleanly and the Playwright suite passes | `builds-and-tests-pass` |
