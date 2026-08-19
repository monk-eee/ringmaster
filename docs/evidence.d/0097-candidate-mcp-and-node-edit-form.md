# EV-0097: Candidate accept/reject via MCP, and a Node identity/lifecycle edit form in the UI

Evidence for [ADR-0097](../adr.d/0097-candidate-mcp-and-node-edit-form.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0097-candidate-mcp-and-node-edit-form"

[[check]]
id = "shared-transition-function-exists"
invariant = "extraction::transition_candidate_state exists and transition_one delegates to it instead of duplicating the logic."
type = "present"
pattern = "pub async fn transition_candidate_state"
paths = ["backend/src/extraction.rs"]

[[check]]
id = "mcp-tools-call-shared-function"
invariant = "The accept_candidate/reject_candidate MCP tools call extraction::transition_candidate_state."
type = "present"
pattern = "transition_candidate_tool|transition_candidate_state"
paths = ["backend/src/bin/ringmaster-ingest/mcp.rs"]

[[check]]
id = "node-edit-form-has-identity-fields"
invariant = "Graph Explorer's enrich form has Name and Lifecycle state input fields."
type = "present"
pattern = "enrichCanonicalText|enrichLifecycleState"
paths = ["frontend/src/components/GraphExplorer.tsx"]

[[check]]
id = "enrich-form-sends-only-changed-fields"
invariant = "Submitting the enrich form only sends fields that actually changed from the loaded node."
type = "present"
pattern = 'enrichCanonicalText !== detail\.canonical_text'
paths = ["frontend/src/components/GraphExplorer.tsx"]

[[check]]
id = "builds-and-tests-pass"
invariant = "Backend compiles cleanly and the Playwright suite passes against the new MCP tools and edit form."
type = "manual"
last_verified = "2026-08-20"
rationale = "`cargo build` succeeded with zero errors after adding transition_candidate_state, the two MCP tools, and refactoring transition_one. A new Rust unit test (accept_via_shared_function_matches_existing_http_behavior) passed via the required MCP test runner. `npx playwright test --project=chromium` run against the app with the extended enrich form; existing specs passed unchanged."
```
