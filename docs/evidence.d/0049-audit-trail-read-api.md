# EV-0049: Audit trail read API — a chronological activity feed

Evidence for [ADR-0049](../adr.d/0049-audit-trail-read-api.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0049-audit-trail-read-api"

[[check]]
id = "audit-recent-function-exists"
invariant = "audit::recent reads recent audit_events rows, ordered newest first."
type = "present"
pattern = 'pub async fn recent'
paths = ["backend/src/audit.rs"]

[[check]]
id = "audit-events-route-exists"
invariant = "GET /api/audit-events exists."
type = "present"
pattern = '"/api/audit-events"'
paths = ["backend/src/api/mod.rs"]

[[check]]
id = "limit-is-clamped-not-rejected"
invariant = "An out-of-range limit is clamped, not rejected with an error."
type = "present"
pattern = "clamp"
paths = ["backend/src/audit.rs"]

[[check]]
id = "frontend-activity-tab-exists"
invariant = "An Activity tab renders the feed."
type = "present"
pattern = "export default function Activity"
paths = ["frontend/src/components/Activity.tsx"]

[[check]]
id = "playwright-proves-activity-feed-shows-real-data"
invariant = "Focused browser coverage proves a just-recorded audit row appears in the Activity tab."
type = "present"
pattern = 'ADR-0049'
paths = ["frontend/tests/obligations.spec.ts"]
```

## Notes

`cargo test` covers: `audit::recent` returns rows newest-first; a `limit`
above 200 is clamped to 200; a `limit` below 1 is clamped to 1 (default 50
when omitted); the route surfaces real rows written by an actual candidate
transition in the same test, found by a unique marker in `new_state`
rather than an aggregate count. `tsc --noEmit` and `vite build` pass.
