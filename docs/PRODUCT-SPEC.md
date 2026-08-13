# Ringmaster — Product Specification

**Attention management for leaders**

A graph-native, agent-based organisational memory system that sees
obligations before they become emergencies.

**North-star question:** What deserves my attention now, and what will
become a problem if I ignore it?

**Owner:** Lyndon Swan  •  **Status:** Personal working draft  •  **Version:** 0.2

> This document is an explicit **draft**, not an accepted engineering
> decision. See [Relationship to accepted ADRs](#relationship-to-accepted-adrs-and-vision) at the end for what
> already conflicts with work this repository has built and accepted, and
> what still needs a bounded ADR before implementation.

## 1. Executive summary

**Product in one sentence.** Ringmaster captures obligations, commitments,
requests, decisions, risks and follow-ups from meetings and connected work
systems, links them in a time-aware graph, and tells one manager what needs
attention before the date arrives.

Ringmaster is being built first for one user: an engineering manager whose
attention is spread across delivery, people, leadership, operations, customer
outcomes and the constant stream of informal asks that never become formal
work items. The first problem is not task creation. The first problem is
dependable recall and early warning. The system must recognise that
management work is broader than core engineering delivery. A promise to a
manager, a training follow-up, upcoming leave, an onboarding gap, a morale
activity, a service handover and an ADO feature can all become obligations.
Some have hard dates. Others only have a soft horizon or an implied cadence.
Ringmaster makes both visible.

| Decision | Position |
|---|---|
| Primary user | Lyndon, initially. Optimise for personal utility before generalisation. |
| Primary entity | Obligation, with Commitment as an explicit promise subtype. |
| Primary input | Recorded meeting transcript plus available AI summary; later, federated enterprise sources. |
| Primary output | A prioritised future-risk horizon with traceable evidence. |
| Data model | PostgreSQL relational core plus graph edges and pgvector embeddings. |
| Interaction model | Web-based and agent-based, exposed through MCP. |
| Implementation | Rust services and a web client. |
| Initial posture | Read, extract, validate and remind. Do not autonomously create or mutate ADO work. |

## 2. Problem and product thesis

### 2.1 The real problem is attention scarcity

Managers are expected to integrate fragmented reality. Delivery systems know
work items. Calendars know dates. Meetings contain asks and decisions. Chats
contain informal promises. Email carries requests. People carry context. The
manager becomes the integration layer, and human memory becomes the weakest
persistence mechanism.

**Product thesis.** A better manager does not need more meetings or another
backlog. A better manager needs reliable organisational memory, evidence, and
enough warning to act before an obligation becomes urgent.

### 2.2 Work is often created by dates

In a delivery-driven environment, the date frequently exists before the work
plan. Reorg transitions, new-starter dates, leave periods, planning
milestones, launches, training deadlines, compliance reviews and performance
cycles all create work. Ringmaster therefore models obligation, time, risk,
action and evidence as a connected chain.

```
DATE / HORIZON -> OBLIGATION -> RISK -> ACTION -> EVIDENCE -> OUTCOME
```

### 2.3 ADO is necessary but insufficient

ADO describes implementation work well, but management obligations
frequently live outside it. Ringmaster must connect ADO tasks and features to
the promise or outcome they serve without forcing every human commitment
into engineering backlog semantics. Existing transition work within Learn
explicitly emphasised clear owners, action items and completion dates, which
validates the need for a durable layer above individual systems of record.

## 3. Goals, non-goals and principles

### 3.1 Goals

- Capture all credible asks, promises, decisions, risks and follow-ups from
  recorded meetings.
- Make hard dates, soft dates, inferred horizons and recurring cadences
  visible before they become urgent.
- Connect management obligations to people, customers, services, features,
  ADO items, meetings and evidence.
- Provide a daily and weekly attention brief suitable for managing down and
  managing up.
- Preserve provenance so every extracted claim can be validated against its
  source.
- Use semantic retrieval across every meaningful graph node. pgvector is
  mandatory, not an optional enhancement.
- Start as an assistive memory system; earn trust before taking actions in
  external systems.

### 3.2 Non-goals for the MVP

- Not a replacement for ADO, Planner, Outlook, Teams, Scout or Microsoft
  Graph.
- Not an employee performance scoring, surveillance or sentiment-inference
  system.
- Not autonomous work assignment or autonomous calendar management.
- Not a generic enterprise SaaS platform in the first version.
- Not a meeting-summary product. Summaries are inputs; durable management
  state is the output.
- Not a system that silently treats model extraction as fact.

### 3.3 Design principles

| Principle | Meaning |
|---|---|
| Attention before administration | The product should reduce the need to reconstruct reality, not create another place to maintain. |
| Evidence before confidence | Every extracted object retains source, quote span, timestamp and extraction confidence. |
| Time before status | Emphasise approaching risk and stale obligations, not raw counts of open items. |
| Customer before activity | Where possible, obligations link to the customer problem or business outcome they serve. |
| Human control before automation | Suggestions precede actions; actions require explicit approval until trust is established. |
| Graph before folders | Relationships and temporal state are first-class. Documents remain evidence, not the organising model. |
| Personal utility before platform | Solve Lyndon's real management load first; generalise only from proven workflows. |

## 4. Agent personas and integration providers

Ringmaster separates reasoning concerns from data-access concerns. Personas
decide what matters. Providers retrieve or mutate data. MCP is the contract
boundary between Ringmaster and connected capabilities. The MCP
specification defines standard resources, prompts and tools for composable
integrations; Ringmaster should adopt the current protocol deliberately
rather than inventing a bespoke plug-in RPC.

### 4.1 Personas

| Persona | Responsibility | Typical question |
|---|---|---|
| Chief of Staff | Cross-domain attention briefing and prioritisation. | What should I care about today? |
| Executive Liaison | Commitments made upward, evidence, decisions and escalation needs. | What do I owe my manager, and what should I tell them? |
| People Steward | Onboarding, leave coverage, training, recognition, career and team rituals. | Who needs support or a follow-up? |
| Delivery Steward | Links commitments to features, ADO items, blockers and evidence. | Which promise has no credible execution path? |
| Customer Advocate | Maintains the chain from customer problem to business outcome and work. | What activity has lost its customer rationale? |
| Risk Sentinel | Detects stale, repeated, unowned or date-compressed risks. | What is likely to bite me next month? |
| Archivist | Maintains provenance, deduplication, temporal history and corrections. | Why does Ringmaster believe this? |

### 4.2 Providers

| Provider | Initial mode | Purpose |
|---|---|---|
| Transcript provider | Read | Recorded meeting transcript and AI summary ingestion. |
| Scout federated provider | Read | Federated discovery across accessible Microsoft 365 and connected data. |
| Microsoft Graph provider | Read first | Calendar, people, mail and meeting context available to the signed-in user. |
| ADO provider | Read first | Features, stories, tasks, dates, ownership and delivery evidence. |
| Ringmaster MCP server | Read/write to Ringmaster | Expose search, obligations, risks, evidence and later approved actions. |
| Future action providers | Approval-gated | Create ADO items, drafts, reminders or coordination artefacts. |

**Boundary rule.** Personas never receive unrestricted credentials or direct
system access. Providers enforce identity, scope, allowlists, audit and
least privilege. Scout and Graph access are advantages, not excuses to copy
every record into Ringmaster.

## 5. Domain model

### 5.1 Obligation as the primary entity

An Obligation is anything a manager may reasonably need to remember, prepare
for, follow up, coordinate, evidence or deliberately dismiss. It may be
explicit, inferred, recurring or system-generated. A Commitment is an
Obligation containing an explicit promise or acceptance of responsibility.

| Type | Examples | Typical time semantics |
|---|---|---|
| Leadership | Transition plan, update, strategy proposal, escalation. | Hard date, next 1:1, soft expectation. |
| Delivery | Feature, migration, quality uplift, customer fix. | Milestone, sprint, quarter, release date. |
| People | Onboarding, training, career follow-up, recognition. | Start date, review cycle, agreed follow-up. |
| Team health | Morale activity, team ritual, coverage check. | Cadence or soft horizon. |
| Operational | Compliance, service ownership, on-call, incident action. | SLA, audit date, rotation date. |
| Personal | Informal promise, reminder, relationship follow-up. | Explicit or inferred soft date. |

### 5.2 Core node types

| Node | Purpose | Vectorised |
|---|---|---|
| Person | Participant, owner, stakeholder, manager or collaborator. | Yes |
| Meeting | Source event and container for transcript evidence. | Yes |
| Source Fragment | Exact quote or bounded source passage. | Yes |
| Obligation | Persistent item requiring awareness or action. | Yes |
| Commitment | Explicit accepted promise. | Yes |
| Request | Ask that may not yet be accepted. | Yes |
| Follow-up | Conversation or validation expected after the source event. | Yes |
| Risk | Explicit or inferred future harm to delivery, people, operations or customers. | Yes |
| Decision | Outcome that constrains future work. | Yes |
| Expectation | Leadership or stakeholder standard without a concrete task. | Yes |
| Date / Event | Hard date, soft horizon, recurrence or triggering event. | Yes |
| Customer Problem | Need or pain that justifies an outcome. | Yes |
| Outcome | Business or customer result. | Yes |
| Service / Feature / Work Item | Execution and ownership context. | Yes |
| Evidence | Observed completion, progress, contradiction or validation. | Yes |

### 5.3 Core edge types

- `PERSON` made / accepted / owns / supports / depends_on `OBLIGATION`
- `OBLIGATION` originated_from `MEETING` or `SOURCE_FRAGMENT`
- `OBLIGATION` due_on / expected_by / recurs_on `DATE_EVENT`
- `OBLIGATION` serves `OUTCOME`; `OUTCOME` addresses `CUSTOMER_PROBLEM`
- `OBLIGATION` implemented_by `FEATURE` or `WORK_ITEM`
- `RISK` threatens `OBLIGATION`, `PERSON`, `SERVICE`, `OUTCOME` or `CUSTOMER_PROBLEM`
- `EVIDENCE` validates / contradicts / advances / closes `OBLIGATION`
- `DECISION` supersedes / constrains / creates `OBLIGATION`
- `FOLLOW_UP` validates / clarifies / escalates `OBLIGATION` or `RISK`
- `ENTITY` semantically_related_to `ENTITY` with model, score and timestamp metadata

### 5.4 Temporal model

| Field | Meaning |
|---|---|
| `observed_at` | When the source evidence was observed. |
| `valid_from` / `valid_to` | When the fact or relationship is believed to apply. |
| `hard_due_at` | Explicit externally meaningful deadline. |
| `soft_due_at` | Negotiable or inferred expected point. |
| `horizon` | 7, 30, 60 or 90-day planning window, or custom. |
| `recurrence_rule` | Cadence for morale activities, reviews or recurring obligations. |
| `last_touched_at` | Last evidence of discussion, progress or validation. |
| `stale_after` | Threshold after which lack of evidence becomes attention-worthy. |

## 6. Transcript-first ingestion and extraction

### 6.1 Input contract

- Meeting identity, title, date, organiser and participants when available.
- Full transcript with speaker and timestamp markers.
- AI-generated summary when available, stored as secondary evidence rather
  than replacing the transcript.
- Links or identifiers for related meetings, files, ADO items and calendar
  events.
- Access classification, tenant/source information and retention metadata.

### 6.2 Extraction pipeline

1. Ingest source metadata and immutable raw transcript reference.
2. Chunk by conversation structure, speaker turns and topic boundaries, not
   arbitrary character count alone.
3. Generate embeddings for chunks and extracted entities using the
   configured embedding model.
4. Extract candidate requests, commitments, obligations, decisions, risks,
   expectations and follow-ups.
5. Attach exact evidence spans, speaker, meeting timestamp, extraction model
   and confidence.
6. Resolve people and work entities against existing graph nodes using
   deterministic identifiers before semantic matching.
7. Deduplicate or link candidates to existing obligations; never silently
   overwrite.
8. Infer dates or recurrence only when evidence supports it, and label the
   inference explicitly.
9. Present uncertain or high-impact candidates for human validation.
10. Update the attention horizon and produce a change summary.

### 6.3 Extraction object contract

```json
{
  "type": "commitment | request | risk | follow_up | decision | expectation",
  "statement": "Provide a transition plan",
  "owner": { "person_id": "...", "confidence": 0.94 },
  "counterparty": { "person_id": "..." },
  "time": {
    "hard_due_at": null,
    "soft_due_at": "2026-08-27",
    "basis": "two-week transition",
    "inferred": true
  },
  "source": {
    "meeting_id": "...",
    "speaker": "...",
    "start_ms": 123400,
    "end_ms": 129900,
    "quote": "..."
  },
  "confidence": 0.88,
  "requires_validation": true
}
```

### 6.4 Validation states

| State | Meaning |
|---|---|
| Candidate | Model-extracted and not yet reviewed. |
| Accepted | Human confirmed the object and material fields. |
| Corrected | Human changed type, owner, date, wording or linkage. |
| Rejected | Not a genuine management object. |
| Superseded | Replaced by a later decision or obligation. |
| Observed complete | Evidence indicates completion but has not been manually closed. |
| Closed | Deliberately completed, dismissed or no longer relevant. |

## 7. Attention and risk engine

The engine should not merely rank due dates. It should combine time,
evidence, ownership, repetition, dependency and customer impact to identify
what deserves attention. Scores are advisory and explainable, never employee
performance metrics.

### 7.1 Initial risk signals

| Signal | Example | Explanation required |
|---|---|---|
| Date compression | A transition is due soon but no handover evidence exists. | Date, missing evidence and source. |
| Staleness | An accepted commitment has not been touched since the source meeting. | Last evidence and stale threshold. |
| Unowned obligation | A request is accepted but no accountable owner is linked. | How acceptance was inferred. |
| Repeated concern | The same risk appears in multiple meetings without mitigation. | Meetings and semantic match. |
| Work disconnect | A delivery commitment has no linked feature or work item. | Search performed and scope. |
| Outcome disconnect | Activity exists but no customer problem or outcome is linked. | Missing relationship, not a judgement. |
| Coverage gap | Leave or departure overlaps critical ownership. | Calendar event, ownership and period. |
| Cadence lapse | A recurring morale or training follow-up is overdue. | Configured cadence and last occurrence. |
| Contradiction | Later evidence conflicts with an accepted decision or due date. | Both source fragments. |

### 7.2 Attention horizons

| Horizon | Question |
|---|---|
| Now | What is overdue, blocked or high-impact today? |
| 7 days | What requires an intervention this week? |
| 30 days | What will become risky next month without preparation? |
| 60 days | What still lacks ownership, plan or evidence? |
| 90 days | Which cycles, transitions or commitments are approaching? |

## 8. User experience

### 8.1 Home: "Run the show"

| Panel | Content |
|---|---|
| Attention now | Small, ranked list of obligations and risks with a plain-language reason. |
| Future risk horizon | 7/30/60/90-day view with hard versus soft dates clearly distinguished. |
| Commitments upward | Promises and expectations connected to the manager and leadership meetings. |
| People and team | Onboarding, leave coverage, training, recognition and recurring team-health obligations. |
| Delivery and customer | Commitments linked to outcomes, features, work items and evidence. |
| Recently changed | New, corrected, contradicted, completed or newly risky graph objects. |

### 8.2 Entity view

- Canonical statement and type.
- Owner, counterparty and related people.
- Hard date, soft horizon, recurrence and staleness.
- Current state and confidence.
- Exact source evidence with transcript navigation.
- Related meetings, decisions, risks, services, features and ADO items.
- Progress or completion evidence over time.
- Accept, correct, merge, split, dismiss, snooze and close controls.

### 8.3 Agent interaction

- "What am I likely to forget from this week's meetings?"
- "Show commitments I made to my manager that have no progress evidence."
- "What becomes risky while these people are on leave?"
- "Find unresolved follow-ups related to onboarding."
- "Which delivery work has no customer outcome attached?"
- "Prepare a factual brief for my next management 1:1, with sources."

**No meeting inflation.** Ringmaster should reduce status and reconstruction
meetings. It should never recommend a new meeting when a written follow-up,
reminder, decision request or existing operating rhythm would suffice.

## 9. Technical architecture

```
WEB CLIENT -> RUST API -> AGENT ORCHESTRATOR -> POSTGRES + PGVECTOR -> MCP PROVIDERS
```

### 9.1 Service components

| Component | Responsibility |
|---|---|
| Web client | Attention brief, graph navigation, evidence review, validation and settings. |
| Rust API | Tenant/user boundary, query API, mutations, audit and policy enforcement. |
| Ingestion workers | Source polling/event intake, transcript parsing, chunking and embedding. |
| Extraction service | Structured extraction, entity resolution, confidence and provenance. |
| Graph service | Node/edge persistence, temporal history, deduplication and traversal. |
| Attention engine | Horizon generation, staleness, risk signals and explanation. |
| Agent orchestrator | Persona routing, tool selection, grounded responses and approval gates. |
| MCP server | Expose Ringmaster resources and tools to approved agent hosts. |
| Provider adapters | Scout, Graph, ADO and future systems behind governed interfaces. |

### 9.2 PostgreSQL and pgvector

PostgreSQL is the system of record. Graph semantics are represented through
typed node and edge tables with temporal attributes. pgvector stores
embeddings beside relational metadata, enabling semantic retrieval to be
combined with normal filters, joins and transactional updates. The official
pgvector project supports exact and approximate nearest-neighbour search and
multiple distance metrics while retaining PostgreSQL capabilities such as
ACID transactions, joins and point-in-time recovery.

| Table | Key contents |
|---|---|
| `nodes` | id, type, canonical text, JSON attributes, lifecycle state, created/updated timestamps. |
| `edges` | from_id, to_id, edge_type, confidence, valid_from/to, provenance. |
| `source_fragments` | source id, bounded text, speaker, timestamps, classification, hash. |
| `embeddings` | entity id, model id, vector, source hash, created_at. |
| `obligations` | owner, status, dates, recurrence, confidence, validation state. |
| `evidence_events` | observed progress, contradiction, completion or correction. |
| `attention_items` | derived signal, score, explanation, horizon, first/last seen. |
| `audit_events` | actor, action, previous/new state, source and policy outcome. |

### 9.3 MCP posture

MCP should expose a narrow, auditable surface. The current MCP specification
supports resources, prompts and tools through a standard protocol. Ringmaster
should use resources for graph context, prompts for persona workflows and
tools for explicit operations. Write tools remain approval-gated. Provider
scopes must remain distinct, so a persona cannot bypass source-system
controls.

| Surface | MVP examples |
|---|---|
| Resources | `ringmaster://attention/today`, `/obligations/{id}`, `/people/{id}/context`, `/meetings/{id}/extractions`. |
| Prompts | `daily-brief`, `weekly-review`, `manager-1on1-prep`, `validate-meeting-extractions`. |
| Read tools | `search_graph`, `list_attention`, `get_evidence`, `compare_obligations`, `find_unlinked_work`. |
| Write tools | `accept_candidate`, `correct_entity`, `link_evidence`, `snooze_attention`, `close_obligation`. |
| Future tools | `propose_ado_item`, `create_ado_item`, `draft_update`, `schedule_follow_up`. |

## 10. Security, privacy and responsible use

- Use the signed-in user's delegated/federated access. Do not broaden access
  beyond what the user can already retrieve.
- Store source references and minimal required fragments where possible;
  avoid copying full mailboxes or unrelated meeting content.
- Apply source classification, retention and deletion to derived nodes and
  embeddings.
- Encrypt data in transit and at rest; isolate credentials from persona
  prompts and model context.
- Redact or exclude secrets, tokens, raw tool payloads, browser storage and
  sensitive operational data from logs.
- Require explicit approval before any external-system mutation.
- Provide complete audit history for extraction, validation, correction,
  linking, action proposal and execution.
- Do not infer employee sentiment, morale scores, performance ratings,
  health conditions or protected characteristics. Team-health support should
  track manager obligations and observable activities, not covertly rank
  people.
- Offer per-source exclusion, per-meeting exclusion, node deletion, export
  and re-index controls.

## 11. MVP scope

### 11.1 MVP promise

> After a recorded meeting, Ringmaster reliably shows the requests,
> commitments, risks and follow-ups that may deserve the manager's
> attention, with dates, people and exact evidence; it then carries accepted
> items forward until completed, dismissed or superseded.

### 11.2 Included

- Personal sign-in and single-user workspace.
- Manual transcript upload plus a provider interface for Scout/federated
  retrieval.
- Optional ingestion of AI meeting summaries as secondary evidence.
- Extraction of request, commitment, obligation, decision, risk, expectation
  and follow-up.
- Human validation queue with correction and deduplication.
- PostgreSQL graph schema and pgvector embeddings for every meaningful node
  and source fragment.
- 7/30/60/90-day attention horizons.
- Web home, meeting extraction review, obligation view and semantic search.
- Read-only ADO linking and read-only people/calendar context where
  authorised.
- Ringmaster MCP server with read tools and internal validation tools.
- Audit trail and basic source deletion/re-index flow.

### 11.3 Deferred

- Autonomous ADO item creation.
- Autonomous assignment, email sending or meeting scheduling.
- Multi-manager tenancy and organisational roll-ups.
- Automated performance or sentiment assessments.
- Broad dashboards, OKR authoring, capacity planning and workforce
  analytics.
- General plugin marketplace.

## 12. MVP user stories and acceptance criteria

| Story | Acceptance criteria |
|---|---|
| Review a meeting | A transcript produces typed candidates, each with source quote, speaker/timestamp, confidence and validation controls. |
| Remember a promise | An accepted commitment remains visible across sessions and appears in the correct time horizon until closed or superseded. |
| See future risk | The system explains why an obligation is at risk using dates, missing evidence, ownership or recurrence. |
| Validate an ask | The user can trace an extracted request back to the exact transcript segment and accept, correct or reject it. |
| Connect delivery | The user can link an obligation to an existing ADO work item without copying or mutating the item. |
| Manage upward | The system can list accepted obligations connected to the manager or leadership meetings with current evidence. |
| Manage people obligations | The system surfaces onboarding, leave coverage, training, recognition and cadence items without scoring employees. |
| Search semantically | A natural-language query retrieves relevant meetings, obligations and evidence even when wording differs. |
| Correct memory | Corrections preserve previous values and provenance and influence future deduplication. |

## 13. Evolution path

| Stage | Capability | Trust boundary |
|---|---|---|
| 1. Remember | Transcript ingestion, extraction, graph, pgvector, validation and search. | Read and store only. |
| 2. Anticipate | Attention horizon, staleness, repeated risk and missing-evidence detection. | Advisory, fully explainable. |
| 3. Recommend | Draft proposed follow-ups, ADO items and management updates. | Nothing leaves Ringmaster without approval. |
| 4. Coordinate | Create or update approved ADO items, drafts, reminders and artefacts. | Explicit approval, scoped tools, full audit. |
| 5. Orchestrate | Run bounded recurring management workflows and verify outcomes. | Policy-gated automation with kill switch. |

## 14. Success measures

The MVP should be measured against personal usefulness and trust, not vanity
adoption. Baselines should be collected before setting targets.

| Dimension | Measure |
|---|---|
| Recall | Accepted obligations later rediscovered by Ringmaster before manual reminder or deadline. |
| Precision | Share of extracted candidates accepted without material correction. |
| Evidence quality | Share of surfaced items with a usable source fragment and provenance. |
| Early warning | Risks surfaced before the hard date or crisis point. |
| Actionability | Attention items dismissed, resolved, linked or acted upon rather than ignored. |
| Trust | Rate of corrections, false alarms, unsupported claims and action reversals. |
| Meeting reduction | Whether status/reconstruction meetings or manual status preparation decrease. |
| Coverage | Balance across leadership, delivery, people, team health, operations and customer obligations. |
| Latency | Time from transcript availability to validated candidates and updated attention horizon. |

## 15. Open design decisions

| Decision | Current position |
|---|---|
| Graph implementation | Typed PostgreSQL node/edge model first; avoid a separate graph database until traversal evidence requires it. |
| Embedding strategy | pgvector required. Version model IDs and retain source hashes for re-embedding. |
| Transcript storage | Prefer immutable source reference plus bounded evidence fragments; verify retention constraints per source. |
| Date inference | Allow soft inferred dates only with basis, confidence and visible "inferred" label. |
| Risk score | Use explainable factors and user-tunable thresholds; never present as employee scoring. |
| Scout ingestion | Treat Scout as a federated provider behind identity and policy boundaries; define allowed resource types explicitly. |
| Agent runtime | Keep persona prompts and provider contracts independent so models and hosts can change. |
| ADO actions | Deferred until read-only linkage and evidence quality earn trust. |

## 16. Initial implementation backlog

| Epic | First deliverable |
|---|---|
| E1: Foundation | Rust workspace, PostgreSQL migrations, pgvector extension check, identity boundary and audit skeleton. |
| E2: Graph model | Nodes, edges, source fragments, obligations, evidence and temporal fields. |
| E3: Transcript ingestion | Upload/import contract, parsing, chunking, hashing and source provenance. |
| E4: Extraction | Structured candidate schema, model adapter, prompts, deterministic validation and confidence. |
| E5: Validation UI | Meeting review queue, evidence panel, accept/correct/reject/merge controls. |
| E6: Semantic retrieval | Embedding pipeline, hybrid search, metadata filters and graph expansion. |
| E7: Attention horizon | Dates, staleness, recurrence, unowned obligations and explainable risk signals. |
| E8: Web home | Attention now, 7/30/60/90-day horizon, commitments up, people obligations and recent changes. |
| E9: MCP server | Resources, prompts and read/internal-write tools with policy and audit. |
| E10: Providers | Scout/federated interface, read-only ADO linker and authorised calendar context. |

## Appendix A. Example management objects

| Source statement | Extracted object | Potential future signal |
|---|---|---|
| "Send me the sprint commitments after planning." | Commitment: provide sprint commitments; counterparty: manager; soft due: after planning. | No evidence exists after the planning event. |
| "We have a two-week transition." | Event/horizon: transition window; inferred end from start date. | Ownership and handover obligations lack evidence as the end approaches. |
| "I need to follow up on the training." | Follow-up: training review; owner: manager. | No date recorded, item stale after configured threshold. |
| Calendar shows leave next month. | Event: person unavailable; evidence: calendar. | Critical service ownership or onboarding overlaps the absence. |
| Team morale activity expected monthly. | Recurring obligation: team ritual. | Cadence lapse when no evidence exists within the configured window. |
| ADO feature has work but no linked outcome. | Execution context without outcome relationship. | Customer Advocate asks for validation, not automatic rejection. |

## Appendix B. Grounding notes

This specification consolidates the design discussion and is informed by
internal examples showing that transition work requires clear ownership,
risks, action items and completion dates; by existing Ringmaster draft text
captured in Teams; by current Scout/MCP dogfood work that emphasises bounded
allowlisted tools, cancellation, audit and fail-closed behaviour; and by
public pgvector and MCP documentation. These sources support architecture
and product constraints but do not replace a formal internal privacy,
security or compliance review.

- Internal: Ringmaster draft Teams message, 13 August 2026.
- Internal: Learn transition meeting invitation requiring owners, risks,
  actions and completion dates.
- Internal: Scout MCP dogfood task using bounded allowlisted invocation and
  governed lifecycle.
- Public: pgvector project documentation for PostgreSQL vector similarity
  search.
- Public: Model Context Protocol specification and 2026-07-28 release
  documentation.

## Relationship to accepted ADRs and Vision

This spec is a much more detailed, versioned refinement of
[docs/VISION.md](VISION.md), and in places it changes direction from what is
already **Accepted** and **built**:

- **Primary entity.** [VISION.md § Core philosophy](VISION.md#core-philosophy-commitments-are-the-primary-entity)
  makes Commitment primary. This spec makes **Obligation** primary, with
  Commitment as an explicit promise subtype. VISION.md has not been rewritten
  to match — see the note at its top.
- **Data model — resolved by ADR-0007.**
  [ADR-0005](adr.d/0005-adopt-rust-event-sourced-postgres-commitment-graph.md)
  is **Accepted** and was already **implemented and tested** as an immutable,
  append-only `commitment_events` log with a `commitment_projection` read
  model, always fully rebuilt from that log. §9.2's `obligations` /
  `evidence_events` / `audit_events` language read as a different guarantee
  on a literal reading. [ADR-0007](adr.d/0007-generalize-obligation-and-require-pgvector.md)
  (Amends ADR-0005) resolves this by renaming the aggregate and its schema
  from Commitment to Obligation — `obligation_events` and
  `obligation_projection` — while keeping every event-sourcing guarantee
  ADR-0005 established. ADR-0005's own text was not rewritten; its evidence
  now points at the renamed files, which continue to satisfy it.
- **pgvector — resolved by ADR-0007.** The `vector` extension is now a
  required migration (`backend/migrations/0003_enable_pgvector.sql`,
  verified against a running Postgres instance), with a minimal,
  dimension-unconstrained `embeddings` table per §9.2. No Rust code reads or
  writes it yet — the embedding pipeline (Epic E6) remains future,
  ADR-governed work.
- **Ringmaster's own outward-facing MCP server** (§4.2, §9.3) is new scope:
  distinct from [ADR-0003](adr.d/0003-ringmaster-ingests-mindleak-as-an-mcp-source.md),
  which governs Ringmaster *consuming* MindLeak, not Ringmaster *exposing*
  itself to other agent hosts. Needs its own ADR.
- **Provider/persona architecture, extraction pipeline, and the read-only
  initial posture** (§4, §6, §11.1 "Do not autonomously create or mutate ADO
  work") are all new, real engineering decisions with no governing ADR yet.
- **Compatible, no new ADR needed:** the single-user, personal-sign-in
  posture (§1, §11.2) matches [ADR-0004](adr.d/0004-defer-multi-user-access-control-single-user-v1.md)
  as already accepted.
