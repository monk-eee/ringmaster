# EV-0088: Career/Connect export — a person's completed obligation history, with evidence

Evidence for [ADR-0088](../adr.d/0088-career-connect-export.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0088-career-connect-export"

[[check]]
id = "career-history-returns-closed-obligations-with-evidence"
invariant = "person_career_history returns only status = 'closed' Obligations linked to the person, each with an evidence citation."
type = "present"
pattern = "fn person_career_history"
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "career-history-excludes-open-obligations"
invariant = "An open (non-closed) Obligation linked to the same person is excluded."
type = "present"
pattern = "fn career_history_excludes_an_open_obligation"
paths = ["backend/src/api/nodes.rs"]

[[check]]
id = "career-history-http-route-exists"
invariant = "GET /api/people/:id/career-export serves the same composition over HTTP."
type = "present"
pattern = '"/api/people/:id/career-export"'
paths = ["backend/src/api/mod.rs"]

[[check]]
id = "career-export-honest-empty-state"
invariant = "Person detail renders a Career export section with an honest empty state when there is nothing closed."
type = "present"
pattern = "career-export"
paths = ["frontend/src/components/People.tsx"]

[[check]]
id = "existing-reads-remain-closed-obligation-free"
invariant = "get_node_detail's relationship grouping and person_brief's open_commitments remain unchanged (still closed-obligation-free)."
type = "manual"
rationale = "get_node_detail and person_brief are not edited by this change (person_career_history is a new, separate function); their existing test suites -- including the explicit \"a closed obligation must never appear in either relationship group\" assertions -- continue to pass unmodified, which is the direct proof."
last_verified = "2026-08-19"
```

## Notes

Implemented: `person_career_history` (`backend/src/api/nodes.rs`) queries
`obligation_projection` for `status = 'closed'` rows linked to the person
by any edge, joined against `source_fragments` for evidence, ordered by
`updated_at` descending. Exposed as `GET /api/people/:id/career-export`
(`backend/src/api/mod.rs`). `frontend/src/api.ts` adds `fetchCareerHistory`;
`frontend/src/components/People.tsx` adds a "Career export" section below
Relationship, rendering a read-only, copy-to-clipboard textarea of plain
lines (`- <date>: <reason>`) or an honest empty state.
`get_node_detail`/`person_brief` are not edited.

Verified: three new backend tests
(`person_career_history_returns_closed_obligations_with_evidence`,
`career_history_excludes_an_open_obligation`,
`person_career_history_rejects_a_non_person_node`) pass, alongside the
full backend suite. `npx tsc --noEmit` and `npm run build` both clean.
Two new Playwright tests -- `people tab: Career export shows an honest
empty state for a person with nothing completed (ADR-0088)` and `people
tab: Career export renders completed obligations as copyable text
(ADR-0088)` -- both pass, alongside the full frontend suite (25 passed, 5
pre-existing skips, 0 failed).
