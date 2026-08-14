# EV-0017: Add a GitHub Actions CI pipeline for backend, frontend, and governance

Evidence for [ADR-0017](../adr.d/0017-add-github-actions-ci-pipeline.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0017-add-github-actions-ci-pipeline"

[[check]]
id = "workflow-defines-backend-job"
invariant = "A GitHub Actions workflow runs the backend test suite."
type = "present"
pattern = 'cargo test'
paths = [".github/workflows/*.yml"]

[[check]]
id = "workflow-defines-frontend-job"
invariant = "A GitHub Actions workflow builds the frontend."
type = "present"
pattern = 'npm run build'
paths = [".github/workflows/*.yml"]

[[check]]
id = "workflow-defines-governance-job"
invariant = "A GitHub Actions workflow runs the ADR/evidence governance gate."
type = "present"
pattern = 'check-evidence\.mjs'
paths = [".github/workflows/*.yml"]

[[check]]
id = "pipeline-observed-green"
invariant = "The pipeline has been observed to run successfully on GitHub."
type = "manual"
last_verified = "2026-08-14"
```

## Notes

`pipeline-observed-green` was verified 2026-08-14: run 31758406186 ("Fix CI:
regenerate frontend lockfile") completed with conclusion `success` across
all three jobs (`gh run list --repo monk-eee/ringmaster`). The two prior
runs on this same workflow failed for real, since-fixed reasons (missing
`@types/react`/`@types/node`, then an out-of-sync `package-lock.json`) —
exactly the "expected, correct signal" ADR-0017's own Consequences section
names, not a defect in the checker.
