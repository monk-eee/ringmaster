# ADR-0068: Add an optional API key to the embedding adapter

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Direct instruction ("accepted now work on it"), 2026-08-17
- **Amends:** [ADR-0018](0018-generate-and-store-source-fragment-embeddings.md)'s embedding adapter configuration with optional bearer authentication.
- **Depends on:** [ADR-0018](0018-generate-and-store-source-fragment-embeddings.md), [ADR-0065](0065-model-adapter-optional-api-key-hosted-first.md)
- **Tags:** architecture, embeddings, model-adapter, configuration, security

## Context

[ADR-0065](0065-model-adapter-optional-api-key-hosted-first.md) made hosted
OpenAI-compatible chat models usable by adding an optional bearer token to the
model adapter. It explicitly left the embedding adapter's identical no-auth
limitation for a separate decision. `EmbeddingConfig` still carries only a URL
and model, and `embed` never sends an `Authorization` header. Ringmaster can
therefore use keyless local Ollama embeddings but not a hosted embedding
endpoint that requires a bearer token.

The storage decision in [ADR-0018](0018-generate-and-store-source-fragment-embeddings.md)
remains binding: `embeddings.embedding` is `vector(768)`. Authentication does
not make an arbitrary hosted model compatible. A configured hosted model must
return exactly 768 dimensions unless a future ADR changes the schema.

## Decision

- `EmbeddingConfig` gains `api_key: Option<String>`, read from
  `RINGMASTER_EMBEDDING_API_KEY`. A missing or whitespace-only value becomes
  `None`.
- `embed` attaches `Authorization: Bearer <key>` only when `api_key` is
  present. With no key it sends no authorization header, preserving keyless
  Ollama behavior.
- `EmbeddingConfig::from_env` still returns `None` when
  `RINGMASTER_EMBEDDING_URL` is unset. The optional adapter remains disabled
  cleanly rather than blocking ingestion, storage, or retrieval.
- `compose.yaml` passes `RINGMASTER_EMBEDDING_API_KEY` through without a
  default credential. `.env.example` documents both a hosted
  OpenAI-compatible endpoint and the existing keyless Ollama option, including
  the fixed 768-dimension requirement.
- A focused adapter test captures a real local HTTP request and proves the
  bearer header is present with a key and absent without one. The test uses the
  existing toolchain and adds no production or test dependency.
- The key is configuration-only secret material. It must not be logged,
  persisted, committed, or included in errors.

## Scope

**In scope:** optional embedding API-key configuration, conditional bearer
authentication, Compose pass-through, local configuration guidance, and
focused request-header coverage.

**Out of scope, named honestly:** changing the fixed 768-dimensional schema;
automatically adapting dimensions; a provider registry; sharing the chat-model
key implicitly; per-request provider selection; credential validation; retry
or rate-limit policy; and storing secrets anywhere other than the process
environment.

## Options considered

- **A separate optional embedding key (chosen):** mirrors ADR-0065 while
  preserving the independently configurable chat and embedding adapters from
  ADR-0018.
- **Reuse `RINGMASTER_LLM_API_KEY`:** rejected because chat and embedding
  endpoints may use different providers, credentials, and security scopes.
- **Require a key whenever embeddings are configured:** rejected because it
  breaks local Ollama and other keyless OpenAI-compatible endpoints.
- **Add provider-specific configuration:** rejected because the current
  OpenAI-compatible request shape only lacks optional authentication; a
  provider enum would add machinery without solving another demonstrated gap.

## Consequences

- **Positive:** hosted OpenAI-compatible embedding endpoints that return 768
  dimensions become usable by every existing ingest, reindex, HTTP search, and
  MCP search path through the shared adapter.
- **Positive:** local Ollama and the unconfigured/disabled path retain their
  current behavior.
- **Negative / trade-off:** the new environment variable is secret material
  operators must manage, and hosted model selection remains constrained by the
  existing 768-dimensional database schema.
- **Risk:** low. The configuration is additive and optional; request behavior
  changes only when a key is explicitly supplied.

## Exit criteria and evidence

Evidence: [EV-0068](../evidence.d/0068-embedding-adapter-optional-api-key.md)

| Exit criterion | Evidence |
|---|---|
| `EmbeddingConfig` reads an optional, non-blank API key | `embedding-config-carries-optional-api-key` |
| `embed` sends bearer authentication only when configured | `embedding-auth-header-is-conditional` |
| Keyless and unconfigured behavior remain intact | `embedding-keyless-posture-is-preserved` |
| Compose and local configuration guidance expose the option safely | `embedding-api-key-is-configurable` |
| Focused adapter tests pass | `embedding-auth-tests-pass` |