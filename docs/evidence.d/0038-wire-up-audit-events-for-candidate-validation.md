# EV-0038: Wire up audit_events for candidate validation actions

Evidence for [ADR-0038](../adr.d/0038-wire-up-audit-events-for-candidate-validation.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0038-wire-up-audit-events-for-candidate-validation"

[[check]]
id = "accept-writes-audit-row"
invariant = "Accepting a candidate writes an immutable audit row in the same transaction as the state change."
type = "present"
pattern = 'accept_route_writes_an_audit_row_with_the_honest_placeholder_actor'
paths = ["backend/src/api.rs"]

[[check]]
id = "reject-writes-audit-row"
invariant = "Rejecting a candidate writes an immutable audit row in the same transaction as the state change."
type = "present"
pattern = 'reject_route_writes_an_audit_row'
paths = ["backend/src/api.rs"]

[[check]]
id = "promote-writes-audit-row"
invariant = "Promoting a candidate writes an immutable audit row in the same transaction as the state change."
type = "present"
pattern = 'promote_route_writes_an_audit_row'
paths = ["backend/src/api.rs"]

[[check]]
id = "actor-is-honest-placeholder"
invariant = "The recorded actor is the literal local-operator placeholder, not a fabricated per-request identity."
type = "present"
pattern = '"local-operator"'
paths = ["backend/src/api.rs"]
```

## Notes

All four checks are automated against the implementing tests, which assert
the audit row exists (via a before/after count delta, safe against the
shared, ever-growing development database) and that `actor` is exactly
`"local-operator"`. `cargo test` (89/89) passes, including all three new
audit tests, run serially against a live Postgres instance.
