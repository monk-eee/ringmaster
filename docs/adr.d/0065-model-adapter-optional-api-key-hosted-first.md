# ADR-0065: Optional API key for the model adapter — hosted models first, Ollama as an option

- **Status:** Accepted
- **Date:** 2026-08-17
- **Decider:** monk-eee
- **Approval:** Direct instruction ("let make it configurable and use normal models first and the user can choose — i really just wanted an option of ollama"), 2026-08-17
- **Amends:** [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)'s `ModelConfig`/`complete` — adds an optional API key; the unconfigured and unreachable paths are unchanged.
- **Depends on:** [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)
- **Tags:** architecture, model-adapter, configuration

## Context

The extraction model adapter ([backend/src/model_adapter.rs](../../backend/src/model_adapter.rs))
already targets an OpenAI-compatible `/chat/completions` endpoint driven
entirely by env (`RINGMASTER_LLM_URL`, `RINGMASTER_MODEL`), so it is
*almost* provider-agnostic. But it sends **no `Authorization` header**, and
`ModelConfig` carries no credential — so today only a keyless endpoint works
in practice, which in this environment means local Ollama and its reasoning
model `glm-4.7-flash`. That model is genuinely correct (this session watched
it extract a real candidate from an ADO work item), but slow: ~52s per
fragment, because it emits ~4,400 characters of hidden reasoning before the
JSON. monk-eee's own words: *"it would be nice to use ollama — let['s] make
it configurable and use normal models first and the user can choose — i
really just wanted an option of ollama tbh ... hosted models are just
quicker."*

The ask is not to remove Ollama, and not to drop the existing tests (the
unconfigured/"disabled" path and the live round-trip must both stay). It is
to make a **hosted** OpenAI-compatible model the primary, recommended choice
— which is fast — while keeping Ollama as a documented, keyless option. The
only code change that unblocks hosted providers (OpenAI, Azure OpenAI, Groq,
Together, etc., all of which require `Authorization: Bearer <key>`) is an
optional API key.

## Decision

- `ModelConfig` gains an optional `api_key: Option<String>`, read from a new
  `RINGMASTER_LLM_API_KEY` env var. Absent key → `None`, exactly today's
  behavior; a keyless endpoint (Ollama) is unaffected.
- `complete` attaches `Authorization: Bearer <key>` **only when the key is
  present**. When absent, no auth header is sent, so Ollama keeps working
  with zero configuration.
- `ModelConfig::from_env` still returns `None` when `RINGMASTER_LLM_URL` is
  unset (extraction stays cleanly disabled, [ADR-0011](0011-extraction-pipeline-candidate-schema-and-model-adapter.md)'s
  never-block guarantee unchanged). The API key is never *required*: a URL
  with no key is valid (Ollama), a URL with a key is valid (hosted).
- **Configuration guidance flips to hosted-first:** [.env.example](../../.env.example)
  leads with a hosted OpenAI-compatible example (`RINGMASTER_LLM_URL`,
  `RINGMASTER_MODEL`, `RINGMASTER_LLM_API_KEY`) as the recommended, faster
  default a user would actually pick, with the local-Ollama config kept
  directly below it as the explicitly-labeled keyless option. `compose.yaml`
  passes `RINGMASTER_LLM_API_KEY` through (no default), and keeps the Ollama
  URL/model as the zero-credential local fallback so a checkout with no `.env`
  still runs offline.

## Scope

**In scope:** the optional `api_key` on `ModelConfig`/`from_env`; the
conditional `Authorization` header in `complete`; `RINGMASTER_LLM_API_KEY`
passthrough in `compose.yaml`; hosted-first `.env.example` guidance.

**Out of scope, named honestly:**

- **The embedding adapter** ([ADR-0018](0018-generate-and-store-source-fragment-embeddings.md)'s
  `embedding_adapter.rs`) has the identical no-auth limitation; giving *it* a
  hosted key is the same three-line change but a separate concern and ADR,
  not bundled here.
- **A `max_tokens` cap** on the reasoning-model slowness. It would not fix a
  reasoning model (which reasons regardless of the answer budget); the real
  speed fix is *using a hosted non-reasoning model*, which this ADR enables.
  A separate optional cap can be added later if wanted.
- **Per-request provider selection or a model registry.** One configured
  endpoint at a time, chosen by env, matches this codebase's existing
  single-endpoint posture.
- **Removing or defaulting-away Ollama.** It stays a first-class, documented,
  keyless option and the offline zero-config fallback.

## Options considered

- **Optional API key, hosted-first docs (chosen):** the smallest change that
  unblocks every hosted OpenAI-compatible provider, keeps Ollama working
  keyless, changes no existing behavior when the key is absent, and needs no
  new dependency.
- **Require an API key always:** would break keyless Ollama and the offline
  zero-config path — rejected.
- **A provider-type enum (`ollama` | `openai` | `azure`) with per-provider
  request shaping:** more machinery than the problem needs; every target is
  already OpenAI-compatible, so one code path plus an optional bearer token
  covers them all.

## Consequences

- **Positive:** hosted models (fast, no local GPU) become a first-class,
  recommended choice; the ADO-work-item → extraction loop this session
  proved can run in seconds instead of ~52s once pointed at a hosted model.
- **Positive:** fully backward compatible — no key means today's exact
  behavior; Ollama and the unconfigured/disabled paths are untouched.
- **Negative / trade-off:** the API key is a secret in env; it must never be
  logged or committed (`.env` is already gitignored, [ADR-0016](0016-publish-repository-publicly-on-github.md)'s
  pre-publish audit). The embedding adapter still can't use a hosted key
  until its own follow-up.
- **Risk:** low. Purely additive optional config; no schema change; the
  existing unreachable-endpoint and live round-trip tests still hold.
