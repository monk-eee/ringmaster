# EV-0068: Add an optional API key to the embedding adapter

Evidence for [ADR-0068](../adr.d/0068-embedding-adapter-optional-api-key.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0068-embedding-adapter-optional-api-key"

[[check]]
id = "embedding-config-carries-optional-api-key"
invariant = "EmbeddingConfig reads an optional, non-blank RINGMASTER_EMBEDDING_API_KEY."
type = "present"
pattern = 'api_key: Option<String>[\s\S]*RINGMASTER_EMBEDDING_API_KEY'
paths = ["backend/src/embedding_adapter.rs"]

[[check]]
id = "embedding-auth-header-is-conditional"
invariant = "embed sends bearer authentication when an API key is present and no Authorization header when it is absent."
type = "present"
pattern = 'if let Some\(api_key\) = &config\.api_key \{[\s\S]*bearer_auth\(api_key\)'
paths = ["backend/src/embedding_adapter.rs"]

[[check]]
id = "embedding-keyless-posture-is-preserved"
invariant = "An unset embedding URL still disables the adapter cleanly, and a configured keyless endpoint remains valid."
type = "manual"
last_verified = "2026-08-17"
rationale = "The existing unconfigured-URL test remains in the adapter. Focused request-capture coverage also completed successfully with no API key and verified that no Authorization header was sent."

[[check]]
id = "embedding-api-key-is-configurable"
invariant = "Compose passes through RINGMASTER_EMBEDDING_API_KEY and .env.example documents hosted and keyless 768-dimensional options without a credential value."
type = "present"
pattern = "RINGMASTER_EMBEDDING_API_KEY"
paths = ["compose.yaml", ".env.example"]

[[check]]
id = "embedding-auth-tests-pass"
invariant = "Focused adapter tests prove authenticated and keyless request headers."
type = "manual"
last_verified = "2026-08-17"
rationale = "The focused embedding_adapter_auth integration binary completed twice through Unit Test MCP with PASSED status. It covers a configured bearer token, a keyless request, and non-blank versus blank environment configuration."
```

## Notes

The Unit Test MCP custom runner reports process status for Rust but does not
emit parsed per-test counts or a coverage artifact. Both focused runs returned
PASSED; compile diagnostics were also clean.