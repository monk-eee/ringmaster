# EV-0058: Extract a due date from a candidate and carry it to the promoted obligation

Evidence for [ADR-0058](../adr.d/0058-extract-candidate-due-date-to-obligation.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0058-extract-candidate-due-date-to-obligation"

[[check]]
id = "extraction-captures-a-due-date"
invariant = "Extraction requests and persists an optional due_at in the extracted event payload."
type = "present"
pattern = "due_at"
paths = ["backend/src/extraction.rs"]

[[check]]
id = "promote-carries-due-date-to-soft-due-at"
invariant = "promote_candidate carries a candidate's extracted due_at into the Obligation created event as soft_due_at."
type = "present"
pattern = "candidate_extracted_due_at"
paths = ["backend/src/api/candidates.rs"]

[[check]]
id = "due-date-carry-is-tested"
invariant = "A deterministic test proves the extracted due_at reaches the promoted Obligation's soft_due_at."
type = "present"
pattern = "promote_carries_extracted_due_date_into_soft_due_at"
paths = ["backend/src/api/candidates.rs"]

[[check]]
id = "backend-suite-passes-with-due-date-carry"
invariant = "The full backend suite passes against ringmaster_test with the due-date carry in place."
type = "manual"
last_verified = "2026-08-17"
rationale = "A live test run is not a file-content regex. Verified directly: ran the full backend suite via podman against ringmaster_test with --test-threads=1; all tests passed, including promote_carries_extracted_due_date_into_soft_due_at, which appends an extracted event carrying a due_at, promotes the candidate, and asserts the resulting obligation_projection row has that exact soft_due_at."
```

## Notes

This ADR amends [ADR-0027](../adr.d/0027-promote-accepted-candidate-to-obligation.md):
0027 promoted a candidate carrying only its `source_fragment_id` forward and
stated "no due date is implied by a candidate"; this record adds a soft due
date inherited from extraction. It introduces no migration — `due_at` lives in
the existing `extracted` `candidate_events` JSON payload, and
`obligation::rebuild_projection` already carries `soft_due_at` forward from a
`created` event ([ADR-0020](../adr.d/0020-obligation-due-date-fields.md)).
