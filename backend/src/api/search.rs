use crate::embedding_adapter::EmbeddingConfig;
use crate::graph;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value as JsonValue};
use sqlx::PgPool;

#[derive(Debug, Deserialize)]
pub(super) struct SearchParams {
    q: Option<String>,
    limit: Option<i64>,
}

/// Semantic search over embedded source fragments (ADR-0019): read-only,
/// never automatic. Mirrors the extraction route's error posture exactly --
/// a typed `503` when no embedding model is configured, never a panic.
pub(super) async fn search(
    State(pool): State<PgPool>,
    Query(params): Query<SearchParams>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let query = params.q.as_deref().unwrap_or("").trim();
    if query.is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "q is required".to_string(),
        ));
    }
    let limit = params.limit.unwrap_or(10);

    let Some(config) = EmbeddingConfig::from_env() else {
        return Err((
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "RINGMASTER_EMBEDDING_URL is not set; search is disabled".to_string(),
        ));
    };

    let results = graph::search_source_fragments(&pool, &config, query, limit)
        .await
        .map_err(|error| match error {
            graph::EmbeddingError::Adapter(_) => (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                error.to_string(),
            ),
            graph::EmbeddingError::Database(_) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                error.to_string(),
            ),
        })?;

    Ok(Json(json!(results)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{app, test_pool};
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn search_route_returns_400_when_query_is_missing() {
        let pool = test_pool().await;
        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri("/api/search")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn search_route_returns_400_when_query_is_blank() {
        let pool = test_pool().await;
        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri("/api/search?q=%20")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    /// Exercises a real, live round-trip when RINGMASTER_EMBEDDING_URL is
    /// configured (ADR-0019); otherwise reports and passes trivially, same
    /// posture as the extraction route's own live-model test. Embeds a
    /// unique marker word alongside the semantic content and searches for
    /// both -- this repository's shared test database accumulates one
    /// embedding per run of this exact test (worse since ADR-0062 auto-
    /// embeds on ingestion too), so a fixed query would eventually rank the
    /// newest row outside the default limit once enough near-duplicates
    /// exist; a marker only this run's fragment contains cannot lose that
    /// race no matter how many prior runs already accumulated.
    #[tokio::test]
    async fn search_route_ranks_results_when_a_model_is_configured() {
        if EmbeddingConfig::from_env().is_none() {
            eprintln!(
                "skipped: RINGMASTER_EMBEDDING_URL is not set, no live embedding model configured"
            );
            return;
        }
        let pool = test_pool().await;
        let marker = format!("marker{}", uuid::Uuid::new_v4().simple());
        let fragment_id = graph::create_source_fragment(
            &pool,
            uuid::Uuid::new_v4(),
            &format!("Roopa: please send me a transition plan by Friday. Reference {marker}."),
            "search-api-test-hash",
        )
        .await
        .expect("create source fragment");
        graph::embed_source_fragment(&pool, &EmbeddingConfig::from_env().unwrap(), fragment_id)
            .await
            .expect("embed source fragment");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/search?q=transition+plan+{marker}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert!(parsed.is_array(), "response body must be a JSON array");
        assert!(
            parsed
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row.get("source_fragment_id").and_then(|v| v.as_str())
                    == Some(fragment_id.to_string().as_str())),
            "the just-embedded fragment must appear among the ranked results"
        );
    }
}
