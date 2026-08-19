# ADR-0097: Candidate accept/reject via MCP, and a Node identity/lifecycle edit form in the UI

- **Status:** Accepted
- **Date:** 2026-08-20
- **Decider:** monk-eee
- **Approval:** Direct instruction ("ok fix gaps"), following the gap audit
  from the prior session ("look for gaps") that named these two items as
  explicit follow-ups in [ADR-0093](0093-obligation-editing-across-surfaces.md)'s
  own "Out of scope" section — the same standing pattern this session has
  used for every self-initiated fix since ADR-0091.
- **Depends on:** [ADR-0024](0024-candidate-accept-reject-buttons.md) (the
  accept/reject transition this ADR exposes over MCP), [ADR-0093](0093-obligation-editing-across-surfaces.md)
  (the shared-function-per-surface pattern this ADR reuses), [ADR-0025](0025-node-edge-write-api-and-traversal.md)
  (the `canonical_text`/`lifecycle_state` PATCH fields this ADR finally
  exposes in the UI)
- **Tags:** backend, mcp, frontend

## Context

Two named, bounded gaps remained after ADR-0093 closed Obligations' total
edit-surface gap:

1. **Candidates had zero MCP presence.** `backend/src/api/candidates.rs`
   exposed accept/reject/correct/promote over HTTP+UI only. An agent
   working through the MCP server (this repo's own stated integration
   surface, ADR-0003/ADR-0066) could ingest sources and read candidates,
   but could not triage a single one — every accept/reject required a
   human clicking the Inbox.
2. **Nodes had no identity/lifecycle edit form.** `PATCH /api/nodes/:id`
   and the `update_entity`/`upsert_entities` MCP tools have accepted
   `canonical_text` and `lifecycle_state` since ADR-0025/ADR-0066, but
   Graph Explorer's only edit affordance was an "Enrich attributes (JSON)"
   textarea — renaming a wrongly-typed node or marking one archived
   required a raw API call, never a UI action.

This ADR closes the higher-value half of each gap without attempting full
four-surface parity for every entity in one pass (that remains
deliberately out of scope, as ADR-0093 already named).

## Decision

- **`extraction::transition_candidate_state`** (new, `backend/src/extraction.rs`):
  the state-check-then-transition-then-audit-then-rebuild-then-refetch
  logic `api/candidates.rs`'s private `transition_one` used to duplicate,
  now shared — mirroring `obligation::update_status`'s own precedent
  exactly (ADR-0093). `api/candidates.rs`'s `transition_one` is refactored
  to call it instead of reimplementing the same steps.
- **MCP tools `accept_candidate`/`reject_candidate`**
  (`backend/src/bin/ringmaster-ingest/mcp.rs`): call the identical shared
  function with `source="mcp"`, so the audit trail honestly distinguishes
  an MCP-driven triage action from an HTTP one, matching ADR-0093's own
  `actor`/`source` provenance discipline.
- **Graph Explorer's enrich form** (`frontend/src/components/GraphExplorer.tsx`):
  gains **Name** and **Lifecycle state** text fields alongside the
  existing attributes textarea, pre-filled from the selected node and
  reset whenever a different node is focused. Submitting only sends the
  fields that actually changed from the currently-loaded values — an
  unfocused/untouched field never overwrites the node with a stale echo
  of what was already there.

## Scope

**In scope:** `backend/src/extraction.rs` (`transition_candidate_state`,
`TransitionCandidateError`), `backend/src/api/candidates.rs`
(`transition_one` refactored to delegate), `backend/src/bin/ringmaster-ingest/mcp.rs`
(two new tools + a shared `transition_candidate_tool` helper),
`frontend/src/components/GraphExplorer.tsx` (Name/Lifecycle state
fields), `frontend/src/api.ts` (unchanged — `updateNode` already accepted
these fields), `frontend/public/style.css` (`.field-input` styling for
the new text inputs).

**Out of scope, named honestly:**

- **Candidate `correct`/`promote` via MCP, and any candidate action via
  CLI.** Accept/reject are the two simplest, most common agent-triage
  actions and the ones actually named in the gap audit; `correct` (dual
  optional fields, its own no-op guard) and `promote` (due-date/owner
  carry-forward) are more involved and are a separate, still-real
  follow-up if agent-driven correction/promotion is wanted. CLI parity for
  any candidate action was judged lower value than MCP (an agent calling
  a tool is a far more realistic caller than a human shelling out to
  approve one candidate at a time) and is not attempted here.
- **Bulk candidate MCP/CLI actions.** Single-candidate only, matching
  ADR-0076/ADR-0077's own UI-only bulk scope — no MCP/CLI bulk surface
  exists for anything yet, and this ADR does not start one.
- **A node-type-specific `lifecycle_state` enumeration or validation.**
  Remains free text exactly as ADR-0025 decided; this ADR adds a UI
  affordance for the existing field, not a new constraint on its values.
- **Editing `node_type` itself.** Immutable per ADR-0025's own decision;
  unchanged here.

## Options considered

- **Share one function per surface (chosen):** the exact pattern
  ADR-0093 already established and proved out for Obligations; extending
  it to Candidates keeps the codebase's edit-surface story consistent
  rather than inventing a second pattern.
- **A single generic MCP `transition_candidate` tool taking the target
  state as a parameter:** rejected — `accept_candidate`/`reject_candidate`
  as two distinctly-named, distinctly-described tools is clearer for a
  calling agent than a `state: "accepted" | "rejected"` parameter on one
  tool, and matches how `create_entity`/`update_entity` are already named
  as separate tools rather than one verb-parameterized tool.
- **A full identity-edit modal/dedicated form separate from the existing
  enrich form:** rejected as unnecessary complexity — extending the one
  existing form with two more fields, submitting only what changed, is a
  smaller diff with the same result.

## Consequences

- **Positive:** an MCP-driven agent can now triage candidates (the most
  common Inbox action) without a human in the loop for the simple
  accept/reject case, and a human can now fix a node's name or mark it
  archived without leaving the UI.
- **Positive:** the audit trail gains `mcp` as a `source` value for
  candidate transitions, extending the same provenance discipline
  ADR-0093 established for Obligation edits.
- **Negative / trade-off:** MCP-driven triage still stops at accept/reject
  — an agent wanting to correct or promote a candidate still needs the UI
  or a direct HTTP call, named honestly above as a real, separate gap.
- **Risk:** low. The Candidate refactor is a pure extraction (identical
  behavior, now shared) validated by the existing HTTP test suite; the two
  MCP tools and the UI form are additive with no changed routes or
  response shapes.

## Exit criteria and evidence

Evidence: [EV-0097](../evidence.d/0097-candidate-mcp-and-node-edit-form.md)

| Exit criterion | Evidence |
|---|---|
| `extraction::transition_candidate_state` exists and `transition_one` delegates to it | `shared-transition-function-exists` |
| MCP `accept_candidate`/`reject_candidate` tools call the shared function | `mcp-tools-call-shared-function` |
| Graph Explorer's enrich form has Name and Lifecycle state fields | `node-edit-form-has-identity-fields` |
| Only changed fields are sent on submit (no stale-value clobber) | `enrich-form-sends-only-changed-fields` |
| Backend compiles cleanly and the Playwright suite passes | `builds-and-tests-pass` |
