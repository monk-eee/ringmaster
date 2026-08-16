# EV-0044: Today attention items show management meaning, not identifiers

Evidence for [ADR-0044](../adr.d/0044-today-attention-items-management-meaning.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0044-today-attention-items-management-meaning"

[[check]]
id = "daily-brief-returns-source-text"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once GET /api/daily-brief returns the already-selected source_text on each item, matching GET /api/obligations."

[[check]]
id = "today-row-hides-raw-id"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once the Today attention row no longer renders a raw obligation_id chip (progressive disclosure)."

[[check]]
id = "today-row-shows-human-date-or-honest-empty"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once each Today row renders a deterministic human date/horizon phrase from hard_due_at/soft_due_at, or an honest 'No date recorded' when both are null."

[[check]]
id = "today-row-shows-evidence-status"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once each Today row renders an explicit evidence-status indicator derived from source_fragment_id (recorded vs. none)."
```

## Notes

Pre-implementation: all four checks are deliberately `manual`/unproven, per
this repo's own convention (evidence stays honest about intent vs. proof
until the ADR is accepted and implemented). Do not implement before
[ADR-0044](../adr.d/0044-today-attention-items-management-meaning.md)'s
Status flips to Accepted.
