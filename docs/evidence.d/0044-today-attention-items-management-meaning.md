# EV-0044: Today attention items show management meaning, not identifiers

Evidence for [ADR-0044](../adr.d/0044-today-attention-items-management-meaning.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0044-today-attention-items-management-meaning"

[[check]]
id = "daily-brief-returns-source-text"
invariant = "GET /api/daily-brief returns the already-selected source_text on each item, matching GET /api/obligations."
type = "manual"
notes = "Deferred one-line backend addition: the daily_brief function is under concurrent edit (ADR-0046 unowned-owner signal touches the same SQL query and .map closure). The frontend already consumes item.source_text (null-safe, honest status-label fallback), so the title auto-upgrades to the evidence quote the moment this field is returned. Flip to a present-type check ('\"source_text\"' in backend/src/api.rs's daily_brief) once that concurrent edit lands."

[[check]]
id = "today-row-hides-raw-id"
invariant = "The Today attention row renders no raw obligation identifier (progressive disclosure)."
type = "absent"
pattern = 'id-marker'
paths = ["frontend/src/components/DailyBrief.tsx"]

[[check]]
id = "today-row-shows-human-date-or-honest-empty"
invariant = "Each Today row renders a deterministic human date/horizon phrase, or an honest 'No date recorded' when both dates are null."
type = "present"
pattern = 'No date recorded'
paths = ["frontend/src/components/DailyBrief.tsx"]

[[check]]
id = "today-row-shows-evidence-status"
invariant = "Each Today row renders an explicit evidence-status indicator derived from source_fragment_id (recorded vs. none)."
type = "present"
pattern = 'No evidence recorded'
paths = ["frontend/src/components/DailyBrief.tsx"]
```

## Notes

Three of four checks are automated against the implementing component
(`DailyBrief.tsx`): the raw-id chip is gone (`absent` check on `id-marker`),
and each row now renders a human due-date phrase and an explicit
evidence-status line. The fourth check, exposing `source_text` on
`GET /api/daily-brief`, is a deliberately-`manual`, honestly-deferred
one-liner: the `daily_brief` function was under concurrent edit (the
unowned-owner risk signal modifies the same SQL query and result closure)
when this shipped, so the collision-free path was to land the frontend now
— which already consumes `source_text` null-safely, falling back to an
honest status label — and add the backend field once that edit lands.
