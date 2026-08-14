# Ringmaster — Vision

**Status:** Draft business/architecture context. This document is not an ADR
and does not itself govern implementation — see
[Relationship to the ADR system](#relationship-to-the-adr-system).

> A more detailed, versioned specification now exists at
> [docs/PRODUCT-SPEC.md](PRODUCT-SPEC.md) (v0.2, personal working draft). It
> refines the primary entity from Commitment to **Obligation** (with
> Commitment as a promise subtype) and adds detail this document does not
> cover. Where the two disagree on an already-accepted engineering decision,
> [PRODUCT-SPEC.md § Relationship to accepted ADRs and Vision](PRODUCT-SPEC.md#relationship-to-accepted-adrs-and-vision)
> is the authoritative account of the conflict.

## Tagline

Ringmaster. Keep the whole show moving.

## A Management Operating System

Managers are expected to do more with less.

At scale, organizations do not fail because people are lazy, lack talent, or
have insufficient tools. They fail because commitments become disconnected
from reality.

Promises are made in meetings. Actions are captured in ADO. Decisions live in
chats. Context lives in email. Dates live in calendars. Knowledge lives in
people's heads.

The manager becomes the integration layer.

Ringmaster exists to remove the manager as the bottleneck. Its purpose is to
continuously maintain a living model of commitments, people, work, outcomes,
risks, and time — so managers spend less effort reconstructing reality and
more effort making decisions.

## Problem statement

Current tools optimize for work. Managers optimize for outcomes. There is a
fundamental mismatch.

**Existing systems know:** work items, tasks, features, bugs, meetings,
emails, calendars, documents.

**Managers need to know:**

- What promises have we made?
- What commitments are at risk?
- What is going to become a problem?
- Who needs help?
- What has changed?
- What should I tell my boss?
- What is falling through the cracks?
- Are we solving the right customer problems?

None of these questions have a natural home in existing systems.

## Core philosophy: commitments are the primary entity

Ringmaster does not model the organization around tasks. It models the
organization around commitments. Everything else exists to support a
commitment.

```
Customer Problem -> Business Goal -> Commitment -> Feature -> ADO Work -> Delivery
```

The commitment is the durable object. Work items may change. People may move.
Teams may reorganize. The commitment remains.

## What Ringmaster tracks

- **Delivery commitments** — reduce production incidents, improve service
  quality, deliver feature X, complete migration Y, improve repository health.
- **Leadership commitments** — produce a transition plan, build an onboarding
  process, improve cross-team collaboration, deliver an organizational
  objective.
- **Team commitments** — morale activities, training plans, onboarding
  programs, operating rhythm.
- **People commitments** — career development, promotion readiness,
  mentoring, recognition, performance conversations.
- **Operational commitments** — security reviews, compliance actions, service
  ownership transitions, incident follow-ups.
- **Personal commitments** — things a manager promised, follow-up
  discussions, actions from 1:1s, requests from leadership, informal
  agreements.

## The real insight

Work is not the scarce resource. Attention is.

Ringmaster is fundamentally an attention allocation system. Its job is
determining: **what should this manager care about right now?**

## Time-centric design

Most systems are work-centric. Organizations are often date-driven instead:
reorg effective dates, new-hire start dates, connect cycles, planning
milestones, launch dates, fiscal deadlines, training completion dates,
compliance reviews, service transitions. The date creates the work.

Ringmaster operates around:

```
Date -> Commitment -> Risk -> Action -> Evidence
```

## Future risk horizon

The key feature is not current work — it is future problems. Managers already
know today's fires. The value comes from identifying tomorrow's fires.

Ringmaster continuously identifies:

| Horizon | Question |
|---|---|
| 7 days | What requires action this week? |
| 30 days | What becomes risky next month? |
| 60 days | What lacks preparation? |
| 90 days | What commitments are approaching without sufficient progress? |

## Organizational memory

The most important capability. Ringmaster remembers promises, decisions,
agreements, expectations, and follow-ups — for example, "Roopa asked for a
transition plan," "leadership requested quality improvements," "John
requested mentoring support," or "new-hire onboarding was identified as a
risk."

Months later the system can answer: was this completed? Who owns it? What
evidence exists? Is it still relevant?

This solves one of the biggest organizational failures: important things
quietly disappearing.

## Managing up

Ringmaster supports upward management: what have I committed to leadership?
What needs an update? What risks require escalation? What evidence shows
progress?

The system continuously builds an executive narrative — not status reports,
but evidence-backed leadership communication.

## Managing down

Ringmaster supports people leadership: who have I not spoken to recently?
Who looks overloaded? Who may be disengaged? Where is knowledge
concentrated? Which onboarding activities are incomplete?

The goal is proactive leadership rather than reactive management.

## Managing sideways

Cross-team coordination is increasingly important: which dependencies exist?
Which commitments span multiple teams? Which stakeholders are affected?
Which services lack clear ownership?

Ringmaster exposes dependency risk before delivery risk appears.

## Customer obsession

Everything starts with a customer problem. Features are not first-class
objects; customer outcomes are.

- Every commitment should answer: what customer problem does this solve?
- Every feature should answer: which commitment does it support?
- Every work item should answer: why does this exist?

The chain from customer to implementation should always be visible.

## Agent architecture

| Agent | Answers |
|---|---|
| Chief of Staff | What changed? What needs attention? What should I care about? |
| Delivery | Which commitments lack implementation? Which initiatives are drifting? |
| Customer | Which work is tied to customer outcomes? Which work lacks customer justification? |
| Team Health | Who needs support? Who owns too much? What knowledge is concentrated? |
| Leadership | What should be escalated? What updates are required? |
| Portfolio | Are commitments translating into outcomes? Where are investment and outcomes misaligned? |

## Dashboard philosophy

No vanity metrics. No ticket counts. No meeting counts. No activity
visualization. Only actionable context:

- **Attention required** — things needing action now.
- **Upcoming risks** — things needing action soon.
- **People** — humans requiring manager attention.
- **Leadership** — commitments made upward.
- **Customers** — outcomes requiring attention.
- **Delivery** — commitments lacking execution.

## Technology direction

- **Backend:** Rust.
- **Storage:** Postgres — a temporal, event-sourced commitment graph.
- **Integration layer:** MCP-first architecture.
- **Sources:** Azure DevOps, Outlook, Teams, Calendar, SharePoint, OneNote,
  GitHub, service ownership systems, MindLeak.

Ringmaster consumes data; it does not replace source systems. ADO remains the
work system. Ringmaster becomes the intelligence system.

This is intended direction, not an accepted engineering decision. The backend
runtime, storage engine, and event-sourcing model are addressed by
[ADR-0005](adr.d/0005-adopt-rust-event-sourced-postgres-commitment-graph.md).
Each source connector is expected to become its own bounded, accepted ADR
under [`docs/adr.d/`](adr.d/README.md) before implementation begins.

## Relationship to MindLeak

MindLeak remembers what agents forget. Ringmaster remembers what
organizations forget.

- MindLeak models: repositories, files, decisions, agents.
- Ringmaster models: people, commitments, outcomes, organizations.

Both are fundamentally temporal memory systems. Both exist to stop context
leakage. The precise boundary between them (federation vs. ingestion vs.
shared schema) is not yet decided — see
[Open questions](#open-questions-for-future-adrs).

## The UX is the product

*(monk-eee, 2026-08-14 — a reframe worth recording verbatim in spirit, not
just summary.)* The graph, Rust, pgvector, MCP, provider integrations, even
the ADR-governance process itself — all of that is infrastructure. Nobody
opens Ringmaster because it has a good node/edge model. They open it because
they're thinking *"Shit, what am I forgetting?"*, *"What should I do next?"*,
or *"Why does this feel overwhelming?"*

**The biggest UX mistake to avoid** is building an engineer's UI — a grid of
entity types (People, Meetings, Services, Risks, Nodes, Edges, Obligations,
Candidates) or a row of vanity counts (327 obligations, 91 risks, 204
candidates). Nobody wants to manage entities. They want to manage their
attention.

### The Daily Brief

Jira, Planner, ADO, Monday, even Copilot — all of them start with **Work**.
Ringmaster should start with **Attention**. Imagine opening Ringmaster on a
Monday morning. Not a count. A brief:

> **Good morning, Lyndon.**
>
> 4 things need attention today.
> 2 will become risks this week.
> 1 commitment appears forgotten.
>
> 1. **Transition Plan** — Roopa expectation. Due in 8 days. No evidence
>    recorded.
> 2. **New Team Members** — Intro conversations missing. 2 people affected.
> 3. **John Leave Coverage** — Starts in 12 days. No replacement owner
>    recorded.
> 4. **Team Health** — No morale activity in 43 days.

That screen *is* the product. Everything else — the graph, the extraction
pipeline, the attention horizon, the risk engine — exists to make that one
screen true, current, and trustworthy. This sharpens §8.1's existing "Home:
Run the show" panels (Attention now, Future risk horizon, Commitments
upward, People and team, Delivery and customer, Recently changed) from a
dashboard grid into a single ranked, narrative brief.

### Congruence over completion — "the killer widget"

Not "what should I do?" but **"what should I do together?"** When several
open items are actually one piece of context — a transition plan, an
ownership review, service mapping, new team members — Ringmaster should
notice they're connected and propose a themed block of focused work rather
than surfacing four unconnected tasks:

> **Suggested Focus Blocks**
>
> 🎯 **Reorg Transition** — these belong together: Transition Plan, Service
> Ownership, New Team Members, Knowledge Transfer. Estimated effort: 90
> mins. **[Start Focus Session]**

The grouping isn't manual linking — it comes from the graph already
knowing these share the same people, the same services, the same
meetings, the same dates, and the same customer outcomes. monk-eee calls
this "the most useful thing in the application." This turns "manage
tasks" into "manage context."

A longer-horizon version of this same idea is a **Congruence Engine**
(working name; "Coherence Score" also considered): detecting when a stated
commitment, the goals derived from it, and the actual work being done drift
apart — "Improve quality" was promised, but no ADO work items exist under
it; a monthly morale commitment has no evidence in 70 days; twenty work
items exist with no customer outcome linked. Not task management —
management *coherence*. This is a new concept beyond anything currently in
[PRODUCT-SPEC.md §7](PRODUCT-SPEC.md#7-attention-and-risk-engine)'s risk
signals and deserves its own future bounded ADR once the underlying
obligation/work-item linkage exists to detect it from.

### Context switching is the enemy — Focus Sessions

Managers don't lose time doing work — they lose time switching between
unrelated kinds of work (quality review → vacation request → onboarding →
customer escalation → sprint planning → career discussion, back to back).
Rather than one flat list, Ringmaster should cluster by management context
— People, Delivery, Leadership, Operations — as an explicit **Focus
Session** the manager starts on purpose:

> **People Focus Session** — David onboarding, training follow-up,
> recognition for Hector, career discussion with John, team morale
> activity. Estimated 35 mins.
>
> **Leadership Focus Session** — Roopa commitments, transition plan, FY27
> goals. Estimated 25 mins.

Now the manager works *in* contexts instead of bouncing constantly. This is
a different organizing principle than §8.1's panel-by-panel layout: cluster
by *what kind of attention it takes*, not by entity type.

### The manager's workbench, not a dashboard

Three panes, not a dashboard grid:

| Pane | Content |
|---|---|
| Left — Attention | Needs attention now / needs attention soon / recently changed. |
| Centre — Current focus | The selected item's owners, risks, evidence, related commitments. |
| Right — Relationship context | The relevant person: open commitments, recent asks, next scheduled conversation. |

Concretely, selecting an item in the left pane fills the centre pane with
its full context — urgency color-coded (🔴🟠🟡🟢), never a bare list:

```
ATTENTION NOW          CURRENT FOCUS
🔴 Transition Plan     Transition Plan
🟠 New Team Members      Due: 8 days · Counterparty: Roopa
🟡 Leave Coverage        Related: Ownership review, Service allocation,
🟢 Team Morale                    New team onboarding
                         Risks: No draft located, No evidence in 8 days
                         Evidence: Meeting transcript, Roopa 1:1
```

### Timeline, not graph, not table, not kanban — the Future Risk Horizon

The whole thesis is `Date → Obligation → Risk`
([§6, Attention and risk engine](PRODUCT-SPEC.md#7-attention-and-risk-engine)).
Managers think in time, not hierarchy or entity type. The default view
worth building toward is a timeline — today, then each upcoming
obligation/risk/milestone in date order — not a graph visualization, not an
entity table, not a kanban board:

> **Future Problems**
>
> **Next 7 Days** — 🔴 Transition Plan (no evidence recorded); 🟠 Team
> onboarding (no intro meetings completed).
>
> **Next 30 Days** — 🟡 John leave coverage (no successor identified); 🟡
> Training follow-up (no activity recorded).
>
> **Next 90 Days** — 🟢 Connect cycle; 🟢 Service review cycle; 🟢 Team
> morale checkpoint.

Notice what's absent: no tasks, no epics, no stories. Problems.

### Relationship pages as external memory

A dedicated page per person (Roopa, David, John, ...) showing: commitments
made to them, requests from them, decisions involving them, risks affecting
commitments connected to them, recent meetings, open follow-ups:

> **Roopa**
>
> Open Commitments (4) — Transition Plan, Ownership Review, Quality
> Metrics, FY27 Alignment.
> Recent Requests — Service visibility, Quality outcomes.
> Upcoming — 1:1 Tuesday.
> Risks — Transition plan evidence missing.
> Last interaction — 3 days ago.

A manager spends most of their time managing relationships, not entities —
monk-eee expects "this view will be gold." This sharpens §8.2's "Entity
view" (owner/counterparty/related people) into a first-class, person-
centric page rather than a generic entity template — this *is* the
organizational memory the [Organizational memory](#organizational-memory)
section above describes, made concrete as a UI surface.

### A meeting enters as a proposed subgraph

The intended starting workflow is ordinary management work. Lyndon has a
meeting with his manager, Roopa Venkat, and gives the transcript to an agent
through Ringmaster's MCP or CLI surface. The durable output is not another
meeting summary. It is a proposed addition to organizational memory:

```text
Roopa Venkat ── attended ──> Weekly 1:1
Weekly 1:1 ── contains ──> Transcript fragment
Roopa Venkat ── requested ──> Transition plan
Lyndon ── owns ──> Transition plan
Transition plan ── originated from ──> Weekly 1:1
Transition plan ── supported by ──> Exact transcript fragment
Transition plan ── expected by ──> Next 1:1
```

The agent extracts people, meetings, actions, requests, obligations,
decisions, risks, expectations, dates, documents, and the relationships
between them. A useful agent-facing grammar is **source → relationship →
destination**. In the product, however, these are typed, directed
relationships rendered as ordinary language — "Roopa requested the
transition plan" — rather than abstract database labels.

The proposal retains the meeting, exact quoted fragments, speaker, time,
confidence, and extraction history. The interface presents the transcript
beside the proposed items; selecting an item highlights its supporting
passage. The manager can accept, correct, merge, or reject it. Accepted
items become durable memory and flow into the relevant relationship page,
Daily Brief, and Future Risk Horizon. Model suggestions remain visibly
different from accepted facts.

The ingestion boundary should feel atomic and repeatable from the agent's
perspective: submit one meeting and its proposed subgraph, rather than issue
a loose series of unrelated low-level writes. The eventual implementation
details — batch contract, identity resolution, idempotency, and promotion
rules — require bounded ADRs. The product intent is clear regardless: MCP
or CLI is the ingestion surface; the web experience is where evidence is
reviewed and attention is directed.

### The graph as progressive exploration

The graph is not only hidden machinery. It can also be a primary memory and
discovery surface when it is traversed deliberately, one meaningful step at
a time. The fuller interaction design is captured in
[Relationship Memory and Progressive Graph Design](RELATIONSHIP-GRAPH-DESIGN.md).
Opening Roopa starts with Roopa and her immediate neighbours.
Selecting the Weekly 1:1 makes that meeting the new centre and enriches it
with participants, transcript evidence, extracted actions, decisions,
risks, related documents, and adjacent meetings. Selecting a Product Docs
Archive mentioned in that meeting makes the archive the new centre and
reveals the documents, decisions, obligations, people, and history connected
to it.

```text
Roopa Venkat
    └── attended ──> Weekly 1:1
                           └── discussed ──> Product Docs Archive
                                                    └── informed ──> Migration decision
                                                                            └── created ──> Migration obligation
```

The traversal path remains visible and reversible:

```text
Roopa > Weekly 1:1 > Product Docs Archive > Migration decision
```

Every selected node becomes a suitably enriched object, not merely a label
in a diagram. Its detail surface answers:

- What is this?
- Why is it connected to the place I started?
- What happened before?
- What is true now?
- What actions or risks are open?
- What happens next?
- Which sources support this account?

The default neighbourhood is one hop. The manager can widen it to three,
ten, or another useful distance, but depth is not permission to dump the
entire graph onto the screen. Each extra hop is progressively disclosed,
ranked, and constrained by the current question. A broad ten-hop enquiry is
better expressed as "find the strongest paths between Roopa and the Product
Docs Archive" than as thousands of simultaneously rendered nodes.

Useful traversal controls include:

- **Distance** — one hop, two, three, or a custom reach.
- **Direction** — incoming, outgoing, or both.
- **Time** — current truth, full history, or truth as of a chosen date.
- **Relationship lens** — actions, people, meetings, documents, risks, or
  everything.
- **Evidence posture** — accepted facts only or accepted facts plus model
  suggestions.

Visual language should preserve trust. Current accepted relationships are
solid. Superseded relationships remain available in history. Suggested or
semantically discovered relationships are visibly provisional — for
example, dashed — and can be inspected, accepted, or rejected. Every
important connection can explain itself through a readable path and source
evidence.

This produces two synchronized experiences over the same memory. The
relationship page gives an opinionated **past → present → future** account
of working with Roopa. The progressive graph lets the manager leave that
account and follow the context wherever it leads. The graph is not a
decorative network visualization and not an engineer-only CRUD screen. It
is a navigable model of organizational memory.

### The home-screen formula

Every morning, Ringmaster should answer exactly five questions:

1. What needs attention today?
2. What becomes risky soon?
3. What am I likely forgetting?
4. What should I tell my manager?
5. **What should I do together?**

The last one is unique to Ringmaster — no work-tracking tool asks it,
because none of them model congruence across obligations in the first
place.

### Instruments, not dashboards

What's worth stealing from flight simulators isn't a dashboard — it's
instruments. A top bar showing overall **Attention Pressure** (a filled
bar, not a number to optimize) alongside a load reading per management
direction:

```
Attention Pressure  ██████░░░░

People       6
Delivery     4
Leadership   8
Operations   2
```

The point isn't to create stress or gamify a score — it's to answer
*"where is managerial load accumulating?"* at a glance, the same way a
pilot glances at instruments rather than reading a report.

### One button: "What am I forgetting?"

The most radical simplification of all: not search, not a query language,
not a graph explorer. One button:

> 🪄 **WHAT AM I FORGETTING?**

which answers directly, in plain language:

> - Training follow-up from 18 days ago
> - Commitment to provide transition plan
> - Team morale activity overdue
> - John leave coverage unresolved

This is the same underlying data
([§9.2 attention_items](PRODUCT-SPEC.md#92-postgresql-and-pgvector),
semantic search over embeddings) presented as a single, fearless
interaction instead of a search box a manager has to know how to use.

### A reframed priority order

monk-eee's stated build order, if prioritizing from a blank slate: Daily
Brief, Relationship View, Time Horizon, Congruence Engine, Risk Engine,
Candidate Validation, ADO integration, Automation — reasoning that if the
first six work properly, ADO integration becomes close to obvious. This
maps onto, and reprioritizes ahead of, the existing
[§16 backlog](PRODUCT-SPEC.md#16-initial-implementation-backlog)'s E5–E8
ordering; reconciling the two into a formal, versioned backlog revision is
PRODUCT-SPEC.md's own call, not made here.

The through-line, in monk-eee's own words: **"Ringmaster isn't helping
managers manage work. It's helping managers maintain a coherent mental
model of reality."** That's the product. The graph, Rust, and pgvector are
just how it's kept true.

## Ultimate goal

A manager should be able to open Ringmaster on Monday morning and immediately
understand: what matters, what changed, what is at risk, who needs help, what
leadership expects, what customers need, and what commitments are being
forgotten — without scheduling a single extra meeting.

That is management leverage. That is how organizations genuinely do more
with less.

## Open questions for future ADRs

This vision intentionally leaves the following undecided. Each is a candidate
for its own bounded ADR before the affected implementation begins:

1. **MindLeak/Ringmaster boundary** — does Ringmaster ingest MindLeak as one
   MCP source among many, federate queries across both graphs, or something
   else? They must not silently merge into one schema. Resolved by
   [ADR-0003](adr.d/0003-ringmaster-ingests-mindleak-as-an-mcp-source.md).
2. **Sensitive data boundary** — People Commitments include
   performance/promotion-readiness data flowing through MCP sources such as
   Outlook, Teams, and SharePoint. Access control and handling for this class
   of data needs an explicit decision before any such ingestion is built.
   Interim v1 answer in
   [ADR-0004](adr.d/0004-defer-multi-user-access-control-single-user-v1.md).
3. **Event-sourced commitment schema** — the Postgres event-sourcing model
   for the commitment graph is a foundational, hard-to-reverse data-model
   decision and needs its own ADR with options considered, not an assumption
   carried over from this document.

## Relationship to the ADR system

Per [`docs/adr.d/README.md`](adr.d/README.md), target architecture and broad
business outcomes belong in an architecture document, linked from the bounded
ADRs that implement pieces of it — not absorbed into an ADR itself. This
document is that architecture/business context. It does not carry `must` /
`must not` engineering rules, exit criteria, or evidence, and accepting it is
not equivalent to accepting any specific engineering decision described
inside it.
