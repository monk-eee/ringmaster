# EV-0011: Extraction pipeline — candidate schema, deterministic validation, and an optional model adapter

Evidence for [ADR-0011](../adr.d/0011-extraction-pipeline-candidate-schema-and-model-adapter.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0011-extraction-pipeline-candidate-schema-and-model-adapter"

[[check]]
id = "candidate-events-are-immutable"
invariant = "The database rejects mutation or deletion of an existing candidate_events row."
type = "present"
pattern = 'reject_candidate_event_mutation'
paths = ["backend/migrations/0006_candidate_events.sql"]

[[check]]
id = "candidate-projection-is-derived"
invariant = "rebuild_candidate_projection always truncates and rewrites the projection from the full event log, never patching it in place."
type = "present"
pattern = 'TRUNCATE candidate_projection'
paths = ["backend/src/extraction.rs"]

[[check]]
id = "deterministic-validation-function-exists"
invariant = "candidate_type and confidence are validated before an event is appended."
type = "present"
pattern = 'fn validate_candidate_payload'
paths = ["backend/src/extraction.rs"]

[[check]]
id = "model-adapter-function-exists"
invariant = "A model adapter calls an OpenAI-compatible endpoint and returns a typed error, without panicking, when unconfigured."
type = "present"
pattern = 'RINGMASTER_LLM_URL'
paths = ["backend/src/model_adapter.rs"]

[[check]]
id = "model-driven-extraction-function-exists"
invariant = "A function calls the configured model, parses its JSON response, and persists at most one candidate from it, without panicking on a malformed or unreachable response."
type = "present"
pattern = 'fn extract_candidate_via_model'
paths = ["backend/src/extraction.rs"]
```

## Notes

All five checks are automated and verified directly against the migration
and crate files that implement them. `cargo test` cases exercise, against a
live Postgres instance: candidate event immutability; projection rebuild
from the event log after a `corrected` event; and rejection of an invalid
`candidate_type` and an out-of-range `confidence` before any row is
written. The model adapter's "no `RINGMASTER_LLM_URL` configured" path is
tested directly.

A live round-trip against a real running model has now been exercised and
verified in this environment: with `RINGMASTER_LLM_URL` pointed at a local
Ollama instance (`glm-4.7-flash:latest`, reachable from the backend
container at `host.containers.internal:11434`), the
`extract_candidate_via_model_round_trips_against_a_live_endpoint_when_configured`
test calls the real model with the fragment "Roopa: please send me a
transition plan by Friday." and persists its response. The resulting
`candidate_events` row was inspected directly: `candidate_type = request`,
`statement = "please send me a transition plan by Friday"`,
`confidence = 1.0`, `extraction_model = glm-4.7-flash:latest` — a real
model response, correctly parsed and stored, not a stub. This closes the
honest gap this ADR's Consequences section named ("no real extraction has
been exercised against a live model"); the provisional prompt itself is
still unvalidated for quality/accuracy beyond this one example, which
remains out of scope here.
