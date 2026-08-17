# EV-0060: Extract an owner name from a candidate and link it at promotion

Evidence for [ADR-0060](../adr.d/0060-extract-candidate-owner-and-link-at-promotion.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0060-extract-candidate-owner-and-link-at-promotion"

[[check]]
id = "owner-name-parsed-defensively"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once extract_candidate_via_model parses an optional owner_name, degrading to None on anything malformed or absent."

[[check]]
id = "promotion-creates-owns-edge-on-exact-match"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "Will become a present-type check once promoting a candidate whose owner_name exactly matches an existing Person node creates an owns edge in the same transaction."

[[check]]
id = "promotion-unchanged-without-a-match"
invariant = "Not yet implemented -- awaiting acceptance."
type = "manual"
notes = "A negative/unchanged-behavior claim; verified by direct test run once implemented, matching EV-0058's own precedent for this kind of claim."
```

## Notes

Pre-implementation: all three checks are deliberately `manual`/unproven,
per this repo's own convention. Do not implement before
[ADR-0060](../adr.d/0060-extract-candidate-owner-and-link-at-promotion.md)'s
Status flips to Accepted.
