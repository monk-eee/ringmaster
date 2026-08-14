# EV-0020: Add due-date fields to Obligation, the schema prerequisite for Epic E7

Evidence for [ADR-0020](../adr.d/0020-obligation-due-date-fields.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0020-obligation-due-date-fields"

[[check]]
id = "due-date-columns-exist"
invariant = "obligation_projection carries nullable hard_due_at/soft_due_at columns."
type = "manual"

[[check]]
id = "rebuild-preserves-due-dates"
invariant = "rebuild_projection carries a previously-recorded due date forward across a later event that doesn't name it."
type = "manual"

[[check]]
id = "obligations-route-includes-due-dates"
invariant = "GET /api/obligations includes hard_due_at and soft_due_at for each row."
type = "manual"
```

## Notes

All three checks are `manual` and unverified (`ASSERTED`) because ADR-0020
is **Proposed**, not yet accepted or implemented. Once accepted, replace
each with a `present` pattern check against the implementing migration and
module, mirroring EV-0015's shape.
