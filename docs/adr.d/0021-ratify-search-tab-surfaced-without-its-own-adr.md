# ADR-0021: Ratify surfacing semantic search in the frontend SPA (retroactive)

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Direct instruction ("Accept as drafted"), 2026-08-14
- **Depends on:** [ADR-0014](0014-react-vite-single-page-app.md), [ADR-0015](0015-expose-source-fragment-traceability-on-candidates.md), [ADR-0019](0019-semantic-search-over-source-fragments.md)
- **Tags:** architecture, frontend, governance, retroactive

## Context

[ADR-0019](0019-semantic-search-over-source-fragments.md)'s own Scope
section named "surfacing search in the frontend" as explicitly "a future,
separate UI decision" — out of scope for that record. A commit ("Surface
ADR-0019 semantic search in the SPA") nonetheless added a Search tab
(`frontend/src/components/SearchResults.tsx`, wired into `App.tsx`/
`api.ts`) directly, reasoning by analogy to
[ADR-0015](0015-expose-source-fragment-traceability-on-candidates.md)'s
evidence-column treatment rather than drafting a new ADR first.
[docs/ARCHITECTURE.md](../ARCHITECTURE.md) — an independent review pass —
flagged this honestly as "a judgment call rather than a clean-cut case"
instead of treating it as settled. This record is that outstanding
governance step: either ratify the judgment after the fact, or determine
it needed the fuller review ADR-0019 deferred.

Verified before drafting this: the Search tab commit touched no
`frontend/package.json`/`package-lock.json` — it added no dependency, no
new client-side state-management pattern, no new build step, and no write
capability. It is a presentational, read-only view over
`GET /api/search`, a route [ADR-0019](0019-semantic-search-over-source-fragments.md)
had already accepted as read-only, using the exact same tab convention
[ADR-0014](0014-react-vite-single-page-app.md) already established for
Obligations/Candidates.

## Decision

- Ratify: adding a presentational, read-only frontend surface over an
  already-accepted, already-read-only backend route does not, by itself,
  require a separate governing ADR when it introduces no new backend
  behavior, dependency category, client-side architecture, or interface
  contract change. This extends the same reasoning
  [docs/adr.d/README.md](README.md) already applies to "purely editorial
  corrections that do not alter behavior, constraints, interfaces, or
  operating rules" to this analogous case: a purely presentational
  consumer of an already-governed contract.
- This ratification is scoped **only** to the Search tab as shipped. It is
  not a blanket exemption: a future frontend change that introduces new
  architecture (a state-management library, a new build step, a
  write-capable UI, a new styling system, client-side routing beyond the
  existing tab pattern) still requires its own ADR.

## Scope

**In scope:** retroactive ratification of the already-shipped Search tab
specifically, and the narrow principle above that justifies it.

**Out of scope:** a general, forward-looking policy defining exactly which
future frontend changes do or don't need an ADR — that remains a separate,
deliberate decision if broader clarity is ever wanted; any change to the
already-shipped code itself.

## Options considered

- **Ratify as a defensible, narrowly-scoped call (chosen):** the addition
  introduces nothing architecturally new beyond the precedent
  [ADR-0015](0015-expose-source-fragment-traceability-on-candidates.md)
  already set; reverting or gating already-correct, tested, additive code
  behind a paperwork sequencing fix would be governance theater, not a
  real risk mitigation.
- **Treat it as a process violation and require rework:** rejected — the
  code is correct, tested (Playwright covers the Search tab), read-only,
  and low-risk; nothing here needs to change functionally.
- **Retroactively amend [ADR-0019](0019-semantic-search-over-source-fragments.md)
  to include frontend surfacing in its own scope:** rejected — that ADR is
  Accepted and immutable; amending it would misrepresent what was actually
  decided at the time. A new record is the correct instrument for
  clarifying the position going forward.

## Consequences

- **Positive:** closes the self-flagged governance gap honestly, without
  discarding working, tested code; gives future agents a clear, narrow
  precedent to cite instead of re-litigating the same question.
- **Negative / trade-off:** sets a precedent ("presentational surface over
  an already-accepted route") that could be stretched more broadly than
  intended if a future agent reads it loosely instead of narrowly.
- **Risk:** low — this record changes no code; it only resolves an
  already-identified documentation/governance gap.

## Exit criteria and evidence

Evidence: [EV-0021](../evidence.d/0021-ratify-search-tab-surfaced-without-its-own-adr.md)

| Exit criterion | Evidence |
|---|---|
| The ratified Search tab component exists as shipped | `search-results-component-exists` |
| No new frontend dependency was introduced for the Search tab | `no-new-frontend-dependency` |
