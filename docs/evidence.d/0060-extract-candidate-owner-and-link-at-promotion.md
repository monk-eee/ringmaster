# EV-0060: Extract an owner name from a candidate and link it at promotion

Evidence for [ADR-0060](../adr.d/0060-extract-candidate-owner-and-link-at-promotion.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0060-extract-candidate-owner-and-link-at-promotion"

[[check]]
id = "owner-name-parsed-defensively"
invariant = "Extraction requests and persists an optional owner_name in the extracted event payload, degrading to None on anything malformed or absent."
type = "present"
pattern = 'parsed\.get\("owner_name"\)'
paths = ["backend/src/extraction.rs"]

[[check]]
id = "promotion-creates-owns-edge-on-exact-match"
invariant = "Promoting a candidate whose owner_name exactly matches an existing Person node creates an owns edge in the same transaction."
type = "present"
pattern = "owner_person_id"
paths = ["backend/src/api.rs"]

[[check]]
id = "promotion-unchanged-without-a-match"
invariant = "Promoting a candidate with no owner_name, or one matching no existing Person, promotes exactly as before -- no new Person node, no edge."
type = "present"
pattern = "promotion_creates_no_owns_edge_without_an_exact_match"
paths = ["backend/src/api.rs"]

[[check]]
id = "backend-suite-passes-with-owner-extraction"
invariant = "The full backend suite passes against ringmaster_test with owner extraction and promotion-time linking in place."
type = "manual"
last_verified = "2026-08-17"
rationale = "A live test run is not a file-content regex. Verified directly: ran the full backend suite via podman against ringmaster_test with --test-threads=1; 136 passed, 0 failed, including promotion_creates_owns_edge_on_exact_owner_match and promotion_creates_no_owns_edge_without_an_exact_match."
```

## Notes

Implemented: `extract_candidate_with_due_at` gained a ninth parameter,
`owner_name: Option<&str>`, stored in the same `extracted` event payload as
`due_at` -- no migration, mirroring ADR-0058's exact pattern. `promote_candidate`
resolves it against `nodes` via `lower(canonical_text) = lower($1)`, creating
an `owns` edge (`graph::create_edge`, widened to a generic executor per
ADR-0038's own precedent) in the same transaction as the Obligation, only on
an exact match. Found and fixed one genuine test-isolation bug while
implementing this: the first version of `promotion_creates_owns_edge_on_exact_owner_match`
used a fixed literal person name ("Roopa Venkat"), which, once the test had
run more than once against the long-lived `ringmaster_test` database, left
multiple identically-named Person nodes -- the promotion route's unordered
`LIMIT 1` lookup then non-deterministically resolved to an older run's node
instead of this run's, failing the assertion despite the feature itself
working correctly. Fixed by giving the test a per-run-unique name (a fresh
UUID suffix), the same pattern already used elsewhere in this repo's
Playwright suite for the identical reason.
