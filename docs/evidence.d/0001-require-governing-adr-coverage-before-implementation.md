# EV-0001: Require governing ADR coverage before implementation

Evidence for [ADR-0001](../adr.d/0001-require-governing-adr-coverage-before-implementation.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0001-require-governing-adr-coverage-before-implementation"

[[check]]
id = "agent-guidance-requires-governing-adr"
invariant = "Always-on agent guidance requires an accepted governing ADR before implementation."
type = "present"
pattern = 'must identify and read an accepted governing ADR'
paths = ["AGENTS.md"]

[[check]]
id = "github-review-declares-governing-adr"
invariant = "The GitHub pull request workflow requires a governing ADR or valid editorial exemption."
type = "present"
pattern = 'Required: link the accepted ADR'
paths = [".github/pull_request_template.md"]

[[check]]
id = "repository-guidance-links-adrs"
invariant = "Repository guidance links contributors to the ADR collection and lifecycle."
type = "present"
pattern = 'docs/adr\.d/'
paths = ["README.md", "docs/CONTRIBUTING.md"]
```