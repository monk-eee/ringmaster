# EV-0052: Context-derived Focus Sessions — group by shared node *and* similar timeframe, not node alone

Evidence for [ADR-0052](../adr.d/0052-context-derived-focus-sessions.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0052-context-derived-focus-sessions"

[[check]]
id = "focus-blocks-split-by-time-horizon-bucket"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once a shared node's Obligations spanning two Time Horizon buckets form two separate blocks."

[[check]]
id = "focus-blocks-single-bucket-unchanged"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once a shared node's Obligations all in one bucket still form exactly one block, matching today's behavior."

[[check]]
id = "focus-block-label-names-node-and-bucket"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once each block's label includes both the node's canonical_text and its Time Horizon bucket."
```

## Notes

Pre-implementation: all three checks are deliberately `manual`/unproven,
per this repo's own convention. Do not implement before
[ADR-0052](../adr.d/0052-context-derived-focus-sessions.md)'s Status flips
to Accepted.
