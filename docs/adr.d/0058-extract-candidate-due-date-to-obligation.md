# ADR-0058: Extract a due date from a candidate and carry it to the promoted obligation

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Amends:** [ADR-0027](0027-promote-accepted-candidate-to-obligation.md)
- **Depends on:** [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md), [ADR-0020](0020-obligation-due-date-fields.md), [ADR-0027](0027-promote-accepted-candidate-to-obligation.md)
- **Tags:** extraction, obligations, product

## Context

A live end-to-end run this session exposed a concrete gap. Ingesting a real
1:1 whose transcript said *"send me the transition plan **by Friday**"*
extracted a `request` candidate and promoted it into an Obligation — but the
Obligation landed with **no due date**, so Today rendered it as *"No due date
recorded"* and could not rank it by urgency. The deadline the manager
actually stated was silently dropped.

The cause is structural, not a bug:
[ADR-0020](0020-obligation-due-date-fields.md) added
`hard_due_at`/`soft_due_at` to an Obligation, and
[ADR-0027](0027-promote-accepted-candidate-to-obligation.md) deliberately
promotes a candidate into an Obligation carrying *only* its
`source_fragment_id` forward, with an explicit code comment that *"no due
date is implied by a candidate."* Nothing between a transcript and an
Obligation ever captured a stated deadline, even though
[ADR-0020](0020-obligation-due-date-fields.md)'s fields and Today's own
urgency ranking ([ADR-0022](0022-daily-brief-endpoint.md)) are built to use
one. This is the single most load-bearing missing link between "real data
went in" and "Today ranks what matters."

## Decision

Capture an optional due date during extraction and carry it into the promoted
Obligation as a **soft** due date, event-sourced, with **no schema
migration**.

### Extraction captures an optional `due_at`

- The provisional extraction prompt ([ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md),
  already labelled not-final) additionally asks the model for an optional
  `"due_at"` (RFC3339 or null) when the fragment states a deadline, resolving
  relative expressions ("by Friday") against a **reference date** the caller
  supplies.
- `extract_candidate_via_model` takes that reference date and injects it into
  the prompt; it parses `due_at` defensively (a malformed or absent value
  becomes `null`, never an error — identical posture to the rest of the
  provisional extractor).
- The parsed `due_at` is written into the existing `extracted`
  `candidate_events` payload (already a JSON document), so **no new column
  and no migration** is introduced. The event log remains the source of
  truth.

### Promotion carries `due_at` into the Obligation's `soft_due_at`

- `promote_candidate` reads the candidate's `extracted` event `due_at` and,
  when present, includes it as `"soft_due_at"` in the Obligation's `created`
  event payload.
- `soft`, not `hard`: a model-inferred date is advisory, not an
  authoritative commitment a human confirmed. This matches
  [ADR-0020](0020-obligation-due-date-fields.md)'s own distinction and lets a
  later explicit correction set `hard_due_at` without contradiction.
- [`obligation::rebuild_projection`](../../backend/src/obligation.rs) already
  carries `soft_due_at` forward from a `created` event payload
  ([ADR-0020](0020-obligation-due-date-fields.md)), so the projection,
  Today's ranking, Time Horizon bucketing, and the risk engine all pick the
  date up with **zero further change** — the plumbing already exists; only
  the value was never supplied.

### Reference date is the extraction time (provisional)

- The reference date passed for relative-date resolution is the extraction
  request time (`now`), not the fragment's `occurred_at`. For extraction run
  near ingestion these coincide; resolving "by Friday" against the meeting's
  own `occurred_at` instead is named out of scope below, matching
  [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)'s
  "provisional prompt" framing.

## Scope

**In scope:** the prompt addition; `due_at` parsing and persistence into the
`extracted` event payload; a reference-date parameter on
`extract_candidate_via_model`; `promote_candidate` reading `due_at` and
setting the Obligation's `soft_due_at`; a deterministic test proving the
carry.

**Out of scope, named honestly:**

- **Resolving relative dates against the fragment's `occurred_at`** rather
  than extraction time — a follow-up once the provisional prompt is
  hardened.
- **Surfacing the candidate's `due_at` in the Inbox before promotion** — that
  needs a `candidate_projection` column (a migration) and its own decision;
  this ADR deliberately keeps `due_at` in the event log only.
- **Setting `hard_due_at` from extraction**, or natural-language date parsing
  outside the model — an authoritative/`hard` date stays a human act.
- **Back-filling** already-promoted Obligations that lost their date before
  this change.

## Options considered

- **Store `due_at` in the `extracted` event payload, read it at promote
  (chosen):** migration-free (this repo's single most error-prone operation
  on the shared dev DB, per repo memory), event-sourced, and delivers the
  full user-visible win (Today ranks a real deadline). The only cost is one
  extra read of the source-of-truth event at promote time.
- **Add a `due_at` column to `candidate_projection`:** cleaner for also
  showing the date in the Inbox, but requires a migration and touches the
  projection rebuild, struct, and every candidate read — more surface and
  the exact migration-desync risk this repo repeatedly hits, for a UX bonus
  that is genuinely separate work; deferred.
- **Parse dates from the statement in Rust (no model):** deterministic but a
  whole natural-language-date subsystem; contradicts
  [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)'s decision
  to let the model do extraction; rejected.
- **Set `hard_due_at` instead of `soft_due_at`:** overstates a model guess as
  an authoritative commitment; rejected in favour of `soft`.

## Consequences

- **Positive:** a promoted Obligation now inherits the deadline its source
  actually stated, so Today, Time Horizon, and the risk engine rank it by
  real urgency instead of showing "No due date recorded" — closing the gap
  the live run surfaced.
- **Positive:** zero migration; the change is additive to an existing JSON
  event payload and one promote path.
- **Negative / trade-off:** the date is only as good as the provisional
  model prompt, and relative dates resolve against extraction time, not the
  meeting time — both named and bounded above.
- **Risk:** low — a `null`/malformed `due_at` degrades exactly to today's
  behaviour (no date), so the worst case is the current state, never worse.

## Exit criteria and evidence

Evidence: [EV-0058](../evidence.d/0058-extract-candidate-due-date-to-obligation.md)

| Exit criterion | Evidence |
|---|---|
| Extraction requests and persists an optional `due_at` | `extraction-captures-a-due-date` |
| Promotion carries a candidate's `due_at` into the Obligation as `soft_due_at` | `promote-carries-due-date-to-soft-due-at` |
| The carry is proven by a deterministic test | `due-date-carry-is-tested` |
| The full backend suite passes with the change | `backend-suite-passes-with-due-date-carry` |
