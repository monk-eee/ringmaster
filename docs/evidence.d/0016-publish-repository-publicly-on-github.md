# EV-0016: Publish the ringmaster repository publicly on GitHub

Evidence for [ADR-0016](../adr.d/0016-publish-repository-publicly-on-github.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0016-publish-repository-publicly-on-github"

[[check]]
id = "readme-names-canonical-repo"
invariant = "Repository guidance documents the canonical public GitHub location."
type = "present"
pattern = 'github\.com/monk-eee/ringmaster'
paths = ["README.md"]

[[check]]
id = "gitignore-excludes-env"
invariant = ".gitignore actually excludes real environment/secret files, not just the example template."
type = "present"
pattern = '^\.env$'
paths = [".gitignore"]

[[check]]
id = "pre-publish-audit-performed"
invariant = "A pre-publish audit found no secrets or People-commitment runtime data in tracked history."
type = "manual"
last_verified = "2026-08-14"
```

## Notes

The pre-publish audit (2026-08-14) covered: `git ls-files` (77 tracked
files at the time), the added-files list of all three existing commits via
`git log --all --diff-filter=A --name-only`, `.env.example`, `compose.yaml`'s
credential defaults, `.vscode/mcp.json`, and a credential/secret-shaped
regex grep (`password|secret|api[_-]?key|token|BEGIN ... PRIVATE KEY`)
across tracked non-Markdown files. No secrets, credentials, tokens, or
runtime/sample data were found. This is a point-in-time manual check;
re-verify (and bump `last_verified`) if a long gap passes before the actual
publish, or periodically thereafter per the checker's staleness threshold.
