# ADR-0030: Human-readable titles and type iconography across the UI

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Direct instruction ("Accept as drafted"), 2026-08-14
- **Depends on:** [ADR-0023](0023-evidence-backed-daily-brief-reasons.md), [ADR-0024](0024-candidate-accept-reject-buttons.md), [ADR-0026](0026-graph-explorer-frontend.md)
- **Tags:** architecture, frontend, ux

## Context

`ObligationsTable.tsx` renders `o.obligation_id.slice(0, 8)…` as its
primary visible content, and `DailyBrief.tsx` shows the same truncated id
next to its reason text. A raw id fragment is not information a manager
can use to recognize what an Obligation *is*. The backend already sends
`source_fragment_id`/`source_text` on `GET /api/obligations`
([ADR-0023](0023-evidence-backed-daily-brief-reasons.md)), but the
frontend's own `Obligation`/`DailyBriefItem` types in `api.ts` never
declared those fields, so the data arrives and is silently dropped before
it ever reaches either component — this is a frontend gap, not a missing
backend capability.

Separately, [docs/PRODUCT-SPEC.md § 5.2](../PRODUCT-SPEC.md#52-core-node-types)
names 15 node types; `GraphExplorer.tsx`'s `nodeTypeColors` already gives
each a distinct background/foreground color, but color alone is a weak
scanning cue (low-vision-unfriendly, and several assigned colors are
visually similar). Candidates carry the same six `candidate_type` values
as node-type analogues (commitment/request/risk/follow_up/decision/
expectation) with no visual distinction at all in `CandidatesTable.tsx`
beyond plain text.

## Decision

- `Obligation`/`DailyBriefItem` (`api.ts`) gain `source_fragment_id: string
  | null` and `source_text: string | null` — fields the backend already
  sends; no backend change. `ObligationsTable.tsx` renders the (truncated)
  evidence quote as its primary visible text when present, falling back to
  the honest existing convention `"No evidence recorded"` when it isn't —
  never fabricating a title. The full id moves to a `title` tooltip on a
  small `<code>` marker rather than being the primary visible content.
  `DailyBrief.tsx` is unchanged beyond this (it already surfaces the quote
  in its `reason` text via [ADR-0023](0023-evidence-backed-daily-brief-reasons.md));
  only the redundant, prominent raw-id display is demoted the same way.
- A new shared `frontend/src/icons.ts` exports one `typeIcon(type: string):
  string` function returning a single emoji per [docs/PRODUCT-SPEC.md § 5.2](../PRODUCT-SPEC.md#52-core-node-types)
  node type (all 15) and the six `candidate_type` values, reusing the same
  glyph for the four names that mean the same thing in both vocabularies
  (risk, decision, request→request, follow_up→follow_up). An unrecognized
  type gets a single neutral fallback glyph — never blank, never an error.
  No icon font or image asset library is added; plain Unicode emoji only.
- `GraphExplorer.tsx` renders `typeIcon(node_type)` beside every existing
  node-type color tag (list, detail panel, SVG neighbor labels) — additive
  to the existing color, not a replacement. `CandidatesTable.tsx` renders
  `typeIcon(candidate_type)` beside its `Type` column text the same way.
- `StatusBadge.tsx` gains one glyph per status (`●` open, `▲` at_risk, `✓`
  closed) prepended to its existing colored badge text, for the same
  color-plus-shape scanning benefit.

## Scope

**In scope:** the `Obligation`/`DailyBriefItem` type fields and the two
components' rendering change; the shared `typeIcon` icon map; wiring it
into `GraphExplorer.tsx`, `CandidatesTable.tsx`, and `StatusBadge.tsx`.

**Out of scope:** any backend/schema change (every field used already
exists and is already sent); a real title/description field on Obligation
itself (still separate, undecided work, named honestly by
[ADR-0027](0027-promote-accepted-candidate-to-obligation.md)); a
picture/avatar system for Person nodes (no image storage exists); an icon
font, SVG icon set, or component library (adds a dependency this ADR does
not decide to take on); re-theming colors already chosen by
[ADR-0026](0026-graph-explorer-frontend.md).

## Options considered

- **Emoji-based `typeIcon` map plus surfacing already-sent evidence text
  (chosen):** zero new dependencies, zero backend changes, reuses data
  that already flows over the wire today; the smallest change that
  directly answers "a guuid isn't useful."
  and "iconography is key."
- **An SVG/icon-font library (e.g. Lucide, Heroicons):** more polished and
  scalable long-term, but a real new dependency and build-tooling decision
  this ADR does not need in order to close the immediate readability gap.
- **Add a real `title`/`statement` field to Obligation now:** would be a
  more complete fix, but is a genuine, larger data-model decision (extraction
  changes, migration, projection carry-forward) that
  [ADR-0027](0027-promote-accepted-candidate-to-obligation.md) already
  named as separate, undecided work — bundling it here would re-open a
  decision already deliberately deferred.

## Consequences

- **Positive:** every Obligation/Daily Brief row shows real content
  instead of a meaningless id fragment whenever evidence exists; every
  node/candidate type gets a consistent, scannable glyph across the whole
  app with no new dependency.
- **Negative / trade-off:** an Obligation with no linked
  `source_fragment_id` still shows only `"No evidence recorded"` — this
  ADR does not fabricate content that doesn't exist, so sparse data still
  looks sparse, honestly.
- **Risk:** low — purely additive/presentational; no new write path, no
  schema change, no new dependency.

## Exit criteria and evidence

Evidence: [EV-0030](../evidence.d/0030-human-readable-titles-and-type-iconography.md)

| Exit criterion | Evidence |
|---|---|
| Obligations/Daily Brief render evidence text instead of a raw id as primary content | `obligations-table-renders-evidence-text` |
| A shared icon map covers every PRODUCT-SPEC node type and candidate type | `type-icon-map-exists` |
| The Graph Explorer and Candidates table render a type icon alongside existing color | `type-icon-wired-into-graph-and-candidates` |
