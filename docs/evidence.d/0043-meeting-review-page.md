# EV-0043: Meeting Review page — transcript fragments with inline extracted candidates

Evidence for [ADR-0043](../adr.d/0043-meeting-review-page.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0043-meeting-review-page"

[[check]]
id = "meetings-tab-exists"
invariant = "A Meetings tab exists in the secondary/developer group."
type = "present"
pattern = 'meetings'
paths = ["frontend/src/App.tsx"]

[[check]]
id = "meeting-detail-renders-fragments-with-candidates"
invariant = "Selecting a meeting renders its fragments with any already-extracted candidates inline."
type = "present"
pattern = 'fetchMeetingCandidates'
paths = ["frontend/src/components/MeetingReview.tsx"]

[[check]]
id = "meeting-review-reuses-existing-validation-actions"
invariant = "Accept/reject/promote on a candidate in this view call the existing routes and refetch."
type = "present"
pattern = 'acceptCandidate|rejectCandidate|promoteCandidate'
paths = ["frontend/src/components/MeetingReview.tsx"]

[[check]]
id = "meeting-review-wires-up-extraction-trigger"
invariant = "A fragment with no candidates offers an Extract action calling the existing per-fragment trigger."
type = "present"
pattern = 'extractSourceFragment'
paths = ["frontend/src/components/MeetingReview.tsx"]

[[check]]
id = "playwright-proves-meeting-review-flow"
invariant = "Focused browser coverage proves viewing a meeting and triggering extraction on a fragment."
type = "present"
pattern = 'meeting review:'
paths = ["frontend/tests/obligations.spec.ts"]
```

## Notes

All five checks are automated against the implementing component/route
wiring/test.
