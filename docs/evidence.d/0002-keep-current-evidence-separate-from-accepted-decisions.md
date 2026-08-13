# EV-0002: Keep current evidence separate from accepted decisions

Evidence for [ADR-0002](../adr.d/0002-keep-current-evidence-separate-from-accepted-decisions.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0002-keep-current-evidence-separate-from-accepted-decisions"

[[check]]
id = "every-live-adr-has-evidence"
invariant = "Every accepted, non-superseded ADR has an exact-name evidence record."
type = "parity"

[[check]]
id = "checker-is-declarative"
invariant = "The dependency-free Node checker derives evidence state without executing shell commands from evidence records."
type = "absent"
pattern = 'node:(child_process|vm)|\b(eval|exec(File)?|spawn|fork)(Sync)?\s*\(|new\s+Function\s*\('
paths = ["scripts/check-evidence.mjs"]

[[check]]
id = "agent-guidance-requires-checker"
invariant = "Always-on agent guidance requires the checker before reporting evidence state."
type = "present"
pattern = 'must run `node scripts/check-evidence\.mjs` before reporting'
paths = ["AGENTS.md"]
```