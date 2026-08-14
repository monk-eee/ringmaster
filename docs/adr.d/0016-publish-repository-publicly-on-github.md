# ADR-0016: Publish the ringmaster repository publicly on GitHub

- **Status:** Accepted
- **Date:** 2026-08-14
- **Decider:** monk-eee
- **Approval:** Explicitly accepted by monk-eee on 2026-08-14
- **Depends on:** [ADR-0001](0001-require-governing-adr-coverage-before-implementation.md), [ADR-0004](0004-defer-multi-user-access-control-single-user-v1.md)
- **Tags:** infrastructure, version-control, security, hosting

## Context

ringmaster has existed only as a local git repository on one machine. monk-eee
wants a real hosted remote so version control, history, and review work
properly, using the GitHub account already authenticated locally
(`monk-eee`), with **public** visibility.

[ADR-0004](0004-defer-multi-user-access-control-single-user-v1.md) already
fences People-commitment *runtime data* (performance, promotion-readiness,
mentoring, recognition content flowing through future MCP integrations) to a
single local operator, and forbids syncing or exposing it without its own
governing ADR. This decision is narrower: it is about publishing the
*source repository* — code, migrations, ADRs, evidence, docs — not about
reopening ADR-0004's runtime-data boundary. Nothing in this ADR permits
People-commitment content to leave the local operator's machine.

Because this repository has never been reviewed with "this will be public"
in mind, a pre-publish audit was performed before drafting this ADR:
tracked files (`git ls-files`, all three existing commits' added files),
`.env.example`, `compose.yaml`'s default credentials, `.vscode/mcp.json`,
and a credential/secret-shaped grep across all tracked non-Markdown files.
Result: no secrets, API keys, tokens, private keys, or runtime/sample data
were found anywhere in tracked files or history. `compose.yaml` and
`.env.example` only carry placeholder local-dev defaults (e.g.
`ringmaster-dev`), never real credentials. One real gap was found: despite
`.env.example`'s comment claiming "`.env` is gitignored," `.gitignore` does
not actually list `.env`, so a real `.env` created later could be committed
by an unrelated broad `git add`. No `.env` file exists yet, so nothing has
leaked, but the gap must close before publishing.

## Decision

- The canonical remote for this repository is
  `https://github.com/monk-eee/ringmaster`, created under monk-eee's
  existing GitHub account, with **public** visibility.
- Before the first push, `.gitignore` must actually exclude `.env` (closing
  the gap found above), alongside the local state it already excludes
  (`.mindleak/`, `/target/`, `frontend/node_modules/`, etc.).
- No secret, API key, credential, or ADR-0004-governed People-commitment
  runtime data may ever be committed. Compose/env defaults remain
  placeholder local-dev-only values, never real credentials.
- If a secret is ever committed to the public history, it must be treated as
  compromised and rotated — removing it in a later commit is not sufficient,
  since public history is visible from the moment of push. Rewriting public
  history (e.g. force-push) is a separate, explicitly-confirmed operational
  action, not a routine response.
- The existing local commits are reused as-is; they were included in the
  audit above and contain nothing that blocks publishing.

## Scope

**In scope:** the hosting location and public-visibility decision, the
pre-publish secrets/data audit obligation, and closing the `.gitignore` gap.

**Out of scope:** CI/CD pipelines (own governing ADR), GitHub branch
protection rules and repository settings beyond visibility, issue/PR
labeling conventions, and multi-user access control to the running
application (already governed separately by
[ADR-0004](0004-defer-multi-user-access-control-single-user-v1.md)).

## Options considered

- **Public GitHub repository under monk-eee (chosen):** matches the explicit
  request; the account is already authenticated locally; no new hosting
  relationship to establish.
- **Private GitHub repository:** rejected — monk-eee explicitly requires
  public visibility.
- **A different host (GitLab, Bitbucket, self-hosted):** not requested, and
  offers no benefit over an already-authenticated GitHub account.
- **Stay local-only:** rejected — defeats the explicit goal of "proper
  version control."

## Consequences

- **Positive:** real remote history, a pull-request review surface,
  off-machine backup, and alignment with
  [ADR-0001](0001-require-governing-adr-coverage-before-implementation.md)'s
  own already-recorded expectation of "GitHub pull requests."
- **Negative / trade-off:** public visibility means the full history, ADRs,
  and architecture are visible to anyone, permanently, once pushed.
- **Risk:** a future contributor or agent commits a real secret. Mitigated
  by the closed `.gitignore` gap, the existing pull request template
  checklist item ("No secrets or credentials are committed"), and this
  ADR's rotate-not-just-remove rule.

## Exit criteria and evidence

Evidence: [EV-0016](../evidence.d/0016-publish-repository-publicly-on-github.md)

| Exit criterion | Evidence |
|---|---|
| Repository guidance documents the canonical public repository location | `readme-names-canonical-repo` |
| `.gitignore` actually excludes real environment/secret files | `gitignore-excludes-env` |
| A pre-publish audit found no secrets or People-commitment runtime data in tracked history | `pre-publish-audit-performed` |
