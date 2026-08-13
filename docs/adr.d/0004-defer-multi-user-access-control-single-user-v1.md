# ADR-0004: Defer multi-user access control; keep sensitive commitment data local and unshared for v1

- **Status:** Accepted
- **Date:** 2026-08-13
- **Decider:** monk-eee
- **Approval:** Explicitly accepted by monk-eee on 2026-08-13
- **Depends on:** [ADR-0001](0001-require-governing-adr-coverage-before-implementation.md)
- **Tags:** security, privacy, access-control, v1-scope

## Context

[docs/VISION.md](../VISION.md) names People Commitments — career development,
promotion readiness, mentoring, recognition, performance conversations — among
what Ringmaster tracks, sourced in part from MCP integrations such as
Outlook, Teams, and SharePoint. That is HR-adjacent data flowing through an
agent-accessible integration layer, and the vision explicitly flags the
sensitive-data boundary as undecided.

monk-eee has confirmed that, for now, Ringmaster is a single-user tool: one
operator, one local Postgres instance, and no other person or service reads
People-commitment data. Designing a full authorization model today would mean
inventing personas, roles, and boundaries with no real second user to design
against — the kind of speculative decision the repository's own ADR
discipline exists to avoid.

Doing nothing is a different risk. Without an explicit record, a later change
— adding cloud sync, a shared dashboard, telemetry export, or a second
operator — could silently start exposing performance and promotion data,
because nothing on record says that requires a decision first.

## Decision

- Ringmaster v1 must operate as a single-user system: one human operator, one
  local Postgres instance. No multi-user authorization model is implemented
  in v1.
- Ringmaster must not sync People-commitment content (performance,
  promotion-readiness, mentoring, recognition data) to any external or cloud
  service, export it to shared logs or telemetry, or make it visible to
  another agent's or user's session — without a new or amending ADR that
  defines an access-control and data-classification model first.
- Ringmaster must not grant a second human or service account read or write
  access to the commitment store without such an ADR.
- This ADR intentionally does not define the eventual authorization model. It
  records the interim single-user assumption and fences the specific actions
  that would break it, so that expansion is a deliberate decision rather than
  an incidental one.

## Scope

**In scope:** the v1 operating assumption (single operator, local-only
storage) and an explicit prohibition on sharing, syncing, or exporting
People-commitment content beyond that operator without a governing decision.

**Out of scope:** the eventual multi-user authorization/RBAC model,
encryption-at-rest mechanics, audit logging design, and the commitment
schema itself (see the forthcoming event-sourced schema ADR).

## Options considered

- **Defer with explicit guardrails (chosen):** avoids designing access control
  against an imagined second user, while still preventing silent scope creep
  into sharing or syncing sensitive content.
- **Design a full RBAC/authorization model now:** more "complete," but with no
  real second persona to validate it against, it is likely to encode the
  wrong boundaries and would need rework once actual multi-user needs exist.
- **No recorded policy at all:** cheapest today, but leaves the sensitive-data
  boundary entirely implicit, exactly the failure mode [ADR-0001](0001-require-governing-adr-coverage-before-implementation.md)
  exists to close for other decisions.

## Consequences

- **Positive:** v1 stays simple and matches its real single-user usage; the
  repository now has an explicit, referenceable guardrail instead of an
  unstated assumption.
- **Negative / trade-off:** this is a policy fence, not a technical control —
  it constrains what an ADR-following contributor or agent should build, not
  what a database permission enforces.
- **Risk:** a change made without consulting governing ADRs could still
  violate this. Mitigated by [ADR-0001](0001-require-governing-adr-coverage-before-implementation.md)'s
  requirement that sync, sharing, or multi-user capabilities each need their
  own governing ADR, and that ADR must reference this one.

## Exit criteria and evidence

Evidence: [EV-0004](../evidence.d/0004-defer-multi-user-access-control-single-user-v1.md)

| Exit criterion | Evidence |
|---|---|
| The vision document names this ADR as addressing the sensitive-data boundary open question | `vision-names-sensitive-data-question` |
| No sync, export, or sharing path for People-commitment content exists outside the single operator | `no-sensitive-data-sharing-path` |
| No second human or service account has commitment-store access | `no-second-account-access` |
