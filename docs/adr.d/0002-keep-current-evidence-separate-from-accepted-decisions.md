# ADR-0002: Keep current evidence separate from accepted decisions

- **Status:** Accepted
- **Date:** 2026-08-13
- **Decider:** monk-eee
- **Approval:** Explicitly accepted by monk-eee during the repository standards uplift on 2026-08-13
- **Depends on:** [ADR-0001](0001-require-governing-adr-coverage-before-implementation.md)
- **Tags:** process, verification, agentic-development

## Context

An accepted ADR proves only that a decision was approved. It does not prove the
implementation ever satisfied the decision or still does. Accepted records must
remain immutable, while evidence describes current reality and can pass, fail,
or become stale.

ringmaster will already use Node for its front end. A dependency-free Node
checker can therefore provide portable repository validation without adopting a
runtime solely for compliance.

## Decision

Every accepted, non-superseded ADR must have an evidence record at
`docs/evidence.d/<same-filename>.md`.

- Evidence records must describe declarative, rerunnable checks in a fenced
  `toml` block.
- Checks must be interpreted by a dependency-free Node script and must not
  execute shell commands from evidence files.
- Evidence state must be derived by running the checker and must not be typed
  into an ADR or evidence record.
- The state vocabulary is `Proven`, `Broken`, `Stale`, `Asserted`, and
  `Deadheaded`.
- A manual check with no `last_verified` value derives as `Asserted`; it must not
  claim a reviewer, date, or successful verification.
- `Broken` evidence must fail the checker. `Stale`, `Asserted`, and `Deadheaded`
  evidence must be visible but remain non-blocking until a later accepted ADR
  changes enforcement.

## Scope

**In scope:** evidence pairing, declarative check data, state derivation, and a
target-native Node checker.

**Out of scope:** product-specific invariants, CI provider configuration, shell
execution from evidence, and automatic blocking for non-broken evidence states.

## Options considered

- **Exact-name evidence records with a Node interpreter (chosen):** separates
  immutable intent from current proof, makes missing evidence computable, and
  uses the intended repository toolchain.
- **Store evidence in each ADR:** easier to find but requires rewriting accepted
  decision history whenever reality changes.
- **Track evidence only in GitHub:** loses the repository-local proof surface
  needed by local contributors and agents.
- **Use arbitrary shell commands as checks:** flexible but platform-dependent
  and unsafe to execute as data.

## Consequences

- **Positive:** accepted intent and current implementation state remain distinct;
  unproved or violated decisions become visible.
- **Negative / trade-off:** every live ADR needs a second maintained file and
  checks must evolve with repository paths.
- **Risk:** a weak check can produce false confidence. Reviews must assess
  whether each check proves the invariant rather than merely matching ADR text.

## Exit criteria and evidence

Evidence: [EV-0002](../evidence.d/0002-keep-current-evidence-separate-from-accepted-decisions.md)

| Exit criterion | Evidence |
|---|---|
| Every accepted, non-superseded ADR has an exact-name evidence record | `every-live-adr-has-evidence` |
| The checker derives state and never executes evidence as shell code | `checker-is-declarative` |
| Repository guidance requires running the checker before reporting evidence state | `agent-guidance-requires-checker` |