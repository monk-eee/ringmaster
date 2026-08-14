# Ringmaster -- Relationship Memory and Progressive Graph Design

**Status:** Working product-design intent, 2026-08-14. This document is not
an ADR and does not govern implementation. It develops the experience
described in [VISION.md](VISION.md); each implementation slice still needs
the accepted ADR coverage required by
[ADR-0001](adr.d/0001-require-governing-adr-coverage-before-implementation.md).

## Purpose

Ringmaster should turn ordinary management interactions into navigable,
evidence-backed organizational memory.

The representative workflow is simple:

1. Lyndon meets with his manager, Roopa Venkat.
2. An agent receives the meeting transcript through MCP or a CLI.
3. The agent proposes people, actions, decisions, risks, dates, context, and
   relationships found in the transcript.
4. Lyndon validates the important claims against their evidence.
5. Accepted memory appears in Roopa's relationship page, the Daily Brief,
   and the Future Risk Horizon.
6. Lyndon can follow the graph from Roopa to the meeting, from the meeting to
   a Product Docs Archive, and onward through decisions and obligations.

The desired result is not a meeting-summary product and not a database
browser. It is a reliable way to recover context and move through it.

## Product mental model

The graph contains the memory. The interface gives that memory a point of
view.

- A **relationship page** answers what working with one person means now,
  what happened before, and what is coming next.
- A **meeting page** explains what entered memory from one interaction and
  lets the manager validate it.
- A **node detail** explains one selected object, why it is connected, and
  what evidence supports it.
- A **progressive graph** lets the manager pivot from item to item without
  losing the path that led there.
- The **Daily Brief** and **Future Risk Horizon** are opinionated projections
  of the same accepted memory.

The graph view therefore complements the timeline-first product direction in
[VISION.md](VISION.md#timeline-not-graph-not-table-not-kanban--the-future-risk-horizon).
The timeline remains the best default for attention. The graph is the place
to investigate context and answer "how is this connected?"

## From meeting to memory

### 1. Ingest

The agent treats one meeting as one ingestion unit. It submits or resolves:

- Meeting identity, time, participants, and source reference.
- The transcript and immutable bounded evidence fragments.
- Proposed nodes such as people, obligations, requests, decisions, risks,
  dates, documents, services, and outcomes.
- Proposed directed relationships between those nodes.
- Confidence, extraction provenance, and the evidence behind each claim.

The useful machine-facing grammar is:

```text
source -> relationship -> destination
```

For example:

```text
Roopa Venkat -> requested -> Transition plan
Lyndon -> owns -> Transition plan
Transition plan -> originated from -> Weekly 1:1
Transition plan -> supported by -> Transcript fragment 18
Transition plan -> expected by -> Next 1:1
```

This grammar should not leak into the user experience as fields named
"source" and "destination." The interface renders the same fact as a
sentence: **Roopa requested the transition plan.**

### 2. Review

The meeting review experience places evidence and interpretation together:

```text
+-------------------------------+-------------------------------+
| TRANSCRIPT                    | PROPOSED MEMORY               |
|                               |                               |
| Roopa: Can you bring me a     | Request                       |
| transition plan next Friday?  | Roopa requested a transition |
| ^ highlighted evidence        | plan by Friday.               |
|                               |                               |
| Lyndon: Yes, I will draft it. | Obligation                    |
|                               | Lyndon owns the draft.        |
|                               |                               |
|                               | [Accept] [Correct] [Merge]    |
|                               | [Reject]                      |
+-------------------------------+-------------------------------+
```

Selecting a proposal highlights its supporting transcript passage. Selecting
a passage reveals claims derived from it. The manager can:

- Accept a supported proposal into durable memory.
- Correct its wording, type, dates, people, or relationships.
- Merge it with an existing object.
- Split a proposal that combines distinct claims.
- Reject it while retaining the extraction history.
- Defer it when the evidence is real but the meaning is unresolved.

The review unit is a claim with evidence, not an isolated node. Accepting
"Transition plan" should make the relevant ownership, request, source, and
date relationships understandable as one proposal.

### 3. Carry forward

Accepted items immediately become available to attention and relationship
views. A manager should not need to file the same fact again after validation.

- Roopa's page gains the request and the next expected conversation.
- Lyndon's obligations gain the transition-plan commitment.
- The Daily Brief can surface it when it deserves attention.
- The Future Risk Horizon can place it against Friday.
- The meeting retains the exact evidence and extraction history.

## Roopa relationship page

This page represents **my working relationship with Roopa**, not a generic
employee profile and not an assessment of Roopa.

### Header

The first screen establishes identity and immediate context:

```text
ROOPA VENKAT
Manager | Last interaction: Weekly 1:1, Tuesday | Next: Friday

You owe Roopa: 2     Waiting on Roopa: 1     Risks: 1
```

Counts are navigation aids, not the content. The important material follows
as readable, evidence-backed items.

### Past, now, next

The primary relationship projection is temporal:

| Area | Questions answered |
|---|---|
| Past | What did we discuss, decide, complete, change, or supersede? |
| Now | What do I owe her, what am I waiting on, and what is at risk? |
| Next | What is due, what conversation is coming, and what preparation is useful? |

```text
PAST                      NOW                       NEXT

Tuesday 1:1               Transition plan          Friday 1:1
Roopa requested...        Owned by Lyndon           Prepare update
[Evidence]                At risk                   [Add to focus]

Architecture decision     Waiting on Roopa          18 August
Agreed to retain...       Confirm headcount         Send proposal
[History]                 Last discussed Tuesday    [Open context]
```

### Relationship actions

Actions are phrased as management questions:

- What do I owe Roopa?
- What am I waiting on?
- What changed since we last met?
- Prepare me for the next 1:1.
- What have I forgotten?

Answers are graph projections with citations, not unsupported generated
summaries. Each answer can be opened into its contributing nodes, paths, and
evidence.

### Privacy boundary

The page is external memory for the manager's relationship. It is not an
employee score, sentiment analysis, covert performance profile, or a reason
to expose unrelated private context. Traversal respects source access and
only surfaces context relevant to the manager's authorized view.

## Progressive graph traversal

### Focus and pivot

The selected node is always the centre of the current exploration. Opening
Roopa initially loads one ring of direct neighbours. Selecting the Weekly
1:1 pivots the centre to that meeting and loads its useful neighbours.
Selecting the Product Docs Archive pivots again.

```text
Roopa Venkat
    |
    +-- attended --> Weekly 1:1
                         |
                         +-- discussed --> Product Docs Archive
                                                  |
                                                  +-- informed --> Migration decision
                                                                          |
                                                                          +-- created --> Migration obligation
```

The existing graph does not disappear on every pivot. The traversed path
remains visible, while newly requested context appears around the new focus.
This gives exploration continuity rather than a sequence of disconnected
detail pages.

### Traversal trail

A persistent trail records how the manager arrived:

```text
Roopa > Weekly 1:1 > Product Docs Archive > Migration decision
```

The manager can go back, jump to any previous focus, pin an important node,
or begin a new trail. Returning to Roopa restores the relationship view and
the relevant exploration state.

### Neighbourhood depth

The default is **one hop**. It is understandable, quick, and usually enough
to choose the next direction.

- **One hop:** direct facts and relationships around the selected node.
- **Two or three hops:** broader context for investigation.
- **Custom depth, including ten hops:** path finding and ranked discovery,
  not an unrestricted visual explosion.

A ten-hop traversal can reach a valuable distant connection, but naïvely
rendering every branch could produce thousands of nodes. The experience
should ask a question such as "find paths from Roopa to the Product Docs
Archive," rank useful paths, and progressively reveal the chosen one.

Depth changes reach. It does not remove relevance, evidence, temporal, or
authorization filters.

### Expansion controls

The exploration surface needs a small set of stable controls:

| Control | Options | Purpose |
|---|---|---|
| Distance | 1, 2, 3, custom | Set traversal reach. |
| Direction | Incoming, outgoing, both | Follow how facts act on or originate from the focus. |
| Time | Current, history, as of date | Separate current truth from previous truth. |
| Lens | Actions, people, meetings, documents, risks, all | Reduce context to the current question. |
| Evidence | Accepted, include suggestions | Control trust posture. |

Expansion can also happen locally: **Show neighbours** on one node adds its
next ring without widening every branch.

### Lenses

A lens is an opinionated traversal policy, not merely a color filter.

- **Actions:** obligations, requests, owners, blockers, dates, and evidence.
- **People:** participants, owners, counterparties, and related meetings.
- **Meetings:** preceding and subsequent interactions, topics, and outcomes.
- **Documents:** archives, source material, decisions informed, and current
  obligations.
- **Risks:** threatened objects, contributing evidence, mitigations, and
  affected outcomes.
- **Why:** the strongest evidence-backed paths explaining why the focused
  object matters.

### Node enrichment

Every focused node becomes a useful object. Its detail surface answers:

1. What is it?
2. Why is it connected to where I started?
3. What happened before?
4. What is currently true?
5. What actions, questions, or risks remain open?
6. What happens next?
7. Which evidence supports this account?

For a Product Docs Archive, that could appear as:

```text
PRODUCT DOCS ARCHIVE

What it is
Documentation repository for the product.

Why it is here
Discussed with Roopa during the 12 August 1:1.

Current context
The migration approach is agreed; ownership remains unresolved.

Connected memory
12 meetings | 4 decisions | 7 obligations | 3 risks

Sources
Transcript quote | Architecture document | Previous decision

[Explore meetings] [Show obligations] [Find a path]
```

Enrichment can combine canonical attributes, accepted neighbouring facts,
temporal history, source excerpts, and derived summaries. Derived text must
remain distinguishable from stored fact and must expose the paths and
evidence used to produce it.

### Visual trust language

The visual treatment communicates epistemic state before decoration:

| Treatment | Meaning |
|---|---|
| Solid relationship | Current, accepted connection. |
| Faded or historical relationship | Superseded or no longer current. |
| Dashed relationship | Suggested or semantically discovered connection. |
| Evidence marker | Direct supporting source is available. |
| Warning marker | Claim is unsupported, contradictory, or needs review. |

Color and shape cannot be the only distinction. Every state also needs a
text label, tooltip, or accessible description.

Selecting an edge explains it as a sentence and shows its temporal and
evidence context:

> Roopa requested the transition plan during Tuesday's 1:1. Lyndon accepted
> ownership. It is expected before Friday.

## Navigation and layout

The relationship projection and graph exploration should feel like two modes
of one workspace, not unrelated product areas.

```text
+----------------------+--------------------------------+------------------+
| TRAIL / CONTEXT      | FOCUSED NODE OR GRAPH          | MEMORY / EVIDENCE|
|                      |                                |                  |
| Roopa                | Progressive neighbourhood      | Why connected    |
| > Weekly 1:1         | around current focus           | Current state    |
| > Product Docs       |                                | History          |
|                      |                                | Sources          |
+----------------------+--------------------------------+------------------+
```

- The centre remains spatially stable while nodes load or expand.
- Selecting a node updates detail without destroying the trail.
- Selecting evidence opens the source without losing graph position.
- Mobile collapses trail and detail into drawers around one primary focus;
  it does not attempt to shrink the full desktop graph into illegibility.
- A list or path representation remains available when visual density makes
  the graph less useful.

## Agent and query behavior

The agent should use the same graph semantics as the web interface. It can:

- Submit a meeting and proposed subgraph.
- Resolve likely existing people and objects before proposing duplicates.
- Ask for the current one-hop context around a node.
- Request a constrained multi-hop path.
- Prepare a relationship brief from accepted facts and cited evidence.
- Propose missing relationships discovered through semantic similarity.

Semantic similarity does not silently become graph truth. It produces a
candidate connection, visibly provisional until accepted or independently
supported.

## Important states

The experience needs deliberate behavior beyond the ideal populated graph:

- **No neighbours:** explain that no accepted relationships exist and offer
  relevant sources or suggestions, rather than showing an empty canvas.
- **Many neighbours:** rank, cluster, and initially collapse lower-value
  branches.
- **Missing evidence:** state "No evidence recorded" plainly.
- **Conflicting evidence:** show both claims and their dates; do not collapse
  them into false certainty.
- **Superseded relationship:** keep it reachable through history while
  excluding it from current truth by default.
- **Unresolved identity:** present possible matches before creating another
  Roopa node.
- **Partial ingestion:** show which stage completed and which proposed items
  remain reviewable.
- **Unauthorized source:** disclose that context is unavailable without
  leaking its contents.

## Non-goals

- Rendering the entire organizational graph at once.
- Making raw node and edge administration the primary experience.
- Treating generated summaries as accepted facts.
- Automatically converting semantic similarity into durable relationships.
- Measuring people through activity, sentiment, graph centrality, or risk
  scores.
- Replacing source systems or copying unrelated archives into Ringmaster.

## Open design questions

These should be resolved through prototypes and later bounded ADRs rather
than assumed by this document:

1. Whether meeting review accepts a claim and its relationships atomically or
   allows individual edge-level acceptance.
2. How much identity resolution happens before review and how ambiguous
   matches are presented.
3. Whether custom traversal depth is a number, a path-finding mode, or both.
4. How graph state and traversal trails persist across sessions.
5. Which node types deserve specialized enrichment layouts.
6. How ranked paths explain why one route was preferred over another.
7. What information can be safely included when a path crosses sources with
   different access or retention rules.
