# Repository standards uplift

- **Recorded:** 2026-08-13
- **Target classification:** NEW
- **Target revision:** unborn/no commit
- **Golden reference:** `../repo-standard-example` at
  `3b40e00d2aa6cd21c0df2af0cdeef485dbb2a1e5`
- **Checkout state:** dirty from the preceding MindLeak installation; monk-eee
  explicitly authorized edits to named compliance surfaces while preserving
  those changes.
- **Decision authority:** monk-eee, sole decision-maker
- **Intended toolchain:** Rust core with a Node front end
- **Review mechanism:** GitHub

## Classification

The repository has no commits, product implementation, shipped behavior, or
existing decision records. A legacy baseline and historical decision backfill
are therefore not applicable. No product decisions have been invented.

## Compatibility

There is no existing ADR or RFC system to preserve or migrate. The proposed
`docs/adr.d/` collection and exact-name `docs/evidence.d/` pairing can be
introduced without rewriting history. The proposed evidence checker uses Node,
which is already part of the intended product toolchain.

## Approval

On 2026-08-13, monk-eee explicitly accepted both bootstrap decisions as the
repository's sole decision-maker by selecting `Accept both ADRs` at the
interactive approval checkpoint:

1. [ADR-0001](../adr.d/0001-require-governing-adr-coverage-before-implementation.md)
   requires accepted governing ADR coverage before prospective implementation.
2. [ADR-0002](../adr.d/0002-keep-current-evidence-separate-from-accepted-decisions.md)
   keeps current proof separate from immutable accepted intent and derives state
   with a dependency-free Node checker.

Their approved guarantees are implemented through `AGENTS.md`, repository and
contributor guidance, the GitHub pull request template, the ADR authoring skill,
and the dependency-free Node evidence checker. No CI or local commit hook was
authorized by these decisions.

## Evidence and validation

`node scripts/check-evidence.mjs` derives both accepted ADRs as `PROVEN` with no
violated invariant. The checker was also exercised against isolated fault cases:

- a missing required pattern reports `BROKEN` and exits non-zero;
- a failed implementation invariant reports `BROKEN` and exits non-zero;
- orphan evidence reports `BROKEN` and exits non-zero; and
- an accepted ADR without evidence reports `DEADHEADED` but remains non-blocking
  as required by ADR-0002.

`node --check scripts/check-evidence.mjs`, `git diff --check`, repository-wide
local Markdown link validation, and VS Code diagnostics all pass.

## Independent review

An independent agent found no target-fact invention, golden service-detail
leakage, accepted-decision conflict, MindLeak modification, broken link, or
enforcement beyond the accepted scope. It identified two checker hardening
issues, both corrected before completion:

- `present` and `absent` checks now reject a missing or empty `pattern` instead
  of relying on JavaScript coercion; and
- the checker self-evidence now covers process execution, VM execution, `eval`,
  and generated functions rather than only selected child-process APIs.

The reviewer noted that a pull request template declares policy but cannot
enforce branch protection, and that deadheaded evidence is visible but
non-blocking. Both are intentional under the accepted decisions.

## Pending enforcement

No CI workflow, branch-protection rule, or local commit hook was added. Enabling
blocking enforcement is a separate decision that requires a future accepted
ADR after the Rust and Node project gates exist. Owner: monk-eee. Exit condition:
the project-specific build, test, format, and evidence commands are established
and validated on the selected GitHub runner platforms. No review date or
tracking issue exists yet, so this repository must not claim those controls.

## Not changed

The uplift has not changed production behavior, dependencies, pipelines,
infrastructure, Git history, or the existing MindLeak installation.