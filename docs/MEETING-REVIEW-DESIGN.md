# Ringmaster -- Meeting Review Design

**Status:** Working product-design intent, 2026-08-14. This document is not
an ADR and does not govern implementation. It develops the meeting-validation
experience described by [RELATIONSHIP-GRAPH-DESIGN.md](RELATIONSHIP-GRAPH-DESIGN.md)
and [PRODUCT-SPEC.md](PRODUCT-SPEC.md). Implementation requires accepted,
bounded ADR coverage.

## Purpose

The Meeting Review is where a transcript becomes trusted organizational
memory.

An agent can identify possible requests, obligations, decisions, risks,
dates, people, and relationships. It cannot silently decide which claims the
manager should trust. The review experience keeps interpretation beside the
source and makes validation quick enough to become part of normal meeting
follow-up.

The target outcome is:

> I can review what Ringmaster believes happened, verify each important claim
> against the exact words, correct mistakes, and know where accepted memory
> will appear next.

It is not a transcript editor, generic annotation tool, or model-debugging
console.

## Entry points

A meeting can enter review from:

- The response to a successful agent, CLI, or future MCP ingestion.
- A meeting awaiting validation in a review queue.
- A Meeting node reached through relationship or graph traversal.
- A candidate link from the Daily Brief or another attention surface.

All entry points open the same meeting workspace and preserve a return path
to the originating context.

## Review workspace

The desktop experience uses two synchronized primary panes and a compact
meeting header:

```text
+-----------------------------------------------------------------------+
| WEEKLY 1:1 -- LYNDON / ROOPA            Tue 14 Aug | 7 proposals      |
| 4 unreviewed | 2 accepted | 1 deferred                [Finish review]  |
+----------------------------------+------------------------------------+
| TRANSCRIPT                       | PROPOSED MEMORY                    |
|                                  |                                    |
| 10:04 Roopa                      | REQUEST                            |
| Can you bring me a transition    | Roopa requested a transition plan |
| plan next Friday?                | by Friday.                         |
| ~~~~~~~~~~~~~~~~~ highlighted    |                                    |
|                                  | People: Roopa -> Lyndon            |
| 10:05 Lyndon                     | Due: Friday [explicit]             |
| Yes, I will draft it.            | Evidence: 2 transcript passages    |
|                                  |                                    |
|                                  | [Accept] [Correct] [More]          |
+----------------------------------+------------------------------------+
| ACCEPTED MEMORY PREVIEW: Roopa > requested > Transition plan          |
+-----------------------------------------------------------------------+
```

The transcript remains readable as a conversation. The proposed-memory pane
is organized around claims, not around raw entity tables. A narrow preview
shows the graph memory that acceptance would create or update.

The screen does not place the transcript, every proposal, a graph, and a
large metadata inspector at equal visual weight. Evidence and the current
claim dominate; deeper graph detail is progressive.

## Meeting header

The header contains only orientation and review progress:

- Meeting title, date, and participants.
- Source and ingestion state.
- Counts for unreviewed, accepted, corrected, rejected, and deferred claims.
- A finish-review action.

Progress counts indicate work remaining; they are not quality scores. The
header does not show token counts, model latency, or extraction internals.
Those belong in technical diagnostics, not the manager's workflow.

## The review unit: an evidence-backed claim bundle

The primary review unit is a **claim bundle**: one readable management fact
plus the nodes and relationships required to express it.

```text
Claim
Roopa requested a transition plan from Lyndon by Friday.

Bundle
Roopa -- requested --> Transition plan
Transition plan -- owned by --> Lyndon
Transition plan -- expected by --> Friday
Transition plan -- originated from --> Weekly 1:1
Transition plan -- supported by --> Transcript passages 18 and 19
```

This avoids asking the manager to validate five disconnected graph writes
when the actual judgment is whether one request occurred. The bundle can be
expanded to inspect or correct its parts before acceptance.

Acceptance applies to the coherent claim and its required provenance
relationships. Optional or weakly inferred relationships remain separate
suggestions. For example, accepting the request does not automatically accept
a semantic link to an unrelated service merely because the model suggested
one.

This bundle behavior is design intent. Its storage and transaction semantics
need their own accepted ADR before implementation.

## Proposal card anatomy

Every proposal presents:

1. **Type** -- request, obligation, decision, risk, expectation, or follow-up.
2. **Plain-language statement** -- readable without graph vocabulary.
3. **People and direction** -- who asked, owns, owes, waits, or is affected.
4. **Time** -- hard date, soft date, recurrence, or no date found.
5. **Evidence** -- exact supporting passages and speakers.
6. **Inference state** -- explicit, inferred, ambiguous, or contradictory.
7. **Existing memory** -- likely duplicate, update, contradiction, or new.
8. **Review state** -- unreviewed, accepted, corrected, merged, rejected, or
   deferred.

Confidence is secondary to evidence. It can help sort uncertain claims but
must not become the main visual signal or a substitute for explanation.

## Transcript synchronization

Evidence and claims move together:

- Selecting a proposal scrolls the transcript to its first supporting
  passage and highlights every passage used by the claim.
- Selecting highlighted transcript text focuses the claims derived from it.
- Multiple claims may share a passage; the interface lists all without
  implying that one acceptance accepts the others.
- A claim spanning separate turns shows each passage in sequence and preserves
  omitted context between them.
- The manager can expand surrounding transcript context without changing the
  evidence span recorded for the claim.
- Search within the transcript changes navigation only; it does not create or
  validate claims.

Highlights use more than color: selected evidence has a border, marker, and
accessible relationship to the focused proposal.

## Review actions

### Accept

Accept confirms the material fields in the claim bundle and its supporting
evidence. The resulting preview changes from proposed to accepted memory. The
action states where the item will surface, such as Roopa's relationship page
or the Future Risk Horizon.

### Correct

Correct opens structured controls for the material fields relevant to the
claim type:

- Statement.
- Type.
- Actor, owner, counterparty, or affected person.
- Direction of the relationship.
- Hard or soft date and whether it was inferred.
- Evidence passages.

The original extraction remains in history. Correction creates reviewed
memory; it does not rewrite the transcript or pretend the model proposed the
corrected form.

### Merge

Merge appears when existing memory is plausibly the same thing. The manager
sees both objects, their evidence, current status, and the effect of merging.
The action adds new evidence or updates the accepted object according to its
own event rules; it does not silently discard either history.

### Split

Split handles one proposal containing separate judgments, for example:

> Roopa requested a transition plan and warned that ownership is unclear.

This can become a Request and a Risk with shared evidence. Each resulting
claim returns to the unreviewed queue independently.

### Reject

Reject marks the proposal as not representing useful management memory. An
optional reason can distinguish extraction error, duplicate, irrelevant
detail, unsupported inference, or sensitive content. The evidence and model
proposal remain auditable but do not enter accepted graph truth.

### Defer

Defer is for a credible but unresolved claim. It remains in the meeting's
review queue and does not appear as accepted fact in relationship or attention
views. The manager can add a short question such as "Was Friday a deadline or
the next check-in?"

## Review flow

The default ordering reduces attention cost:

1. High-impact or time-bound claims.
2. Claims involving the manager's own obligations.
3. Decisions and explicit requests.
4. Risks and contradictions.
5. Lower-confidence context and optional semantic relationships.

The manager can switch to transcript order, type, person, or review state.
Changing sort never changes validation state.

After an action, focus moves to the next unreviewed claim while preserving
transcript position when the next claim uses nearby evidence. Undo is
available for the most recent local action until the workspace refreshes; a
durable reversal after that follows the underlying event/history model rather
than deleting history.

## Finish review

Finishing does not require every proposal to be accepted or rejected.
Deferred claims are legitimate. The confirmation summarizes:

```text
Meeting review complete

3 accepted
1 corrected
1 merged with existing memory
1 rejected
1 deferred for clarification

New attention
- Transition plan due Friday
- Ownership risk needs clarification
```

The manager can then open the Meeting node, Roopa relationship page, or the
newly affected attention view.

## Accepted-memory preview

The preview prevents validation from feeling like an opaque database action.
Before confirmation it shows the proposed local graph:

```text
Roopa -- requested --> Transition plan -- expected by --> Friday
                                  |
                                  +-- owned by --> Lyndon
```

After acceptance it shows where the memory now contributes:

- Roopa: request and next-conversation context.
- Lyndon: open obligation.
- Friday: Future Risk Horizon.
- Weekly 1:1: source history.
- Daily Brief: eligible when attention rules rank it.

The preview does not promise that every accepted claim immediately appears in
the Daily Brief. It explains eligibility and connection, not ranking.

## Existing-memory comparison

Entity resolution is part of review, not hidden cleanup. When the system finds
possible matches, it presents a compact comparison:

```text
POSSIBLE EXISTING MEMORY

Transition plan              Proposed transition plan
Open                          New candidate
Discussed 7 Aug               Discussed 14 Aug
Due date not recorded         Friday
Evidence: previous 1:1        Evidence: current 1:1

[Merge and add evidence] [Keep separate] [Not the same]
```

Names alone are weak identity evidence. People matches include available
stable identifiers, role, organization, and previous relationship context.
The interface never silently merges two people because their display names
match.

## Contradictions and temporal change

Later meetings may change earlier memory. The review shows both claims and
their dates:

```text
CURRENT MEMORY
Transition plan expected Friday.

NEW EVIDENCE
Roopa: "Let's move that to the end of the month."

Suggested change
Supersede Friday with 31 August.
```

Accepting the new claim closes or supersedes the previous temporal
relationship according to its governing decision. It does not erase the old
expectation or rewrite the earlier meeting.

## Agent handoff

An agent can submit the meeting and trigger extraction, then return a review
link or Meeting id. It may summarize what awaits review:

> Ingested Weekly 1:1 with Roopa: 14 transcript fragments and 7 proposed
> claims. Two contain explicit dates; one may contradict an existing due date.

The agent cannot report proposals as accepted obligations. It distinguishes
stored evidence, model proposals, and human-accepted memory in every response.

## Empty, partial, and failure states

- **No proposals:** the transcript remains available. State plainly that no
  reviewable management claims were extracted; offer manual claim creation or
  explicit re-extraction.
- **No transcript turns:** retain meeting metadata and explain that no usable
  evidence fragments were created.
- **Model unavailable:** ingestion remains successful; extraction can be
  retried later without re-ingesting the meeting.
- **Partial extraction:** show completed fragments and failed fragments
  separately. Do not label the meeting fully reviewed.
- **Unsupported claim:** show the proposal with "No supporting passage found"
  and default it toward rejection, never acceptance.
- **Ambiguous person:** block acceptance of the person relationship until the
  manager resolves or deliberately creates the identity.
- **Conflicting claims:** group them for comparison rather than presenting two
  independent accept buttons with no warning.
- **Already reviewed:** open in history mode with reviewed states, corrections,
  and later superseding evidence visible.
- **Sensitive passage:** allow exclusion from derived memory while preserving
  source-handling policy and audit requirements.

## Responsive and accessible behavior

On narrow screens, transcript and proposals become two tabs sharing the same
focused claim. Switching tabs preserves selection and scroll position. The
accepted-memory preview becomes a drawer. The product does not squeeze both
desktop panes into unreadable columns.

All actions are reachable by keyboard. Focus order follows header, proposal,
actions, evidence, and correction controls. Review state, confidence,
temporal state, and evidence selection are communicated with text and
semantics, not color alone.

## Non-goals

- Editing or cleaning the source transcript.
- Showing raw model prompts or token diagnostics in the primary review flow.
- Accepting every extraction in one unexamined bulk action.
- Turning confidence into an automatic acceptance threshold.
- Automatically merging people or obligations on semantic similarity alone.
- Treating deferred claims as accepted truth.
- Requiring a graph expert to validate ordinary management language.

## Candidate implementation slices

The design should be implemented through bounded decisions rather than one
large "Meeting Review" ADR:

1. Network-reachable atomic meeting ingestion.
2. Meeting detail read with ordered transcript fragments.
3. Meeting-scoped candidate listing and extraction progress.
4. Synchronized transcript and proposal selection.
5. Claim acceptance/rejection using existing candidate transitions.
6. Structured correction, merge, split, and defer semantics.
7. Accepted-memory preview and downstream relationship links.
8. Provider identity, idempotency, and real transcript formats.

## Open design questions

1. Which relationships are required parts of a claim bundle and which remain
   separately reviewable suggestions?
2. Should accepting a corrected claim be one action or correction followed by
   a separate acceptance?
3. How long should local undo remain available before durable reversal rules
   take over?
4. What minimum stable identifiers are required before two Person nodes can be
   merged?
5. How should partial extraction retries avoid duplicate candidates?
6. Which sensitive-content exclusions remove derived memory versus only hide
   it from a particular view?
