# ADR-0001: Require governing ADR coverage before implementation

- **Status:** Accepted
- **Date:** 2026-08-13
- **Decider:** monk-eee
- **Approval:** Explicitly accepted by monk-eee during the repository standards uplift on 2026-08-13
- **Tags:** process, agentic-development, documentation

## Context

ringmaster is a new repository with an intended Rust core and Node-based front
end. It has no product implementation or historical decision system yet. This
is the least disruptive point to establish how implementation rationale is
recorded and retrieved.

Code and commit history can show what changed, but they cannot reliably preserve
the constraints, rejected options, and intended trade-offs behind a change.
That missing context is especially risky for coding agents, which otherwise
infer intent from the current implementation.

## Decision

Every mutation to source, tests, configuration, infrastructure, or pipelines
must be governed by an accepted ADR in `docs/adr.d/` before implementation
begins.

- Contributors and agents must first search for an accepted ADR whose decision,
  scope, and trigger cover the proposed work.
- If no accepted ADR applies, a bounded ADR must be proposed and explicitly
  accepted by the named decider before implementation.
- One ADR may govern multiple mutations while they remain within its stated
  boundary. This is ADR coverage for every mutation, not one ADR per mutation.
- Accepted ADRs must not be rewritten. A changed decision must be recorded in a
  new ADR that amends or supersedes the earlier record.
- Purely editorial corrections that do not alter behavior, constraints,
  interfaces, or operating rules are exempt.
- GitHub pull requests must identify the governing ADR or state the editorial
  exemption.

## Scope

**In scope:** prospective changes to repository behavior and engineering
constraints; ADR discovery, lifecycle, and GitHub review declaration.

**Out of scope:** inventing product decisions before product work exists,
retroactively describing rationale, evidence-state mechanics, and blocking CI
enforcement.

## Options considered

- **Require ADR coverage before implementation (chosen):** preserves rationale
  before code shape can be mistaken for intent and gives humans and agents one
  predictable retrieval surface.
- **Record only architecturally significant decisions:** creates a subjective
  gate and can omit rationale whose importance becomes apparent only later.
- **Use issues and pull requests alone:** useful for delivery history, but not a
  stable in-repository decision model and unavailable before every local change.
- **One ADR per mutation:** guarantees coverage but creates ceremonial records
  instead of reusable, bounded decisions.

## Consequences

- **Positive:** implementation starts from explicit, reviewable constraints;
  rejected alternatives and trade-offs remain retrievable.
- **Negative / trade-off:** decision work moves ahead of implementation and the
  collection requires active indexing and lifecycle discipline.
- **Risk:** an unenforced policy can imply coverage that does not exist. GitHub
  review wiring will be added only after this ADR is accepted.

## Exit criteria and evidence

Evidence: [EV-0001](../evidence.d/0001-require-governing-adr-coverage-before-implementation.md)

| Exit criterion | Evidence |
|---|---|
| Always-on agent guidance requires retrieval of a governing accepted ADR before implementation | `agent-guidance-requires-governing-adr` |
| GitHub pull requests declare a governing ADR or valid editorial exemption | `github-review-declares-governing-adr` |
| Contributors can discover the ADR lifecycle and collection from repository guidance | `repository-guidance-links-adrs` |