# EV-0054: Congruence Engine v1 — flag a commitment with no linked node at all

Evidence for [ADR-0054](../adr.d/0054-congruence-engine-v1-isolated-commitment-signal.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0054-congruence-engine-v1-isolated-commitment-signal"

[[check]]
id = "isolated-signal-flags-a-zero-edge-commitment"
invariant = "An Obligation with zero edges is flagged with an isolated risk signal."
type = "present"
pattern = 'fn risk_signals_flags_isolated_when_has_edges_is_false'
paths = ["backend/src/api.rs"]

[[check]]
id = "isolated-signal-does-not-flag-a-linked-commitment"
invariant = "An Obligation with at least one edge is not flagged isolated."
type = "present"
pattern = 'fn risk_signals_does_not_flag_isolated_when_has_edges_is_true'
paths = ["backend/src/api.rs"]

[[check]]
id = "isolated-signal-attached-like-existing-signals"
invariant = "isolated appears in risk_signals on Daily Brief, Time Horizon, and Obligation detail, reusing the existing signal attachment pattern."
type = "present"
pattern = 'risk_signals\(hard_due_at, soft_due_at, updated_at, source_fragment_id, has_owner, has_edges\)'
paths = ["backend/src/api.rs"]
```

## Notes

Implemented: `risk_signals` gained a `has_edges` parameter, computed by
each of its three callers (`GET /api/daily-brief`, `GET /api/time-horizon`,
`GET /api/obligations/:id`) via `EXISTS (SELECT 1 FROM edges WHERE
from_id = op.obligation_id OR to_id = op.obligation_id)`, the same pattern
already used for `has_owner`. Scoped to any non-closed Obligation, not
strictly a "commitment"-type one -- `ObligationProjection` carries no
candidate-type distinction once promoted, and requiring one would need an
extra join against `candidate_projection` for uncertain benefit; flagging
any zero-edge Obligation is the honest, simpler generalization of the
same principle the ADR names.
