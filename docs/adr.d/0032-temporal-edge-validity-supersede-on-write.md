# ADR-0032: Wire up edge temporal validity — supersede-on-write and relationship history in the Graph Explorer

- **Status:** Proposed
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Pending — awaiting monk-eee's decision
- **Depends on:** [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md), [ADR-0025](0025-node-edge-write-api-and-traversal.md), [ADR-0026](0026-graph-explorer-frontend.md)
- **Amends:** [ADR-0025](0025-node-edge-write-api-and-traversal.md)'s "Out of scope: editing or deleting edges" — narrowly, for one automatic, opt-in, system-driven mutation (closing a superseded edge's `valid_to`), not general edge editing
- **Tags:** architecture, api, data-model, frontend, graph

## Context

[PRODUCT-SPEC.md § 5.4 Temporal model](../PRODUCT-SPEC.md#54-temporal-model)
names `valid_from`/`valid_to` as first-class, spec-defined fields: "When
the fact or relationship is believed to apply."
[ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md) already
added `valid_from`/`valid_to TIMESTAMPTZ` columns to the `edges` table for
exactly this purpose ("Confidence, validity window, and provenance are
carried per edge"). Nothing has ever used them: `Edge`'s Rust struct omits
both columns, `create_edge`'s `INSERT` never sets them,
`GET /api/nodes/:id`'s neighbor query never selects them, and the Graph
Explorer has no way to show that a relationship lapsed or was replaced by
a newer one. monk-eee shared a reference diagram: a `User` `LIVES_IN`
Barcelona from January; in March a new `LIVES_IN` Madrid edge is created,
which closes the Barcelona edge's validity window (`valid_to`) rather
than deleting it — both remain visible, one current, one historical. This
ADR is that already-designed, never-wired-up capability.

## Decision

- **Backend, write path (`POST /api/edges`, amends ADR-0025's scope):**
  - Accepts two new optional fields: `valid_from` (RFC 3339 timestamp;
    defaults to `now()` when `supersede` is true and the field is
    omitted) and `supersede` (bool, defaults to `false`).
  - When `supersede` is `false` — the default, so every existing caller
    and test is byte-for-byte unaffected — behavior is unchanged from
    today: `valid_from`/`valid_to` are stored `NULL`, exactly as now.
  - When `supersede` is `true`: in one transaction, every existing edge
    sharing this new edge's `from_id` and `edge_type` where
    `valid_to IS NULL` (still current) has its `valid_to` set to this new
    edge's `valid_from`; then the new edge is inserted with that
    `valid_from` and a `NULL` `valid_to`. Matching is deliberately on
    `(from_id, edge_type)` only, not `to_id` — the point is "this node
    has one current fact of this type," matching the spec's own singular
    `valid_from`/`valid_to` per relationship.
  - `supersede` is opt-in, never automatic from `edge_type` alone, because
    most edge types in real use today (`made`, `flagged`, `owns`) are
    legitimately multi-valued — one person can flag many risks.
    Auto-superseding by type alone would silently destroy real, unrelated
    edges the first time a second same-typed edge was created.
- **Backend, read path:** `Edge`'s struct and every query that selects an
  edge (`get_edge`, the `POST /api/edges` response,
  `GET /api/nodes/:id`'s neighbor query) include `valid_from`/`valid_to`.
  Purely additive JSON fields; existing consumers that ignore unknown
  fields are unaffected.
- **Frontend (Graph Explorer, purely additive rendering):**
  - The "Add relationship" form gains an unchecked-by-default checkbox,
    "Replace any current relationship of this type," which sets
    `supersede: true` on submit.
  - In the SVG relationship view, an edge with a non-null `valid_to`
    (superseded) renders its line dashed and muted, and its pill label
    gains "· until `<date>`". An edge with a non-null `valid_from` and a
    null `valid_to` (current, with a known start) keeps its normal solid
    rendering and its pill label gains "· since `<date>`". An edge with
    both fields null (the overwhelming majority today) renders exactly as
    it does now.

## Scope

**In scope:** `valid_from`/`supersede` request fields on
`POST /api/edges`; the transactional close-out-then-insert supersede
behavior; `valid_from`/`valid_to` in every edge read path; the Graph
Explorer's supersede checkbox and dashed/muted/"until"/"since" rendering.

**Out of scope:** a general `PATCH /api/edges/:id` (still not possible —
the only way `valid_to` is ever set is the automatic supersede path);
deleting edges (unchanged, still impossible); retroactively backfilling
`valid_from` on any of today's existing edges (they keep `NULL`/`NULL`,
rendered exactly as before); any notion of overlapping/non-exclusive
validity windows (e.g., "person A managed by B and C simultaneously") —
supersede always assumes at most one current edge per
`(from_id, edge_type)`; a UI to pick an arbitrary past `valid_from` date
(defaults to `now()` only).

## Options considered

- **Opt-in `supersede` flag, matched on `(from_id, edge_type)` (chosen):**
  safest option — every existing edge-creation call is provably
  unaffected (defaults `false`), and it directly implements the one
  worked example monk-eee provided without guessing at a broader policy.
- **Auto-supersede whenever `(from_id, edge_type)` already has a current
  edge, no flag:** rejected — would silently close out legitimate
  multi-valued relationships (e.g., a second "flagged" risk edge would
  wrongly end the first) the moment this ADR shipped, with no opt-out.
- **A frozen "exclusive" vs "multi-valued" `edge_type` vocabulary decided
  up front:** rejected as premature —
  [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md) already
  deliberately kept `edge_type` free-text; inventing a type taxonomy now
  is a bigger, separately-justified decision than wiring up two
  already-existing columns.
- **A dedicated `PATCH /api/edges/:id` letting any client set `valid_to`
  directly:** rejected — broader surface than the one worked use case
  needs, and reopens ADR-0025's explicit "editing edges" exclusion
  further than necessary.

## Consequences

- **Positive:** closes a gap that has existed since
  [ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md) (schema)
  through [ADR-0025](0025-node-edge-write-api-and-traversal.md)/[ADR-0026](0026-graph-explorer-frontend.md)
  (write API and UI, both still unused) — with no new migration, since
  the columns already exist. Directly implements monk-eee's shared
  example. Zero behavior change for any existing caller
  (`supersede` defaults `false`).
- **Negative / trade-off:** `supersede` matching only on
  `(from_id, edge_type)` means two different UI actions that happen to
  reuse the same free-text `edge_type` string for a `from_id` will
  unintentionally compete to be "current" — an accepted, named risk given
  `edge_type` is still deliberately free-text
  ([ADR-0009](0009-add-graph-nodes-edges-and-source-fragments.md)).
- **Breaking-ish change, named explicitly:** this is the first time an
  `edges` row can be mutated (`valid_to` set) after creation, narrowly
  amending [ADR-0025](0025-node-edge-write-api-and-traversal.md)'s "Out of
  scope: editing or deleting edges." The mutation is system-driven only
  (never a direct client-supplied `valid_to`), gated behind the opt-in
  `supersede` flag.
- **Risk:** low. Additive fields everywhere except one narrowly-scoped,
  opt-in, transactional mutation; no change to any other edge type's
  behavior; no new dependency.

## Exit criteria and evidence

Evidence: [EV-0032](../evidence.d/0032-temporal-edge-validity-supersede-on-write.md)

| Exit criterion | Evidence |
|---|---|
| `POST /api/edges` with `supersede: false` (or omitted) stores `NULL`/`NULL`, unchanged from today | `supersede-defaults-false-and-is-unchanged` |
| `POST /api/edges` with `supersede: true` closes the prior current edge's `valid_to` and inserts the new edge current | `supersede-closes-prior-current-edge` |
| Supersede matches on `(from_id, edge_type)` only, not `to_id` | `supersede-matches-from-id-and-edge-type` |
| Edge reads (`GET /api/nodes/:id` neighbors, `POST /api/edges` response) include `valid_from`/`valid_to` | `edge-reads-include-validity-window` |
| The Graph Explorer renders a superseded edge dashed/muted with an "until" label, and a current dated edge with a "since" label | `graph-explorer-renders-temporal-edges` |
