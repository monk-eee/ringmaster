use crate::embedding_adapter::EmbeddingConfig;
use crate::extraction::{self, CandidateProjection, ModelExtractionError};
use crate::graph;
use crate::model_adapter::ModelConfig;
use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value as JsonValue};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Builds the HTTP API: `/health` and the read-only `/api/obligations`
/// (ADR-0012), plus `/api/candidates` and the extraction trigger (ADR-0013),
/// plus the semantic search route (ADR-0019).
pub fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/obligations", get(list_obligations))
        .route("/api/candidates", get(list_candidates))
        .route("/api/source-fragments/:id/extract", post(extract_source_fragment))
        .route("/api/search", get(search))
        .with_state(pool)
}

async fn health() -> &'static str {
    "OK"
}

/// Reads the current `obligation_projection` rows (ADR-0005/ADR-0012). Never
/// writes; the projection remains the sole source this route reflects.
async fn list_obligations(State(pool): State<PgPool>) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let rows: Vec<(uuid::Uuid, String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT obligation_id, status, updated_at FROM obligation_projection ORDER BY updated_at DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    let body = rows
        .into_iter()
        .map(|(obligation_id, status, updated_at)| {
            json!({
                "obligation_id": obligation_id,
                "status": status,
                "updated_at": updated_at.to_rfc3339(),
            })
        })
        .collect::<Vec<_>>();

    Ok(Json(json!(body)))
}

fn candidate_json(row: &CandidateProjection) -> JsonValue {
    json!({
        "candidate_id": row.candidate_id,
        "candidate_type": row.candidate_type,
        "statement": row.statement,
        "validation_state": row.validation_state,
        "confidence": row.confidence,
    })
}

/// Read model for `GET /api/candidates` only (ADR-0015): the same five
/// fields ADR-0013 already returns, plus source-fragment evidence joined
/// in at read time. `candidate_json`/`CandidateProjection` above are
/// unchanged and still back the extract route's response as-is.
#[derive(Debug, Clone, FromRow)]
struct CandidateWithSource {
    candidate_id: Uuid,
    candidate_type: String,
    statement: String,
    validation_state: String,
    confidence: Option<f32>,
    source_fragment_id: Option<Uuid>,
    source_text: Option<String>,
    speaker: Option<String>,
}

fn candidate_with_source_json(row: &CandidateWithSource) -> JsonValue {
    json!({
        "candidate_id": row.candidate_id,
        "candidate_type": row.candidate_type,
        "statement": row.statement,
        "validation_state": row.validation_state,
        "confidence": row.confidence,
        "source_fragment_id": row.source_fragment_id,
        "source_text": row.source_text,
        "speaker": row.speaker,
    })
}

/// Reads the current `candidate_projection` rows, joined read-only against
/// the immutable `source_fragments` table for evidence (ADR-0013/ADR-0015).
/// Never writes.
async fn list_candidates(State(pool): State<PgPool>) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let rows: Vec<CandidateWithSource> = sqlx::query_as(
        "SELECT cp.candidate_id, cp.candidate_type, cp.statement, cp.validation_state, cp.confidence, \
                cp.source_fragment_id, sf.text AS source_text, sf.speaker \
         FROM candidate_projection cp \
         LEFT JOIN source_fragments sf ON sf.id = cp.source_fragment_id \
         ORDER BY cp.candidate_id",
    )
    .fetch_all(&pool)
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    Ok(Json(json!(rows.iter().map(candidate_with_source_json).collect::<Vec<_>>())))
}

/// Triggers extraction for one named source fragment (ADR-0013): explicit
/// and synchronous, never automatic on ingestion. Translates the model
/// adapter's typed errors into HTTP statuses instead of panicking.
async fn extract_source_fragment(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Response, (axum::http::StatusCode, String)> {
    let fragment = graph::get_source_fragment(&pool, id).await.map_err(|error| match error {
        sqlx::Error::RowNotFound => (axum::http::StatusCode::NOT_FOUND, "source fragment not found".to_string()),
        other => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
    })?;

    let Some(config) = ModelConfig::from_env() else {
        return Err((axum::http::StatusCode::SERVICE_UNAVAILABLE, "RINGMASTER_LLM_URL is not set; extraction is disabled".to_string()));
    };

    match extraction::extract_candidate_via_model(&pool, &config, fragment.id, &fragment.text).await {
        Ok(Some(candidate_id)) => {
            extraction::rebuild_candidate_projection(&pool)
                .await
                .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
            let row: CandidateProjection = sqlx::query_as(
                "SELECT candidate_id, candidate_type, statement, validation_state, confidence FROM candidate_projection WHERE candidate_id = $1",
            )
            .bind(candidate_id)
            .fetch_one(&pool)
            .await
            .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
            Ok((axum::http::StatusCode::CREATED, Json(candidate_json(&row))).into_response())
        }
        Ok(None) => Ok(axum::http::StatusCode::NO_CONTENT.into_response()),
        Err(ModelExtractionError::Model(error)) => Err((axum::http::StatusCode::SERVICE_UNAVAILABLE, error.to_string())),
        Err(other) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, other.to_string())),
    }
}

#[derive(Debug, Deserialize)]
struct SearchParams {
    q: Option<String>,
    limit: Option<i64>,
}

/// Semantic search over embedded source fragments (ADR-0019): read-only,
/// never automatic. Mirrors the extraction route's error posture exactly --
/// a typed `503` when no embedding model is configured, never a panic.
async fn search(
    State(pool): State<PgPool>,
    Query(params): Query<SearchParams>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let query = params.q.as_deref().unwrap_or("").trim();
    if query.is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, "q is required".to_string()));
    }
    let limit = params.limit.unwrap_or(10);

    let Some(config) = EmbeddingConfig::from_env() else {
        return Err((axum::http::StatusCode::SERVICE_UNAVAILABLE, "RINGMASTER_EMBEDDING_URL is not set; search is disabled".to_string()));
    };

    let results = graph::search_source_fragments(&pool, &config, query, limit).await.map_err(|error| match error {
        graph::EmbeddingError::Adapter(_) => (axum::http::StatusCode::SERVICE_UNAVAILABLE, error.to_string()),
        graph::EmbeddingError::Database(_) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    })?;

    Ok(Json(json!(results)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    async fn test_pool() -> PgPool {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to run api tests");
        PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await
            .expect("connect to test database")
    }

    #[tokio::test]
    async fn health_route_returns_ok() {
        let pool = test_pool().await;
        let response = app(pool)
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn obligations_route_returns_json_array() {
        let pool = test_pool().await;
        crate::obligation::append_event(
            &pool,
            uuid::Uuid::new_v4(),
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open"}),
        )
        .await
        .expect("append created event");
        crate::obligation::rebuild_projection(&pool).await.expect("rebuild projection");

        let response = app(pool.clone())
            .oneshot(Request::builder().uri("/api/obligations").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert!(parsed.is_array(), "response body must be a JSON array");
        assert!(!parsed.as_array().unwrap().is_empty(), "must include the just-appended obligation");
    }

    #[tokio::test]
    async fn candidates_route_returns_json_array() {
        let pool = test_pool().await;
        extraction::extract_candidate(
            &pool,
            uuid::Uuid::new_v4(),
            "risk",
            "stated risk",
            uuid::Uuid::new_v4(),
            Some(0.7),
            Some("test-model"),
        )
        .await
        .expect("extract candidate");
        extraction::rebuild_candidate_projection(&pool).await.expect("rebuild candidate projection");

        let response = app(pool.clone())
            .oneshot(Request::builder().uri("/api/candidates").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert!(parsed.is_array(), "response body must be a JSON array");
        assert!(!parsed.as_array().unwrap().is_empty(), "must include the just-appended candidate");
    }

    #[tokio::test]
    async fn extract_route_returns_404_for_unknown_fragment() {
        let pool = test_pool().await;
        let response = app(pool)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/source-fragments/{}/extract", uuid::Uuid::new_v4()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    /// Exercises a real, live HTTP round-trip when RINGMASTER_LLM_URL is
    /// configured (ADR-0013); otherwise reports and passes trivially, same
    /// posture as extraction::tests's own live-model test.
    #[tokio::test]
    async fn extract_route_creates_a_candidate_when_a_model_is_configured() {
        if ModelConfig::from_env().is_none() {
            eprintln!("skipped: RINGMASTER_LLM_URL is not set, no live model configured");
            return;
        }
        let pool = test_pool().await;
        let fragment_id = graph::create_source_fragment(
            &pool,
            uuid::Uuid::new_v4(),
            "Roopa: please send me a transition plan by Friday.",
            "api-test-hash",
        )
        .await
        .expect("create source fragment");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/source-fragments/{fragment_id}/extract"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert!(parsed.get("candidate_id").is_some(), "response must include the created candidate_id");
    }

    #[tokio::test]
    async fn candidates_route_includes_source_fragment_evidence() {
        let pool = test_pool().await;
        let ingested = crate::transcript::ingest_transcript(
            &pool,
            &crate::transcript::MeetingMetadata {
                title: "api-test meeting".to_string(),
                date: None,
                organiser: None,
                participants: vec![],
            },
            "Roopa: please send me a transition plan by Friday.",
        )
        .await
        .expect("ingest transcript");
        let fragment_id = ingested.fragment_ids[0];

        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(&pool, candidate_id, "request", "send a transition plan", fragment_id, Some(0.8), None)
            .await
            .expect("extract candidate");
        extraction::rebuild_candidate_projection(&pool).await.expect("rebuild candidate projection");

        let response = app(pool.clone())
            .oneshot(Request::builder().uri("/api/candidates").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let row = parsed
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row.get("candidate_id").and_then(|v| v.as_str()) == Some(&candidate_id.to_string()))
            .expect("the just-created candidate must be in the response");

        assert_eq!(row.get("source_fragment_id").and_then(|v| v.as_str()), Some(fragment_id.to_string().as_str()));
        assert_eq!(row.get("source_text").and_then(|v| v.as_str()), Some("please send me a transition plan by Friday."));
        assert_eq!(row.get("speaker").and_then(|v| v.as_str()), Some("Roopa"));
    }

    #[tokio::test]
    async fn search_route_returns_400_when_query_is_missing() {
        let pool = test_pool().await;
        let response = app(pool)
            .oneshot(Request::builder().uri("/api/search").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn search_route_returns_400_when_query_is_blank() {
        let pool = test_pool().await;
        let response = app(pool)
            .oneshot(Request::builder().uri("/api/search?q=%20").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    /// Exercises a real, live round-trip when RINGMASTER_EMBEDDING_URL is
    /// configured (ADR-0019); otherwise reports and passes trivially, same
    /// posture as the extraction route's own live-model test.
    #[tokio::test]
    async fn search_route_ranks_results_when_a_model_is_configured() {
        if EmbeddingConfig::from_env().is_none() {
            eprintln!("skipped: RINGMASTER_EMBEDDING_URL is not set, no live embedding model configured");
            return;
        }
        let pool = test_pool().await;
        let fragment_id = graph::create_source_fragment(
            &pool,
            uuid::Uuid::new_v4(),
            "Roopa: please send me a transition plan by Friday.",
            "search-api-test-hash",
        )
        .await
        .expect("create source fragment");
        graph::embed_source_fragment(&pool, &EmbeddingConfig::from_env().unwrap(), fragment_id)
            .await
            .expect("embed source fragment");

        let response = app(pool)
            .oneshot(Request::builder().uri("/api/search?q=transition+plan").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert!(parsed.is_array(), "response body must be a JSON array");
        assert!(
            parsed.as_array().unwrap().iter().any(|row| row.get("source_fragment_id").and_then(|v| v.as_str()) == Some(fragment_id.to_string().as_str())),
            "the just-embedded fragment must appear among the ranked results"
        );
    }
}
