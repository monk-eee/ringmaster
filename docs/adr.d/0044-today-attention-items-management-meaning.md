# ADR-0044: Today attention items show management meaning, not identifiers — plain-language title, human date, and evidence status

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Direct instruction ("accept 44 and go"), 2026-08-17
- **Depends on:** [ADR-0022](0022-daily-brief-endpoint.md), [ADR-0023](0023-evidence-backed-daily-brief-reasons.md), [ADR-0030](0030-human-readable-titles-and-type-iconography.md), [ADR-0039](0039-product-re-steer-primary-navigation.md), [ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md)
- **Amends:** [ADR-0039](0039-product-re-steer-primary-navigation.md)'s Today page, picking up part of the per-item detail it explicitly deferred ("Out of scope, named honestly"); does not amend any backend route's existing fields or the daily-brief ranking.
- **Tags:** frontend, api, ux, information-architecture

## Context

monk-eee re-pasted the product re-steer brief verbatim.
[ADR-0039](0039-product-re-steer-primary-navigation.md) already delivered its
navigation half (Today/Timeline/People/Inbox, the greeting, the capped
ranked list, "Do these together", the coming-soon strip) and honestly
deferred the brief's per-item requirement #3 — *plain-language title, why it
matters now, date/horizon, evidence status, related people, a primary
action, and review/correct/snooze/dismiss controls* — as a named gap.

Today, each attention row on the Today page still renders a **raw truncated
`obligation_id`** as its most prominent identifier. That directly violates
the brief's two clearest rules: *"tabs/first screen contain management
meaning, not implementation metadata"* and progressive disclosure (*"IDs,
event history, extraction confidence... appear only after the user asks for
detail"*). It is the single most "this is a CRUD app" element left on the
landing screen.

Three of the deferred per-item fields are achievable **from data the
`GET /api/daily-brief` response already computes** — no new capability, no
fabrication:

- The endpoint already `LEFT JOIN`s `source_fragments` and selects
  `sf.text` to build its `reason` string
  ([ADR-0023](0023-evidence-backed-daily-brief-reasons.md)); it simply does
  not return that quote. Exposing it is the exact one-field addition
  `GET /api/obligations` already makes (`"source_text": source_text`), same
  query, zero new cost.
- `hard_due_at` / `soft_due_at` are already in the response.
- `source_fragment_id` is already in the response (present ⇒ evidence
  linked; `null` ⇒ none).

The remaining deferred fields — related people/outcomes/services, a real
correct/snooze/dismiss action, and the "What changed" section — each need a
genuinely new backend capability or data-model decision (a related-node
projection on the brief, a persisted snooze/dismiss state, a read surface
over `audit_events`/event history). This ADR does not add any of those; it
takes only the slice that honest, already-present data supports.

## Decision

### Backend (one field, precedented)

`GET /api/daily-brief` adds `source_text: string | null` to each item —
the already-selected `sf.text`, returned verbatim, exactly as
`GET /api/obligations` already does. No query change, no ranking change, no
new route. Every existing field stays.

### Frontend (Today page only, presentation)

Each Today attention item renders, reusing existing components/classes:

- **Plain-language title:** the evidence quote (`source_text`) when present;
  when absent, an honest neutral label derived from `status`
  (e.g. "At-risk obligation", "Open obligation") — never a fabricated
  sentence, never an ID. The raw `obligation_id` chip is **removed** from
  the Today row (it remains available on the demoted Obligations surface,
  where identifiers belong).
- **Why it matters now:** the existing `reason` string and
  `risk_signals` explanations ([ADR-0041](0041-risk-engine-v1-staleness-and-date-compression-signals.md)),
  unchanged.
- **Date / horizon:** a deterministic human phrase from the effective due
  date (`hard_due_at` else `soft_due_at`): "Due today", "Due in N days",
  "N days overdue", or an honest "No date recorded" when both are null.
  No fabricated date.
- **Evidence status:** an explicit, legible indicator — "Evidence recorded"
  when `source_fragment_id` is set, "No evidence recorded" when null —
  rather than leaving it implied inside the reason text. Reuses the
  existing semantic-color restraint (never red unless the item itself is
  at-risk/overdue).

Only the Today page changes. The demoted Obligations/Search/Graph surfaces
keep their existing table/ID presentation unchanged — identifiers and
metadata stay exactly where progressive disclosure wants them.

## Scope

**In scope:** exposing `source_text` on `GET /api/daily-brief`; the Today
item's plain-language title (with an honest no-evidence fallback), removal
of the raw-ID chip from Today rows, a human date/horizon phrase, and an
explicit evidence-status indicator.

**Out of scope, named honestly (still real gaps from the brief, each needing
its own backend/data decision):**

- **Related people/outcomes/services per item** — the daily-brief response
  carries no related-node data; adding it is a real projection/query
  decision, not presentation.
- **A real primary action and correct/snooze/dismiss controls** — "review"
  needs a first-class obligation-detail surface (none exists yet;
  [ADR-0043](0043-meeting-review-page.md) is building the *meeting* detail,
  not an obligation one), and snooze/dismiss need a persisted state and
  route that do not exist.
- **The "What changed" section** — needs a read surface over
  `audit_events` (now populated by
  [ADR-0038](0038-wire-up-audit-events-for-candidate-validation.md)) or
  obligation event history; [ADR-0008](0008-add-append-only-audit-events-table.md)
  explicitly deferred an audit read API.

## Options considered

- **Expose the already-fetched quote + render title/date/evidence from
  existing fields (chosen):** smallest honest step that removes the raw-ID
  progressive-disclosure violation and delivers three of the brief's
  per-item fields with no new capability and no fabrication.
- **Compute a title from a model/summary:** rejected — fabricates a
  description the brief explicitly forbids ("do not fabricate descriptions")
  and adds a model dependency to a landing screen.
- **Build the whole of requirement #3 at once (related people, actions,
  snooze/dismiss, What changed):** rejected as one unreviewable change that
  would force several new backend capabilities the brief warns against
  adding "merely to decorate the UI"; each deferred piece deserves its own
  bounded decision.
- **Leave the raw ID on the Today row:** rejected — it's the clearest live
  violation of the brief's own progressive-disclosure and "management
  meaning not metadata" rules.

## Consequences

- **Positive:** the Today landing screen stops showing database identifiers
  and starts leading with the manager-meaningful quote, why-now, date, and
  evidence status — directly advancing the brief's success test ("know what
  to deal with next without visiting another tab"), with one precedented
  backend field and otherwise pure presentation over existing data.
- **Negative / trade-off:** the plain-language title is the evidence quote,
  not a curated headline; when no evidence is linked it falls back to a
  plain status label rather than a rich description — honest, but plainer
  than the brief's eventual intent, which a later related-node/title
  decision can enrich.
- **Risk:** low. One additive JSON field (existing callers ignoring it are
  unaffected); Today-only presentation; honest empty/unknown states
  throughout; no fabrication, no new route, no new dependency.

## Exit criteria and evidence

Evidence: [EV-0044](../evidence.d/0044-today-attention-items-management-meaning.md)

| Exit criterion | Evidence |
|---|---|
| `GET /api/daily-brief` returns `source_text` for each item | `daily-brief-returns-source-text` |
| The Today row renders no raw obligation identifier | `today-row-hides-raw-id` |
| The Today row shows a human date phrase or an honest "no date" state | `today-row-shows-human-date-or-honest-empty` |
| The Today row shows an explicit evidence-status indicator | `today-row-shows-evidence-status` |
