# EV-0004: Defer multi-user access control; keep sensitive commitment data local and unshared for v1

Evidence for [ADR-0004](../adr.d/0004-defer-multi-user-access-control-single-user-v1.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0004-defer-multi-user-access-control-single-user-v1"

[[check]]
id = "vision-names-sensitive-data-question"
invariant = "The vision document names the sensitive-data boundary as an open question this ADR addresses."
type = "present"
pattern = 'Sensitive data boundary'
paths = ["docs/VISION.md"]

[[check]]
id = "no-sensitive-data-sharing-path"
invariant = "No implementation syncs, exports, or shares People-commitment content outside the single local operator."
type = "manual"
last_verified = "2026-08-18"
rationale = "Re-affirmed after ADR-0066 through ADR-0079 landed: every provider surface is still local-only (HTTP on localhost, stdio MCP, local Postgres); no sync, export, cloud telemetry, shared dashboard, second-user session, or outbound People-commitment path exists. The optional LLM/embedding adapters send only the explicit extraction/embedding prompt or fragment supplied by the caller and remain governed by their own hosted-first API-key ADRs; they do not enumerate, sync, or export the commitment store. Live Outlook/Teams/Calendar/SharePoint connectors remain explicitly deferred in docs/current-status.md pending an access-control ADR. This invariant is a policy/data-flow claim rather than a single literal a regex could honestly prove; keep it manual and re-check whenever a connector, shared deployment, telemetry, or second operator is proposed."

[[check]]
id = "no-second-account-access"
invariant = "No second human or service account has read or write access to the commitment store."
type = "absent"
pattern = 'CREATE ROLE|CREATE USER|GRANT\s'
paths = ["backend/migrations/**", "compose.yaml", ".env.example"]
```

## Notes

`no-second-account-access` is now a declarative `absent` check: the
migrations, Compose file, and env template that define database access all
provision exactly one Postgres account, and none creates a second role,
user, or grant. `no-sensitive-data-sharing-path` remains intentionally
`manual`: no sync, export, or telemetry implementation exists yet, and the
invariant is about data flow rather than any single dependency or literal a
regex could honestly stand in for.
