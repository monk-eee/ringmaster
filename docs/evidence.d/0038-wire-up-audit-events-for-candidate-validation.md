# EV-0038: Wire up audit_events for candidate validation actions

Evidence for [ADR-0038](../adr.d/0038-wire-up-audit-events-for-candidate-validation.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0038-wire-up-audit-events-for-candidate-validation"

[[check]]
id = "accept-writes-audit-row"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once accept_candidate writes an audit_events row atomically with its state change."

[[check]]
id = "reject-writes-audit-row"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once reject_candidate writes an audit_events row atomically with its state change."

[[check]]
id = "promote-writes-audit-row"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once promote_candidate writes an audit_events row atomically with its state change."

[[check]]
id = "actor-is-honest-placeholder"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once the recorded actor is the literal local-operator placeholder, not a fabricated per-request identity."
```

## Notes

Pre-implementation: all four checks are deliberately `manual`/unproven, per
this repo's own convention (evidence stays honest about intent vs. proof
until the ADR is accepted and implemented). Do not implement before
[ADR-0038](../adr.d/0038-wire-up-audit-events-for-candidate-validation.md)'s
Status flips to Accepted.
