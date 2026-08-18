# ADR-0072: Split oversized, low-cohesion backend modules with no behavior change

- **Status:** Accepted
- **Date:** 2026-08-18
- **Decider:** monk-eee
- **Approval:** Direct instruction ("accept"), 2026-08-18
- **Depends on:** [ADR-0005](0005-adopt-rust-event-sourced-postgres-commitment-graph.md)
- **Tags:** backend, refactor, maintainability

## Context

`backend/src/api.rs` has grown to 1,749 non-test lines (measured as
everything above the file's first `#[cfg(test)]` block) and bundles six
distinct responsibilities behind one `Router` and one file: the Today/Daily
Brief/Time Horizon/Focus Blocks read model, meeting/source ingestion,
candidate lifecycle transitions, search, audit events, and graph node/edge
CRUD. This repository has no existing module-size or cohesion policy in
[AGENTS.md](../../AGENTS.md) or any prior ADR; this record establishes a
narrow one and applies it once, to the file that already exceeds it several
times over.

`backend/src/graph.rs` (522 non-test lines) similarly mixes three concerns —
node CRUD, edge CRUD, and source-fragment/embedding search — sharing one
file, though far less severely than `api.rs`.

Every other file in `backend/src/` (`extraction.rs`, `transcript.rs`,
`obligation.rs`, `embedding_adapter.rs`, `model_adapter.rs`, `audit.rs`,
`lib.rs`, `main.rs`, and `bin/ringmaster-ingest/*`) is a single responsibility
at a reasonable size and is out of scope.

## Decision

- Split `backend/src/api.rs` into a `backend/src/api/` directory: a thin
  `mod.rs` that builds the `Router` and re-exports the same public items,
  plus one submodule per responsibility (obligations/daily-brief/time-horizon/
  focus-blocks, ingestion, candidates, search, audit, graph). Large `impl`
  blocks may be split across files using multiple `impl` blocks for the same
  type; this is a Rust-legal, behavior-neutral split.
- Each existing colocated `#[cfg(test)] mod tests` block moves with the code
  it tests, into the same new submodule.
- No public or `pub(crate)` item is renamed, no function signature changes,
  no route path or handler behavior changes, and no re-export is dropped.
  Every existing caller (the binary crate, the test suite, and the frontend's
  HTTP contract) compiles and behaves identically before and after.
- `backend/src/graph.rs` is a second, separate candidate under this same ADR,
  split the same way (node / edge / source-fragment-search submodules) once
  `api.rs` is done and validated.
- Each module's split is its own scoped commit: `cargo build`, the full
  `cargo test` suite, `cargo clippy --all-targets --all-features -- -D
  warnings`, and `cargo fmt --all` must all pass before that commit, and
  before starting the next module.
- `docs/ARCHITECTURE.md`'s module map is updated to reflect the new file
  layout once `api.rs` (and, later, `graph.rs`) land.

## Scope

**In scope:** splitting `backend/src/api.rs` into cohesive submodules under
`backend/src/api/`, and `backend/src/graph.rs` into cohesive submodules under
`backend/src/graph/`, with zero behavior change. Establishing that a module
mixing more than one clearly distinct responsibility (regardless of size) is
a legitimate split candidate in this repository, alongside raw line count.

**Out of scope:** any change to route paths, request/response shapes, SQL,
validation rules, or any other observable behavior; renaming any public or
`pub(crate)` item; splitting any other file in `backend/src/`; a numeric
line-count ceiling as a standing repository-wide policy (this ADR authorizes
two specific splits, not an enforced rule for all future files); touching
`backend/src/transcript.rs` or any file another in-flight ADR/task is actively
editing.

## Options considered

- **Split by responsibility into a directory-per-module, one PR/commit per
  module (chosen):** smallest reviewable diff per step; each commit is
  independently buildable, testable, and revertible; matches this
  repository's existing pattern of one bounded change per ADR/commit.
- **Split by raw line count alone (e.g., every file over N lines):** rejected
  — line count is a symptom; `graph.rs` and `api.rs` both need per-
  responsibility splits regardless of exact size, and a bare numeric rule
  invented here would misrepresent itself as pre-existing repository policy,
  which it is not.
- **One single commit splitting both files at once:** rejected — harder to
  review, harder to bisect if `cargo test`/`clippy` catches a mistake, and
  raises collision risk with the other actively in-flight work on
  `backend/src/api.rs` this session.
- **Do nothing:** rejected — `api.rs` mixing six responsibilities in one
  1,749-line file already measurably slows down finding and safely changing
  any one of them (evidenced by this session's own difficulty scoping edits
  to it without touching unrelated routes).

## Consequences

- **Positive:** each responsibility (obligations, ingestion, candidates,
  search, audit, graph) becomes independently readable, testable, and
  reviewable; smaller future diffs when only one concern changes.
- **Positive:** zero behavior change means this ships with effectively zero
  product risk — the full test suite is the acceptance bar, not new
  functional review.
- **Negative / trade-off:** more files to navigate for a change that happens
  to span two responsibilities (rare, given how the routes are used today).
- **Risk:** collision with the other actively in-flight session's work on
  `api.rs`. Mitigated by doing the split as its own commit immediately after
  confirming no other edit is in flight on that file, and rebasing/re-running
  the full suite if that changes before this lands.

## Exit criteria and evidence

Evidence: [EV-0072](../evidence.d/0072-split-oversized-low-cohesion-backend-modules.md)

| Exit criterion | Evidence |
|---|---|
| `backend/src/api.rs` is replaced by a `backend/src/api/` directory with one submodule per responsibility, and no public/`pub(crate)` item is renamed | `api-split-preserves-public-surface` |
| The full backend test suite passes unchanged after the `api.rs` split | `api-split-tests-pass` |
| `backend/src/graph.rs` is replaced by a `backend/src/graph/` directory with one submodule per responsibility, and no public/`pub(crate)` item is renamed | `graph-split-preserves-public-surface` |
| The full backend test suite passes unchanged after the `graph.rs` split | `graph-split-tests-pass` |
| `docs/ARCHITECTURE.md`'s module map reflects the new layout | `architecture-doc-reflects-new-layout` |
