# EV-0007: Generalize the event-sourced aggregate to Obligation and require pgvector

Evidence for [ADR-0007](../adr.d/0007-generalize-obligation-and-require-pgvector.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0007-generalize-obligation-and-require-pgvector"

[[check]]
id = "obligation-events-table-exists"
invariant = "The renamed obligation_events table and its append-only trigger exist."
type = "present"
pattern = 'CREATE TABLE obligation_events'
paths = ["backend/migrations/0001_obligation_events.sql"]

[[check]]
id = "lib-wires-obligation-module"
invariant = "The Rust library wires the renamed obligation module."
type = "present"
pattern = 'pub mod obligation'
paths = ["backend/src/lib.rs"]

[[check]]
id = "lib-no-longer-mentions-commitment"
invariant = "The library entrypoint no longer references the retired commitment name."
type = "absent"
pattern = 'commitment'
paths = ["backend/src/lib.rs"]

[[check]]
id = "pgvector-extension-required"
invariant = "A migration enables the pgvector extension, making it required for the schema to apply at all."
type = "present"
pattern = 'CREATE EXTENSION IF NOT EXISTS vector'
paths = ["backend/migrations/0003_enable_pgvector.sql"]

[[check]]
id = "embeddings-table-exists"
invariant = "A minimal, dimension-unconstrained embeddings table exists per docs/PRODUCT-SPEC.md SS9.2."
type = "present"
pattern = 'CREATE TABLE embeddings'
paths = ["backend/migrations/0003_enable_pgvector.sql"]
```

## Notes

All checks are automated and verified directly against the renamed migration
and crate files. No Rust code yet reads or writes the `embeddings` table —
that is out of scope for this ADR (see its Scope section) and remains
future, ADR-governed work.
