# EV-0006: Local development stack runs via Podman Compose

Evidence for [ADR-0006](../adr.d/0006-local-development-stack-runs-via-podman-compose.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0006-local-development-stack-runs-via-podman-compose"

[[check]]
id = "compose-defines-postgres"
invariant = "compose.yaml at the repository root defines a postgres service."
type = "present"
pattern = '\n {2}postgres:'
paths = ["compose.yaml"]

[[check]]
id = "compose-defines-backend"
invariant = "compose.yaml at the repository root defines a backend service."
type = "present"
pattern = '\n {2}backend:'
paths = ["compose.yaml"]

[[check]]
id = "docs-name-podman-as-runtime"
invariant = "Local development documentation names Podman as the primary supported container runtime."
type = "present"
pattern = 'Podman is the primary supported container runtime'
paths = ["README.md"]
```

## Notes

All three checks are automated and verified against the actual `compose.yaml`
and `README.md § Local development`. The stack was built and run with
`podman compose up -d`: Postgres reported healthy, the backend logged a
successful connection and migration run, and the append-only trigger was
confirmed by hand to reject both `UPDATE` and `DELETE` against an inserted
event row.
