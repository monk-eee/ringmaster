# ADR-0054: Congruence Engine v1 — flag a commitment with no linked node at all

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Explicitly accepted by monk-eee on 2026-08-17 ("accept all")
- **Depends on:** [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md), [ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md), [ADR-0053](0053-what-am-i-forgetting.md)
- **Tags:** api, architecture, data-model

## Context

[docs/PRODUCT-SPEC.md §8](../PRODUCT-SPEC.md#8-user-experience) names a
"Congruence Engine" as a forward pointer to
[VISION.md](../VISION.md#the-ux-is-the-product) — but neither document
defines it; it has never been built. An independent product review of
[docs/current-status.md](../current-status.md)'s audit described the
intent as detecting *"commitment exists, no supporting work exists"* or
*"customer outcome exists, no related effort exists."*

Detecting that honestly today runs straight into a real limit: this
schema has no concept of "supporting work" or "delivery effort" distinct
from an Obligation itself — no ADO work item, no task, no ticket is
ingested ([VISION.md](../VISION.md#open-questions-for-future-adrs) names
Azure DevOps as a future source, not a present one). Building a detector
against data that doesn't exist would mean fabricating the check, exactly
what this repo has consistently refused to do since
[ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md).
What *is* real and available today: whether a commitment is connected to
anything else in the graph at all.

## Decision

- **A new, narrow signal, `isolated`**: a `commitment`-type Obligation (or
  one promoted from a `commitment`-type candidate) with **zero edges**
  touching its id — nothing links it to a person, a meeting, or any other
  node. This is the honest floor of "congruence": a commitment nobody and
  nothing is connected to is at minimum unverifiable, whether or not
  "supporting work" can be checked.
- **Computed and attached the same way `stale`/`unowned`/`date_compression`
  already are** ([ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md)/[ADR-0046](0046-unowned-obligation-risk-signal.md)):
  a `risk_signals` entry with `"signal": "isolated"` and a plain
  explanation ("Not linked to anyone or anything."), on Daily Brief, Time
  Horizon, and by extension [ADR-0053](0053-what-am-i-forgetting.md)'s
  "What am I forgetting?" composition — no new route, no new frontend
  surface beyond what already renders `risk_signals`.
- **This is explicitly named `v1`** in its own title, signaling — in this
  repo's own established convention (see
  [ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md)'s
  own naming) — that a richer version checking against real delivery work
  is real, separate, future work, not decided here.

## Scope

**In scope:** the `isolated` signal (zero edges on a commitment-type
Obligation), computed and attached exactly where the other three signals
already are.

**Out of scope, named honestly:**

- **"No supporting work exists" / "no related effort exists" in the sense
  the review meant** — checking against actual delivery work (ADO items,
  tasks). No such data is ingested; this is real, larger, later work
  gated on an ADO/work-tracking source that doesn't exist yet.
- **A model or heuristic judging whether a *linked* node is genuinely
  "supporting"** (versus merely linked). `isolated` only checks for zero
  edges, not the quality or relevance of any edges that do exist.
- **Any UI beyond what already renders `risk_signals`.** This is a
  backend signal reusing existing rendering, not a new page or section.

## Options considered

- **A narrow, honest `isolated` signal for v1 (chosen):** the one
  congruence check this data can actually support without fabrication;
  composes directly with [ADR-0053](0053-what-am-i-forgetting.md); named
  `v1` so a real future check against actual work doesn't get confused
  with this one.
- **Build the full "commitment vs. delivery work" check now:** rejected —
  no work-tracking data exists to check against; would be fabricated.
- **Skip a Congruence Engine entirely until ADO ingestion exists:**
  rejected — an honest, narrower version (structural isolation) is real
  and available today; there's no reason to wait for a bigger, harder
  problem to ship a smaller, true one.

## Consequences

- **Positive:** the first real (if intentionally narrow) step toward the
  long-named, never-defined "Congruence Engine," using only data that
  exists, with no fabrication.
- **Positive:** composes for free with [ADR-0053](0053-what-am-i-forgetting.md)'s
  "What am I forgetting?" list and every existing `risk_signals` consumer.
- **Negative / trade-off:** does not detect the review's actual example
  ("commitment exists, no supporting work") in the full sense meant — an
  honestly narrower signal, named as such, not the complete idea.
- **Risk:** low. One additive signal function, reusing the existing edges
  table and the existing `risk_signals` attachment pattern.

## Exit criteria and evidence

Evidence: [EV-0054](../evidence.d/0054-congruence-engine-v1-isolated-commitment-signal.md)

| Exit criterion | Evidence |
|---|---|
| A commitment-type Obligation with zero edges is flagged `isolated` | `isolated-signal-flags-a-zero-edge-commitment` |
| A commitment-type Obligation with at least one edge is not flagged `isolated` | `isolated-signal-does-not-flag-a-linked-commitment` |
| `isolated` appears in `risk_signals` on Daily Brief and Time Horizon, reusing the existing attachment pattern | `isolated-signal-attached-like-existing-signals` |
