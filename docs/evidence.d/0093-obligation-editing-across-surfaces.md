# EV-0093: Obligation editing — status and due dates, across the API, CLI, MCP server, and UI

Evidence for [ADR-0093](../adr.d/0093-obligation-editing-across-surfaces.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0093-obligation-editing-across-surfaces"

[[check]]
id = "update-status-shared-by-four-surfaces"
invariant = "obligation::update_status exists and is the single function all four surfaces call."
type = "present"
pattern = "pub async fn update_status"
paths = ["backend/src/obligation.rs"]

[[check]]
id = "http-patch-route-wired"
invariant = "PATCH /api/obligations/:id is wired to update_obligation, which calls obligation::update_status."
type = "present"
pattern = 'get\(get_obligation_detail\)\.patch\(update_obligation\)'
paths = ["backend/src/api/mod.rs"]

[[check]]
id = "cli-subcommand-wired"
invariant = "The ringmaster-ingest CLI has an update-obligation subcommand calling obligation::update_status."
type = "present"
pattern = '"update-obligation"'
paths = ["backend/src/bin/ringmaster-ingest/main.rs"]

[[check]]
id = "mcp-tool-wired"
invariant = "The MCP server exposes an update_obligation tool calling obligation::update_status."
type = "present"
pattern = "async fn update_obligation"
paths = ["backend/src/bin/ringmaster-ingest/mcp.rs"]

[[check]]
id = "ui-edit-form-wired"
invariant = "ObligationDetail.tsx has a working edit form calling updateObligation (PATCH)."
type = "present"
pattern = "updateObligation"
paths = ["frontend/src/components/ObligationDetail.tsx", "frontend/src/api.ts"]

[[check]]
id = "builds-and-tests-pass"
invariant = "Backend compiles cleanly and the Playwright suite passes against the new edit surfaces."
type = "manual"
last_verified = "2026-08-19"
rationale = "`cargo build` succeeded with zero errors/warnings after adding update_status, the PATCH route, the CLI subcommand, and the MCP tool. `npx playwright test --project=chromium` run against the app with the new ObligationDetail edit form; existing specs passed unchanged (no text/DOM contract they assert on was touched)."
```
