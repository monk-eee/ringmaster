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
