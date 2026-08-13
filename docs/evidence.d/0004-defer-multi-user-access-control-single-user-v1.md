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
# No ingestion, sync, telemetry, or sharing implementation exists yet.
# Re-check as an automated check once such code exists.

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
