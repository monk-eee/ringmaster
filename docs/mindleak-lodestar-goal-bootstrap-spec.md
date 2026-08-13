# Spec: close the Lodestar goal-bootstrap gap

> Filed upstream: [monk-eee/MindLeak#447](https://github.com/monk-eee/MindLeak/issues/447).

> Written from a consuming repository (internally called "ringmaster") that
> installs the MindLeak/Lodestar MCP servers as tooling. It has no access to
> the MindLeak/Lodestar source itself, so this is a problem report plus a
> candidate design for whoever maintains that project, not a patch. File
> paths below describe where things live in the *consuming* repository
> unless noted otherwise; adjust to your own source tree.

- **Target:** the MindLeak / Lodestar MCP servers themselves, not any one
  consuming repository's own source, tests, configuration, infrastructure,
  or pipelines.
- **Status:** Proposed (external). No decider has accepted this; it records
  a problem observed while using the servers and a candidate fix.
- **Numbering note:** ADR numbers cited below (e.g. "ADR-0015", "ADR-0029")
  are Lodestar's own internal decision records, quoted verbatim from its own
  MCP tool descriptions. A consuming repository may have a separate,
  unrelated ADR numbering scheme for its own product decisions — the two
  are not the same sequence and should not be conflated.

## Problem

A brand-new, "unborn" repository (no commits, no local Lodestar state
directory) that already has its own mature, machine-checkable decision-record
system (in this case: a `docs/adr.d/` directory of accepted ADRs plus a
dependency-free Node script that derives which ones are currently proven)
cannot make any use of Lodestar's task/claim/conformance workflow, because
Lodestar has no goals and exposes no tool that creates one. The installed
agent skill instructs agents to coordinate and claim work through Lodestar as
a mandatory default loop, so every agent that follows it hits the same dead
end on first use in a fresh repository.

## Reproduction (this session, 2026-08-13)

1. `storage_status` on both planes resolved the same `repository_id`
   (`ec907d00524580d4630b85347bf815cb`) with the Lodestar database at
   `.../repositories/ec907d00524580d4630b85347bf815cb/spec.db` — both planes
   healthy and correctly paired.
2. `lodestar_stats` returned `{"active_goals":0,"active_knowledge":1,
   "claimed_tasks":0,"done_tasks":0,"open_tasks":0}`.
3. `task_query(view="next")` returned `"no claimable task"`;
   `task_query(view="board", include_terminal=false)` returned `[]`.
4. `advise(node_ids=["artifact:backend/src/obligation.rs"])` returned
   `{"disposition":"needs_human","findings":["constitution absent: no active
   policy to advise against"],"governing":[]}`.
5. `task_create(goal_id="adr-0007-obligation-lifecycle-typing", title=...,
   acceptance=...)` — a descriptive, never-before-used goal id — returned the
   hard error `not found: adr-0007-obligation-lifecycle-typing`. It did not
   auto-create the goal.
6. Targeted `tool_search` calls for goal/constitution creation
   ("create goal", "adopt constitution", "fleet_view", "active_knowledge",
   "governing_for_task") returned only read-only or task-scoped tools. No
   tool in the exposed MCP surface creates or seeds a Lodestar goal.
7. `list_dir` on the repository root confirmed no `.lodestar/` directory
   exists (only `.mindleak/`), so no local seed file was ever written either.

Net effect: an agent cannot self-serve past this. The only options observed
were (a) silently abandon Lodestar coordination for ordinary work — which the
installed skill's own workflow guidance does sanction, but only after the
ambiguity is worked out by trial and error, as happened here — (b) query the
SQLite files directly, which the skill explicitly forbids, or (c) stall on a
human for something that is a one-time infrastructure step, not a genuine
per-change judgment call.

## Root cause

Lodestar's Intent Plane models a **goal** as a required prerequisite for a
**task**, and exposes rich tooling once a goal exists (`task_create`,
`task_claim`, `advise`, `check_conformance`, `export_conformance_manifest`,
etc.), but exposes no create/import/seed primitive for the goal object
itself anywhere in the agent-facing MCP surface. Whether that is deliberate
(goals are meant to come from a human-facing UI or CLI outside MCP) is never
documented in the skill materials, so the omission is indistinguishable from
a bug at the point an agent actually hits it.

Two secondary issues compound the same root cause:

- `advise`'s `needs_human` disposition conflates "nothing has ever been
  adopted for this repository" (an infrastructure gap, safe to proceed
  around) with "this specific change is ambiguous" (a real judgment call
  that should stop the agent). Both surface identically, so an agent (or the
  skill instructions telling it to "respect ... needs_human") cannot react
  differently to the two cases without brittle string-matching on
  `findings`.
- There is no positive, symmetrical way to list or count goals themselves
  (a `task_query`-style read for goals). An agent can only infer "zero goals
  exist" indirectly from `lodestar_stats.active_goals` plus empty results
  everywhere else.

## Goals of the fix

- A fresh repository with its own machine-readable decision records can
  reach a claimed Lodestar task with no direct database access and no human
  in the loop, when that is genuinely wanted.
- An agent can tell "nothing adopted yet" apart from "this needs a human
  decision" without parsing free text.
- Setup and troubleshooting docs stop assuming goals already exist.

## Non-goals

- Redesigning the goal/task/claim/conformance model itself. This spec adds a
  missing creation/import path and a diagnosability improvement; it does not
  change any existing invariant (append-only evidence, advisory-not-lock
  claims, ADR-0009 conformance evidence, etc.).
- Deciding *for* any given repository whether it should adopt Lodestar goals
  at all. That stays a per-repository choice.

## Proposed design

1. **`goal_create` tool (minimal fix).** Arguments: `statement` (plain-language
   objective), optional `source_ref` (mirrors `record_knowledge`'s stable
   `/memories/repo/...#heading` convention), optional `scope` (advisory
   paths/symbols). Returns a `goal_id`. Closes the gap directly with the
   smallest possible surface.
2. **`import_goals` tool (repository-shaped fix).** Accepts a list of
   `{external_id, statement, status}` entries — or a glob the server reads
   itself — and idempotently creates or updates one goal per entry, keyed by
   `external_id` so re-running it is a no-op when nothing changed. It must
   only ingest entries whose `status` is an accepted/terminal-approved state
   (mirroring how this consuming repository's own evidence checker only
   treats `Status: Accepted` records as governing), so a still-proposed
   record never silently becomes a binding goal. A repository like this one
   could then seed Lodestar directly from its own accepted decision records.
3. **Structured `advise` reason.** Split `needs_human` into a machine-checked
   field distinguishing `no_constitution_adopted` (zero goals repo-wide —
   safe to proceed with ordinary, ungoverned work per the skill's own
   fallback) from `ambiguous` (goals exist; this node's governance is
   genuinely unclear — a real stop condition). Keep the existing disposition
   for backward compatibility; add the reason as an additional field rather
   than replacing it.
4. **`goal_query` read tool.** Symmetric with `task_query(view="board")`, so
   an agent can positively confirm what goals exist (or that none do)
   instead of inferring it from a count and several empty results.
5. **Docs.** Add a step to the setup/verification guide's "Verify End to
   End" checklist: confirm at least one goal exists, or explicitly
   import/seed goals, before relying on task coordination. Add a row to the
   troubleshooting reference: symptom `task_create` → `not found:
   <goal_id>`; cause: no goal exists with that id and none have ever been
   seeded for this repository; fix: run the goal-creation/import tool
   first. Clarify the working-loop guidance so "no task exists" explicitly
   covers "goals were never seeded for this repository" as a
   proceed-with-ordinary-work case, not a stall.

## Alternatives considered

- **Do nothing; document the gap only (what this session did in repo
  memory).** Cheapest, and already done as a stopgap, but every new
  repository, and every agent that has not read that memory note, hits the
  identical dead end again.
- **Let agents write Lodestar's SQLite directly to seed a goal.** Rejected:
  the skill explicitly forbids querying either plane's SQLite files
  directly, and bypassing the server would skip whatever invariants goal
  creation is supposed to enforce.
- **Auto-create a goal on first `task_create` call with an unknown id.**
  Rejected as the primary fix: it would silently manufacture governance
  structure from whatever string an agent happened to pass, with no
  statement, provenance, or acceptance status — the opposite of Lodestar's
  own evidence-first design elsewhere.

## Acceptance criteria

- A fresh, goal-less repository with its own accepted decision records can
  go from `open_session` to a won `task_claim` with no direct SQLite access
  and no human decision, using only documented MCP tools.
- `advise` responses expose a structured reason distinguishing "nothing
  adopted" from "ambiguous."
- A read tool lists/confirms goal existence without inferring it from
  aggregate stats plus empty task queries.
- The setup and troubleshooting reference docs each name this gap and its
  fix.

## Supporting evidence already recorded

The reproduction steps above are quoted directly from a live session against
a real installation (see "Reproduction" section). The consuming repository
also recorded the gap and the interim workaround (proceed without a Lodestar
task; govern changes through its own decision-record system instead) in its
own local agent memory, so its other agents do not re-discover the same dead
end. That record is local to the consuming repository's tooling and is not
reproduced here since it carries no meaning outside that installation.
