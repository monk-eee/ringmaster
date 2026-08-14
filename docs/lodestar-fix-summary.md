# Lodestar fix summary

Upstream issue: [monk-eee/MindLeak#447](https://github.com/monk-eee/MindLeak/issues/447)

## What is broken

Ringmaster's installed Lodestar `v0.1.5` starts with zero goals and zero tasks.
It cannot load Ringmaster's accepted ADRs from `docs/adr.d/`, so there is no
goal for `task_create` to decompose.

The installed binary also lacks `constitution_define` and
`constitution_query`. MindLeak `main` contains those tools but still reports
version `0.1.5`, making the stale release and fixed source indistinguishable.

The configured model name is stale and returns HTTP 404. This is secondary:
decomposition on current `main` already has a deterministic fallback.

## Already fixed on MindLeak main

- `constitution_define(action="goal")` creates the first goal.
- `constitution_query(action="active")` lists active goals.
- `task_create` without a title decomposes a goal.
- Decomposition reuses exact-title live tasks.
- Model failure produces a deterministic fallback task.

Do not reimplement those changes.

## Source changes still needed

1. Release current `main` as a version newer than `0.1.5`.
2. Include semantic version and build commit in `open_session` or
   `storage_status` so installed behavior is identifiable.
3. Extend `constitution_define` with `action="import"` accepting structured
   records:

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

4. Persist nullable `source_system`, `external_id`, `source_ref`, and
   `source_digest` on goals. Uniquely constrain `(source_system, external_id)`
   when both are present.
5. Make import deterministic:
   - unseen accepted record: `created`;
   - same identity and digest: `unchanged`;
   - proposed, rejected, or unknown status: `skipped`;
   - same identity with a changed digest: `conflict`, without mutation;
   - records omitted from later imports are never deleted or retired.
6. Return counts plus one outcome row per input record. Reject malformed input
   transactionally.
7. Keep `kind` explicit. Only `objective` records may be decomposed; do not
   infer intent type from ADR prose.

Lodestar should consume structured records supplied by the caller. It should
not scan or parse arbitrary Markdown because the source repository owns ADR
status and acceptance semantics.

## Proof

- A published version newer than `0.1.5` exposes goal creation, query,
  decomposition, and claims in the default MCP profile.
- Importing an accepted objective creates one active goal with provenance.
- Repeating the import creates no duplicate goal.
- Repeating decomposition creates no duplicate live task.
- Proposed ADRs remain inactive.
- A changed digest reports conflict and preserves the original goal.
- An unavailable or invalid model still produces one fallback task.
- A fresh repository can import, decompose, and win a claim using MCP only.

The full rationale and acceptance detail remain in
[`mindleak-lodestar-goal-bootstrap-spec.md`](mindleak-lodestar-goal-bootstrap-spec.md).