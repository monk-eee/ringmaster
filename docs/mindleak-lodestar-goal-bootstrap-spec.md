# Spec: ship Lodestar bootstrap and import repository ADRs

> Filed upstream: [monk-eee/MindLeak#447](https://github.com/monk-eee/MindLeak/issues/447).
> Follow-up verified against MindLeak `main` at
> `d5ec3db62cbdb9248f2f32d65cdedadcb9bf1e26` on 2026-08-14.

> Written from a consuming repository (internally called "ringmaster") that
> installs the MindLeak/Lodestar MCP servers as tooling. It has no access to
> the MindLeak/Lodestar source itself, so this is a problem report plus a
> candidate design for whoever maintains that project, not a patch. File
> paths below describe where things live in the *consuming* repository
> unless noted otherwise; adjust to your own source tree.

- **Target:** the MindLeak / Lodestar MCP servers themselves, not any one
  consuming repository's own source, tests, configuration, infrastructure,
  or pipelines.
- **Status:** Partially resolved upstream. Manual first-goal bootstrap and
  idempotent decomposition exist on `main`; release packaging and repository
  ADR import remain unresolved.
- **Numbering note:** ADR numbers cited below (e.g. "ADR-0015", "ADR-0029")
  are Lodestar's own internal decision records, quoted verbatim from its own
  MCP tool descriptions. A consuming repository may have a separate,
  unrelated ADR numbering scheme for its own product decisions — the two
  are not the same sequence and should not be conflated.

## Problem

A repository can now create its first goal manually on MindLeak `main` through
`constitution_define(action="goal")`, inspect it through
`constitution_query(action="active")`, and decompose an objective through
`task_create` without a title. Decomposition falls back to one deterministic
task when the model is unavailable and reuses exact-title live work when run
again.

That fix is not usable from the consuming repository for two reasons:

1. The installed binaries and upstream `main` both report version `0.1.5`, but
   the installed `v0.1.5` tool surface predates `constitution_define` and
   `constitution_query`. There is no release boundary by which an installer or
   operator can distinguish the fixed build from the stale one.
2. Lodestar still does not import a repository's existing decision records.
   This repository has accepted ADRs under `docs/adr.d/`, but the Intent Plane
   remains at zero goals until every record is manually re-entered. The ADRs
   are therefore not "loading", and there is no goal id for `task_create` to
   decompose.

The second problem is not solved by making Lodestar scan arbitrary Markdown.
The consuming repository already owns status parsing and acceptance semantics.
Lodestar needs a structured, provenance-bearing import boundary.

## Current source state

The original bootstrap diagnosis below remains useful as a reproduction for
the installed binary, but it is no longer an accurate description of upstream
`main`:

- `crates/lodestar-mcp/src/tools/mod.rs` includes `constitution_define` and
  `constitution_query` in `DEFAULT_PROFILE_TOOLS` and tests the complete path
  from an empty store to a won claim.
- `crates/lodestar-mcp/src/tools/constitution.rs` dispatches
  `constitution_define(action="goal")` to `define_goal`.
- `crates/lodestar-core/src/facade/executive.rs::decompose_goal` is model
  optional and idempotent over exact-title live tasks.
- `changelog.d/fixed-fresh-repository-goal-bootstrap.md` and
  `changelog.d/fixed-idempotent-goal-decomposition.md` describe those fixes.
- The workspace package version is still `0.1.5`, which is also the version
  installed in this consuming repository.

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

The original root cause was that goal authoring existed but was omitted from
the default MCP profile. Upstream `main` fixes that omission.

The remaining root causes are now narrower:

- release identity does not change when the executable contract changes, so a
  fixed and stale `0.1.5` are operationally indistinguishable;
- goal creation accepts only one manually authored goal and carries no stable
  external identity, source reference, or source digest, so it cannot support
  an idempotent import from an existing governance system;
- no MCP operation accepts structured external decisions, validates their
  accepted state, and reports created, unchanged, conflicted, and skipped
  records.

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

- Publish the already-implemented bootstrap/decomposition behavior under a new,
  distinguishable release and install it successfully in a consuming repo.
- A repository with machine-readable decision records can idempotently import
  accepted records without direct database access or lossy Markdown parsing in
  Lodestar.
- An imported implementation objective can be decomposed and claimed through
  the default MCP profile.
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

1. **Cut a real release.** Bump the workspace version to at least `0.1.6`,
  publish macOS arm64 binaries, and make `open_session` or `storage_status`
  expose both semantic version and build commit. The installer must refuse to
  call two different binaries the same installed version.
2. **Add structured import to the existing constitution vocabulary.** Extend
  `constitution_define` with `action="import"`; do not add another top-level
  tool merely to bypass the collapsed ADR-0059 vocabulary. The caller passes
  records, not a filesystem glob:

  ```json
  {
    "action": "import",
    "source_system": "ringmaster-adr",
    "records": [{
     "external_id": "ADR-0022",
     "kind": "objective",
     "title": "Daily Brief endpoint",
     "statement": "Expose obligations ranked by urgency.",
     "status": "accepted",
     "source_ref": "docs/adr.d/0022-daily-brief-endpoint.md",
     "source_digest": "sha256:..."
    }]
  }
  ```

3. **Persist external identity and provenance.** Add nullable
  `source_system`, `external_id`, `source_ref`, and `source_digest` fields to
  goals, with a unique constraint on `(source_system, external_id)` when both
  are present. Manually authored goals remain valid with all four fields null.
4. **Make import deterministic and idempotent.** For each record:
  - `accepted` plus unseen external identity creates one active goal;
  - the same external identity and digest returns `unchanged` with its goal id;
  - `proposed`, `rejected`, or unknown status returns `skipped` and creates
    nothing;
  - the same external identity with a different digest returns `conflict` and
    does not rewrite or supersede active intent;
  - absence from a later batch never deletes or retires a goal.
5. **Return a bounded import report.** The response contains counts and one row
  per supplied record with `external_id`, `outcome`, `goal_id` when known, and
  an actionable reason. One malformed record rejects the whole transaction;
  semantic conflicts are reported without partially rewriting existing goals.
6. **Keep kind explicit.** Lodestar must not infer whether an ADR is an
  objective, constraint, or invariant from prose. Only objectives can be
  decomposed; importing a normative record and then asking to decompose it
  must retain the current actionable refusal.
7. **Keep the model optional.** A missing or misconfigured model must continue
  to produce the deterministic single-task fallback with
  `model_call.source="fallback"`. Model health failure is diagnostic, not a
  blocker for decomposition.
8. **Use the existing reads.** `constitution_query(action="active")` is the
  positive list/read surface. `advise.reason="no_constitution_adopted"` already
  provides the structured absent-state distinction on `main`; retain both.
9. **Update setup and troubleshooting docs.** Verification must assert the
  running build version/commit, list active goals, import when the repository
  owns external governance, decompose one objective, and repeat both import
  and decomposition to prove no duplicate goal or task appears.

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

- A published version newer than `0.1.5` advertises `constitution_define`,
  `constitution_query`, `task_create`, and `task_claim` in the default profile.
- `storage_status` or `open_session` identifies that release and its build
  commit; the consuming repository no longer runs an ambiguous `0.1.5`.
- A fresh repository imports an accepted objective and goes from
  `open_session` to a won `task_claim` using only documented MCP tools.
- Repeating the same import creates no second goal; repeating decomposition
  creates no second live task and reports `reused: true`.
- Proposed ADRs do not become active goals.
- A changed digest for an already-imported ADR is reported as a conflict and
  leaves the original goal untouched.
- A 404, unreachable, or malformed model response still yields one fallback
  task and identifies the fallback reason.
- `advise` responses expose a structured reason distinguishing "nothing
  adopted" from "ambiguous."
- `constitution_query(action="active")` lists the imported goal with its
  external identity and provenance.
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
