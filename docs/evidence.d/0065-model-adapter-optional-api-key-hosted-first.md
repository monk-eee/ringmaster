# EV-0065: Optional API key for the model adapter — hosted models first, Ollama as an option

Evidence for [ADR-0065](../adr.d/0065-model-adapter-optional-api-key-hosted-first.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0065-model-adapter-optional-api-key-hosted-first"

[[check]]
id = "config-carries-optional-api-key"
invariant = "ModelConfig carries an optional api_key read from RINGMASTER_LLM_API_KEY."
type = "present"
pattern = "RINGMASTER_LLM_API_KEY"
paths = ["backend/src/model_adapter.rs"]

[[check]]
id = "auth-header-sent-only-when-key-present"
invariant = "complete attaches a bearer Authorization header only when an api_key is present."
type = "present"
pattern = "bearer_auth"
paths = ["backend/src/model_adapter.rs"]

[[check]]
id = "unconfigured-path-still-returns-none"
invariant = "from_env still returns None when RINGMASTER_LLM_URL is unset (extraction stays cleanly disabled)."
type = "present"
pattern = 'env::var\("RINGMASTER_LLM_URL"\)\.ok\(\)\?'
paths = ["backend/src/model_adapter.rs"]

[[check]]
id = "compose-passes-api-key-through"
invariant = "compose.yaml passes RINGMASTER_LLM_API_KEY through to the backend."
type = "present"
pattern = "RINGMASTER_LLM_API_KEY"
paths = ["compose.yaml"]

[[check]]
id = "env-example-documents-hosted-first"
invariant = ".env.example documents the hosted API-key option."
type = "present"
pattern = "RINGMASTER_LLM_API_KEY"
paths = [".env.example"]
```

## Notes

`cargo test` retains ADR-0011's two adapter tests unchanged — the
unreachable-endpoint path (`complete` returns a typed error, never panics)
and the live round-trip (self-skips when no model is configured) — plus a
new `from_env` test that a present `RINGMASTER_LLM_API_KEY` is read into
`ModelConfig.api_key` and an absent one leaves it `None`. The bearer header
is attached via reqwest's `bearer_auth` only when `api_key` is `Some`;
keyless Ollama sends no auth header, unchanged. Verified live this session:
the ADO-work-item fragment extracted a real candidate (`decision`,
confidence 1.0) end-to-end through this adapter.
