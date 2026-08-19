# EV-0091: People view redesign — avatars, status badges, elevated card layout

Evidence for [ADR-0091](../adr.d/0091-people-view-avatar-and-badge-redesign.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0091-people-view-avatar-and-badge-redesign"

[[check]]
id = "people-avatar-present"
invariant = "Person cards and the detail header render a colored initials avatar element."
type = "present"
pattern = 'people-avatar'
paths = ["frontend/src/components/People.tsx", "frontend/public/style.css"]

[[check]]
id = "people-badges-reuse-tokens"
invariant = "At-risk/open badges reuse the existing status color tokens, not new arbitrary colors."
type = "present"
pattern = 'var\(--at-risk-bg\)|var\(--open-bg\)'
paths = ["frontend/public/style.css"]

[[check]]
id = "detail-heading-text-unchanged"
invariant = "The people-detail heading still renders only the person's canonical_text, with the avatar as a sibling, not a child."
type = "present"
pattern = '<h3>\{detail\.canonical_text\}</h3>'
paths = ["frontend/src/components/People.tsx"]

[[check]]
id = "playwright-suite-passes-people-redesign"
invariant = "The full Playwright suite passes against the restyled People tab."
type = "manual"
last_verified = "2026-08-19"
rationale = "`npx playwright test --project=chromium` run against the app after the People redesign; the People-tab describe.serial block and every other spec passed unchanged."
```

## Notes

Implemented entirely in `frontend/src/components/People.tsx` (a small
`initials()`/`avatarClass()` helper pair, and the avatar element added to
the list-card and detail-header JSX) and `frontend/public/style.css` (new
`.people-avatar`/`.people-avatar-lg`/`.people-badge*` rules, plus a
horizontal `.people-card` layout). No route, fetch, or existing
class/heading text changed.
