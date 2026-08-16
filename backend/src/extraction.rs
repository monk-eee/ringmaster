use crate::model_adapter::{self, ModelAdapterError, ModelConfig};
use serde_json::{json, Value as Json};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

pub const ALLOWED_CANDIDATE_TYPES: [&str; 6] =
    ["commitment", "request", "risk", "follow_up", "decision", "expectation"];

/// Rejected before any row is written, so an invalid candidate never
/// reaches the append-only event log (ADR-0011).
#[derive(Debug)]
pub enum ExtractionError {
    InvalidPayload(String),
    Database(sqlx::Error),
}

impl std::fmt::Display for ExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPayload(reason) => write!(f, "invalid candidate payload: {reason}"),
            Self::Database(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ExtractionError {}

impl From<sqlx::Error> for ExtractionError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

fn validate_candidate_payload(candidate_type: &str, confidence: Option<f32>) -> Result<(), ExtractionError> {
    if !ALLOWED_CANDIDATE_TYPES.contains(&candidate_type) {
        return Err(ExtractionError::InvalidPayload(format!(
            "candidate_type must be one of {ALLOWED_CANDIDATE_TYPES:?}, got {candidate_type:?}"
        )));
    }
    if let Some(value) = confidence {
        if !(0.0..=1.0).contains(&value) {
            return Err(ExtractionError::InvalidPayload(format!(
                "confidence must be within [0.0, 1.0], got {value}"
            )));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, FromRow)]
pub struct CandidateEvent {
    pub id: Uuid,
    pub candidate_id: Uuid,
    pub event_type: String,
    pub payload: Json,
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct CandidateProjection {
    pub candidate_id: Uuid,
    pub candidate_type: String,
    pub statement: String,
    pub validation_state: String,
    pub confidence: Option<f32>,
}

/// Appends one `extracted` candidate event (ADR-0011, docs/PRODUCT-SPEC.md
/// SS6.3). `candidate_type` and `confidence` are validated before the row
/// is written.
pub async fn extract_candidate(
    pool: &PgPool,
    candidate_id: Uuid,
    candidate_type: &str,
    statement: &str,
    source_fragment_id: Uuid,
    confidence: Option<f32>,
    extraction_model: Option<&str>,
) -> Result<Uuid, ExtractionError> {
    validate_candidate_payload(candidate_type, confidence)?;

    let payload = json!({
        "candidate_type": candidate_type,
        "statement": statement,
        "source_fragment_id": source_fragment_id,
        "confidence": confidence,
        "extraction_model": extraction_model,
        "requires_validation": true,
    });

    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO candidate_events (candidate_id, event_type, payload) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(candidate_id)
    .bind("extracted")
    .bind(&payload)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

/// Appends a SS6.4 lifecycle transition (accepted/corrected/rejected/
/// superseded/observed_complete/closed) for an existing candidate. `payload`
/// carries whatever changed; this is how corrections preserve previous
/// values and provenance instead of overwriting them. Generic over the SQL
/// executor (ADR-0038) so a caller can append this event in the same
/// transaction as an audit row -- `&PgPool` still works unchanged.
pub async fn transition_candidate<'e, E>(
    executor: E,
    candidate_id: Uuid,
    event_type: &str,
    payload: Json,
) -> Result<Uuid, ExtractionError>
where
    E: sqlx::PgExecutor<'e>,
{
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO candidate_events (candidate_id, event_type, payload) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(candidate_id)
    .bind(event_type)
    .bind(&payload)
    .fetch_one(executor)
    .await?;
    Ok(id)
}

struct CandidateState {
    candidate_type: String,
    statement: String,
    validation_state: String,
    confidence: Option<f32>,
    source_fragment_id: Option<Uuid>,
    promoted_obligation_id: Option<Uuid>,
}

/// Provisional extraction prompt (ADR-0011): not a final, product-quality
/// prompt design. It exists to make a real, live round-trip against a
/// configured model testable today.
const EXTRACTION_PROMPT_PREAMBLE: &str = "You extract management candidates \
from one meeting transcript fragment. Decide whether the fragment contains a \
commitment, request, risk, follow_up, decision, or expectation. Respond with \
ONLY one JSON object and no other text, matching exactly this shape: \
{\"candidate_type\": \"commitment|request|risk|follow_up|decision|expectation\", \
\"statement\": \"...\", \"confidence\": 0.0-1.0}. If the fragment contains \
nothing worth extracting, respond with exactly {\"candidate_type\": null}.";

#[derive(Debug)]
pub enum ModelExtractionError {
    Model(ModelAdapterError),
    Parse(String),
    Persist(ExtractionError),
}

impl std::fmt::Display for ModelExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Model(error) => write!(f, "{error}"),
            Self::Parse(reason) => write!(f, "could not parse model response: {reason}"),
            Self::Persist(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ModelExtractionError {}

fn extract_json_object(raw: &str) -> Option<&str> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    (end >= start).then(|| &raw[start..=end])
}

/// Calls the configured model to extract at most one candidate from a
/// fragment's text, then persists it as an `extracted` event (ADR-0011).
/// Returns `Ok(None)` when the model reports nothing worth extracting.
/// Never panics on a malformed or unreachable model response; returns a
/// typed error instead.
pub async fn extract_candidate_via_model(
    pool: &PgPool,
    config: &ModelConfig,
    source_fragment_id: Uuid,
    fragment_text: &str,
) -> Result<Option<Uuid>, ModelExtractionError> {
    let prompt = format!("{EXTRACTION_PROMPT_PREAMBLE}\n\nFragment: {fragment_text}");
    let raw_response = model_adapter::complete(config, &prompt).await.map_err(ModelExtractionError::Model)?;

    let json_text = extract_json_object(&raw_response)
        .ok_or_else(|| ModelExtractionError::Parse(format!("no JSON object found in: {raw_response:?}")))?;
    let parsed: Json = serde_json::from_str(json_text)
        .map_err(|error| ModelExtractionError::Parse(format!("invalid JSON from model: {error}")))?;

    let Some(candidate_type) = parsed.get("candidate_type").and_then(|value| value.as_str()) else {
        return Ok(None);
    };
    let statement = parsed.get("statement").and_then(|value| value.as_str()).unwrap_or("").to_string();
    let confidence = parsed.get("confidence").and_then(|value| value.as_f64()).map(|value| value as f32);

    let candidate_id = Uuid::new_v4();
    extract_candidate(pool, candidate_id, candidate_type, &statement, source_fragment_id, confidence, Some(&config.model))
        .await
        .map_err(ModelExtractionError::Persist)?;
    Ok(Some(candidate_id))
}

/// Derives current candidate state from the full event log alone
/// (ADR-0011). Always truncates and rewrites the projection; never patches
/// it in place, so a rebuild can never disagree with the event log it was
/// just built from.
pub async fn rebuild_candidate_projection(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let events: Vec<CandidateEvent> = sqlx::query_as(
        "SELECT id, candidate_id, event_type, payload FROM candidate_events \
         ORDER BY candidate_id, recorded_at, id",
    )
    .fetch_all(pool)
    .await?;

    let mut states: std::collections::HashMap<Uuid, CandidateState> = std::collections::HashMap::new();
    for event in &events {
        match event.event_type.as_str() {
            "extracted" => {
                let candidate_type = event.payload.get("candidate_type").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let statement = event.payload.get("statement").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let confidence = event.payload.get("confidence").and_then(|v| v.as_f64()).map(|v| v as f32);
                let source_fragment_id = event
                    .payload
                    .get("source_fragment_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok());
                states.insert(
                    event.candidate_id,
                    CandidateState {
                        candidate_type,
                        statement,
                        validation_state: "candidate".to_string(),
                        confidence,
                        source_fragment_id,
                        promoted_obligation_id: None,
                    },
                );
            }
            transition @ ("accepted" | "corrected" | "rejected" | "superseded" | "observed_complete" | "closed" | "promoted") => {
                if let Some(state) = states.get_mut(&event.candidate_id) {
                    state.validation_state = transition.to_string();
                    if let Some(statement) = event.payload.get("statement").and_then(|v| v.as_str()) {
                        state.statement = statement.to_string();
                    }
                    if let Some(candidate_type) = event.payload.get("candidate_type").and_then(|v| v.as_str()) {
                        state.candidate_type = candidate_type.to_string();
                    }
                    if transition == "promoted" {
                        if let Some(obligation_id) =
                            event.payload.get("obligation_id").and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok())
                        {
                            state.promoted_obligation_id = Some(obligation_id);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let mut tx = pool.begin().await?;
    sqlx::query("TRUNCATE candidate_projection").execute(&mut *tx).await?;
    let mut written = 0u64;
    for (candidate_id, state) in &states {
        sqlx::query(
            "INSERT INTO candidate_projection (candidate_id, candidate_type, statement, validation_state, confidence, source_fragment_id, promoted_obligation_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(candidate_id)
        .bind(&state.candidate_type)
        .bind(&state.statement)
        .bind(&state.validation_state)
        .bind(state.confidence)
        .bind(state.source_fragment_id)
        .bind(state.promoted_obligation_id)
        .execute(&mut *tx)
        .await?;
        written += 1;
    }
    tx.commit().await?;
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    async fn test_pool() -> PgPool {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to run extraction tests");
        PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await
            .expect("connect to test database")
    }

    #[tokio::test]
    async fn deterministic_validation_rejects_an_unrecognized_candidate_type() {
        let pool = test_pool().await;
        let result = extract_candidate(&pool, Uuid::new_v4(), "not_a_real_type", "x", Uuid::new_v4(), None, None).await;
        assert!(matches!(result, Err(ExtractionError::InvalidPayload(_))));
    }

    /// Exercises a real, live round-trip when RINGMASTER_LLM_URL is
    /// configured (ADR-0011); otherwise reports and passes trivially rather
    /// than failing an environment with no model available.
    #[tokio::test]
    async fn extract_candidate_via_model_round_trips_against_a_live_endpoint_when_configured() {
        let Some(config) = ModelConfig::from_env() else {
            eprintln!("skipped: RINGMASTER_LLM_URL is not set, no live model configured");
            return;
        };
        let pool = test_pool().await;
        let source_fragment_id = Uuid::new_v4();

        let result = extract_candidate_via_model(
            &pool,
            &config,
            source_fragment_id,
            "Roopa: please send me a transition plan by Friday.",
        )
        .await;

        assert!(result.is_ok(), "live extraction call failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn deterministic_validation_rejects_out_of_range_confidence() {
        let pool = test_pool().await;
        let result = extract_candidate(&pool, Uuid::new_v4(), "risk", "x", Uuid::new_v4(), Some(1.5), None).await;
        assert!(matches!(result, Err(ExtractionError::InvalidPayload(_))));
    }

    #[tokio::test]
    async fn candidate_events_cannot_be_mutated_or_deleted() {
        let pool = test_pool().await;
        let candidate_id = Uuid::new_v4();
        let event_id = extract_candidate(&pool, candidate_id, "risk", "stated risk", Uuid::new_v4(), Some(0.7), Some("test-model"))
            .await
            .expect("extract candidate");

        let update_result = sqlx::query("UPDATE candidate_events SET event_type = 'tampered' WHERE id = $1")
            .bind(event_id)
            .execute(&pool)
            .await;
        assert!(update_result.is_err(), "UPDATE must be rejected by the append-only trigger");

        let delete_result = sqlx::query("DELETE FROM candidate_events WHERE id = $1")
            .bind(event_id)
            .execute(&pool)
            .await;
        assert!(delete_result.is_err(), "DELETE must be rejected by the append-only trigger");
    }

    #[tokio::test]
    async fn correction_preserves_provenance_and_projection_reflects_it_after_rebuild() {
        let pool = test_pool().await;
        let candidate_id = Uuid::new_v4();
        extract_candidate(&pool, candidate_id, "request", "send the commitments", Uuid::new_v4(), Some(0.6), Some("test-model"))
            .await
            .expect("extract candidate");
        rebuild_candidate_projection(&pool).await.expect("rebuild after extraction");

        let after_extraction: CandidateProjection =
            sqlx::query_as("SELECT candidate_id, candidate_type, statement, validation_state, confidence FROM candidate_projection WHERE candidate_id = $1")
                .bind(candidate_id)
                .fetch_one(&pool)
                .await
                .expect("projection row after extraction");
        assert_eq!(after_extraction.validation_state, "candidate");
        assert_eq!(after_extraction.candidate_type, "request");

        transition_candidate(&pool, candidate_id, "corrected", json!({"candidate_type": "commitment"}))
            .await
            .expect("append corrected event");
        rebuild_candidate_projection(&pool).await.expect("rebuild after correction");

        let after_correction: CandidateProjection =
            sqlx::query_as("SELECT candidate_id, candidate_type, statement, validation_state, confidence FROM candidate_projection WHERE candidate_id = $1")
                .bind(candidate_id)
                .fetch_one(&pool)
                .await
                .expect("projection row after correction");
        assert_eq!(after_correction.validation_state, "corrected");
        assert_eq!(
            after_correction.candidate_type, "commitment",
            "the correction must be reflected only via a rebuild from the full event log"
        );

        let events: Vec<CandidateEvent> =
            sqlx::query_as("SELECT id, candidate_id, event_type, payload FROM candidate_events WHERE candidate_id = $1 ORDER BY recorded_at")
                .bind(candidate_id)
                .fetch_all(&pool)
                .await
                .expect("query candidate_events");
        assert_eq!(events.len(), 2, "the original extracted event must still exist alongside the correction");
        assert_eq!(events[0].event_type, "extracted");
        assert_eq!(events[0].payload.get("candidate_type").and_then(|v| v.as_str()), Some("request"));
    }

    #[tokio::test]
    async fn rebuild_populates_source_fragment_id_from_the_extracted_event() {
        let pool = test_pool().await;
        let candidate_id = Uuid::new_v4();
        let source_fragment_id = Uuid::new_v4();
        extract_candidate(&pool, candidate_id, "risk", "stated risk", source_fragment_id, Some(0.7), Some("test-model"))
            .await
            .expect("extract candidate");
        rebuild_candidate_projection(&pool).await.expect("rebuild after extraction");

        let (stored,): (Option<Uuid>,) =
            sqlx::query_as("SELECT source_fragment_id FROM candidate_projection WHERE candidate_id = $1")
                .bind(candidate_id)
                .fetch_one(&pool)
                .await
                .expect("projection row after extraction");
        assert_eq!(stored, Some(source_fragment_id));
    }
}
