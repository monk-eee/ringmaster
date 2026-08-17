# ADR-0060: Extract an owner name from a candidate and link it at promotion

- **Status:** Proposed
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Pending — awaiting monk-eee's decision
- **Amends:** [ADR-0027](0027-promote-accepted-candidate-to-obligation.md)'s promotion (carries an owner name forward, when resolvable, as an `owns` edge)
- **Depends on:** [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md), [ADR-0046](0046-unowned-obligation-risk-signal.md), [ADR-0058](0058-extract-candidate-due-date-to-obligation.md)
- **Tags:** extraction, obligations, product

## Context

[docs/PRODUCT-SPEC.md §6.3](../PRODUCT-SPEC.md#63-extraction-object-contract)
has, since before this session, specified an `owner: { person_id, confidence }`
field on every extracted candidate — never implemented.
[ADR-0058](0058-extract-candidate-due-date-to-obligation.md) closed the
identical gap for `time`/`due_at` this session, using a stated pattern
worth repeating verbatim: extend the provisional model prompt, store the
raw result in the existing `extracted` event JSON payload (no migration),
degrade to `null` on anything malformed or absent, and let promotion carry
it forward. `owner` is the one piece of that same contract still missing,
and it is the direct, structural reason
[ADR-0046](0046-unowned-obligation-risk-signal.md)'s `unowned` signal
fires on almost everything today: nothing between a transcript and a
promoted Obligation has ever captured who was actually named responsible,
so every `owns` edge today requires a separate, manual step.

An independent product review argued Ringmaster needs a much larger
"Commitment Engine" (owner/stakeholder/evidence-scoring/health-states/
hierarchies/natural-language query as a new primary entity replacing
Obligation). Several of its concrete points already exist, decided, in
this repo's own docs — `owner`/`counterparty`/`confidence` is
[PRODUCT-SPEC.md §6.3](../PRODUCT-SPEC.md#63-extraction-object-contract),
"Commitment" is already [PRODUCT-SPEC.md §5.1](../PRODUCT-SPEC.md#51-obligation-as-the-primary-entity)'s
named promise-subtype of Obligation, and a "Daily Promises Brief" is
functionally what Today plus "What am I forgetting?"
([ADR-0053](0053-what-am-i-forgetting.md)) already is. This ADR is scoped
to the one piece of that review that is genuinely new work, already
specified, and directly buildable without contradicting an existing
decision: capturing `owner` at extraction time. The review's other ideas
are addressed in this ADR's own Options/Scope sections, not silently
ignored.

## Decision

### Extraction captures an optional `owner_name` (a raw string, not a resolved id)

- The provisional prompt gains a fourth field:
  `"owner_name": "name explicitly stated as responsible, or null"` —
  alongside the existing `candidate_type`/`statement`/`confidence`/
  `due_at`. A plain string, matching how `participants`/`speaker` are
  already handled elsewhere ([ADR-0040](0040-dated-source-ingestion.md)'s
  own stance: raw strings, no identity resolution at extraction time).
- `extract_candidate_via_model` parses it the same defensive way as
  `due_at`: a missing or non-string value becomes `None`, never an error.
- Stored in the existing `extracted` event's JSON payload
  (`owner_name`) — **no migration**, identical to
  [ADR-0058](0058-extract-candidate-due-date-to-obligation.md)'s own
  posture.

### Promotion resolves the name against existing Person nodes only — never creates one

- `promote_candidate` reads the candidate's `owner_name`
  (mirroring `candidate_extracted_due_at`'s pattern) and, when present,
  looks up a `person` node whose `canonical_text` matches
  case-insensitively.
- **Exact match found**: an `owns` edge is created from that person to the
  new Obligation, in the same transaction as promotion — closing the
  `unowned` signal for this Obligation automatically, the same way a
  human doing it manually already would.
- **No match (including no `owner_name` at all)**: nothing extra happens.
  No new Person node is fabricated. The Obligation promotes exactly as it
  does today, `unowned` still fires correctly — an honest miss, not a
  guess.

## Scope

**In scope:** the `owner_name` prompt field and its defensive parsing; a
new `candidate_extracted_owner_name` helper (mirroring
`candidate_extracted_due_at`); auto-creating an `owns` edge at promotion
on an exact existing-Person match only.

**Out of scope, named honestly (and against the wider "Commitment Engine"
review specifically):**

- **`counterparty`.** [PRODUCT-SPEC.md §6.3](../PRODUCT-SPEC.md#63-extraction-object-contract)
  also names one; distinguishing "who owes this" from "who it's owed to"
  from raw fragment text is a real, separate prompt-design problem this
  ADR does not solve — `owner` alone is this session's own
  "single most load-bearing missing link" framing
  ([ADR-0058](0058-extract-candidate-due-date-to-obligation.md)), and
  stacking a second, harder field on top risks neither shipping cleanly.
- **Fuzzy/partial name matching, or creating a Person node when no match
  exists.** Exact case-insensitive match only. A near-miss (nickname,
  typo, "Roopa" vs "Roopa Venkat") resolves to no owner, not a guess —
  matching [ADR-0040](0040-dated-source-ingestion.md)'s own refusal to
  auto-resolve participant strings.
- **Making "Commitment" the primary entity, replacing Obligation.** This
  directly contradicts
  [PRODUCT-SPEC.md §5.1](../PRODUCT-SPEC.md#51-obligation-as-the-primary-entity)'s
  own explicit, already-decided position ("Obligation as the primary
  entity... A Commitment is an Obligation containing an explicit promise").
  Revisiting that would need its own ADR explicitly superseding §5.1, with
  monk-eee's explicit sign-off — not a side effect of this one.
- **A continuous "evidence score" that raises/lowers a numeric risk.**
  [ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md)
  ("no combined severity score is computed here") and
  [ADR-0053](0053-what-am-i-forgetting.md) both already chose independent,
  named signals over a score, on the same reasoning: no validated
  weighting model exists. Still true; not revisited here.
- **Health states (Healthy/At Risk/Stalled/Waiting/Broken/Completed).**
  Real and worth having — proposed separately as
  [ADR-0061](0061-obligation-health-label.md) so it can be accepted/
  implemented independently of this one.
- **Commitment drift detection, commitment hierarchies, a natural-language
  "did I say I would do this?" query, and per-audience stakeholder views
  beyond the existing Person page.** All real, all named directly by the
  review, none buildable honestly today without new data this repo
  doesn't have (a distinct "effort"/work-item concept for drift; a
  parent/child edge type for hierarchies; a query engine for natural
  language) or a decision bigger than this ADR. Not fabricated here.
- **Delaying ADO/connector work.** Already true — nothing since
  [ADR-0040](0040-dated-source-ingestion.md) has added a live connector;
  this ADR doesn't change that either way.

## Options considered

- **Raw `owner_name` string, exact-match resolution only, no auto-creation
  (chosen):** the smallest change that closes the actual named gap
  (nothing captures a stated owner today), reuses
  [ADR-0058](0058-extract-candidate-due-date-to-obligation.md)'s exact,
  already-proven pattern, and cannot fabricate a person or a link.
- **Have the model return a `person_id` directly (matching §6.3's literal
  JSON shape):** rejected — the model never sees existing Person node ids;
  it can only report what the text said, a name. Resolution has to happen
  server-side, against real data, not be hallucinated by the model.
- **Fuzzy-match names (edit distance, nicknames):** rejected as premature
  — no evidence yet of how much this actually matters versus how often
  it'd misfire; exact match is the honest, zero-false-positive starting
  point named in this ADR's own Scope.
- **Rebuild extraction around `owner`+`counterparty`+the full §6.3 shape
  at once:** rejected — bigger, slower to verify, and stacks an unsolved
  problem (counterparty disambiguation) on top of a solved one (owner).

## Consequences

- **Positive:** the first extraction field to ever capture *who* is
  responsible, directly reducing false-positive `unowned` signals for
  newly-ingested content without a manual edge-creation step.
- **Positive:** validates and closes one concrete piece of an otherwise
  much larger, partly-already-decided, partly-premature external review —
  with the rest of that review's ideas explicitly named and reasoned
  about rather than silently accepted or ignored.
- **Negative / trade-off:** an owner stated with a nickname, typo, or
  partial name resolves to no owner — a known, named limitation, not a
  silent failure.
- **Risk:** low. No schema migration; reuses an already-proven pattern
  and an already-existing edge type (`owns`); never creates data that
  wasn't stated.

## Exit criteria and evidence

Evidence: [EV-0060](../evidence.d/0060-extract-candidate-owner-and-link-at-promotion.md)

| Exit criterion | Evidence |
|---|---|
| The extraction prompt asks for `owner_name` and defensively parses it, degrading to `None` on anything malformed or absent | `owner-name-parsed-defensively` |
| Promoting a candidate whose `owner_name` exactly matches an existing Person node creates an `owns` edge in the same transaction | `promotion-creates-owns-edge-on-exact-match` |
| Promoting a candidate with no `owner_name`, or one matching no existing Person, promotes exactly as before — no new Person node, no edge | `promotion-unchanged-without-a-match` |
