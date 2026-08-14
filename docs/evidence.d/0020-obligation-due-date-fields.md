# EV-0020: Add due-date fields to Obligation, the schema prerequisite for Epic E7

Evidence for [ADR-0020](../adr.d/0020-obligation-due-date-fields.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0020-obligation-due-date-fields"

[[check]]
id = "due-date-columns-exist"
invariant = "obligation_projection carries nullable hard_due_at/soft_due_at columns."
type = "present"
pattern = 'ADD COLUMN hard_due_at'
paths = ["backend/migrations/0009_obligation_due_dates.sql"]

[[check]]
id = "rebuild-preserves-due-dates"
invariant = "rebuild_projection carries a previously-recorded due date forward across a later event that doesn't name it."
type = "present"
pattern = 'fn payload_timestamp'
paths = ["backend/src/obligation.rs"]

[[check]]
id = "obligations-route-includes-due-dates"
invariant = "GET /api/obligations includes hard_due_at and soft_due_at for each row."
type = "present"
pattern = '"hard_due_at"'
paths = ["backend/src/api.rs"]
```

## Notes

All three checks are automated against the migration, projection-rebuild
logic, and API module that implement them. `cargo test` cases exercise:
projection rebuild carrying a `hard_due_at` forward across a
`status_changed` event that names no due date, and `GET /api/obligations`
returning the field as an RFC 3339 string.
