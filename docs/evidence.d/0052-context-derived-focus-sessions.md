# EV-0052: Context-derived Focus Sessions — group by shared node *and* similar timeframe, not node alone

Evidence for [ADR-0052](../adr.d/0052-context-derived-focus-sessions.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0052-context-derived-focus-sessions"

[[check]]
id = "focus-blocks-split-by-time-horizon-bucket"
invariant = "A shared node's Obligations spanning two Time Horizon buckets form two separate blocks."
type = "present"
pattern = 'fn focus_blocks_route_splits_by_time_horizon_bucket'
paths = ["backend/src/api.rs"]

[[check]]
id = "focus-blocks-single-bucket-unchanged"
invariant = "A shared node's Obligations all in one bucket still form exactly one block."
type = "present"
pattern = 'fn focus_blocks_route_groups_by_shared_node'
paths = ["backend/src/api.rs"]

[[check]]
id = "focus-block-label-names-node-and-bucket"
invariant = "Each block's response includes both the node's canonical_text and its Time Horizon bucket."
type = "present"
pattern = '"time_horizon_bucket": block\.bucket'
paths = ["backend/src/api.rs"]
```

## Notes

Implemented: `focus_blocks` now groups by `(node_id, time_horizon_bucket)`
instead of `node_id` alone, reusing `time_horizon_bucket` verbatim. Each
block's JSON gains a `time_horizon_bucket` field. Ordering changed from
pure Obligation-count descending to urgency-first (ADR-0050): any block
containing an `at_risk` Obligation sorts first, then soonest effective due
date among its Obligations, then count descending as a final tiebreak.
`focus_blocks_route_groups_by_shared_node`'s fixture was updated to give
both Obligations the same `hard_due_at` so they land in the same bucket,
matching what it always intended to test.
