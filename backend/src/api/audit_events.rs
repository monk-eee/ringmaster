use crate::audit;
#[cfg(test)]
use crate::extraction;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value as JsonValue};
use sqlx::PgPool;

#[derive(Debug, Deserialize)]
pub(super) struct AuditEventsQuery {
    limit: Option<i64>,
}

/// A flat, reverse-chronological feed of recent audit rows (ADR-0049):
/// read-only, no correlation to any specific Obligation or candidate.
pub(super) async fn list_audit_events(
    State(pool): State<PgPool>,
    Query(params): Query<AuditEventsQuery>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let rows = audit::recent(&pool, params.limit).await.map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;
    Ok(Json(json!(rows
        .into_iter()
        .map(|row| json!({
            "id": row.id,
            "actor": row.actor,
            "action": row.action,
            "previous_state": row.previous_state,
            "new_state": row.new_state,
            "source": row.source,
            "policy_outcome": row.policy_outcome,
            "recorded_at": row.recorded_at.to_rfc3339(),
        }))
        .collect::<Vec<_>>())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{app, test_pool};
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    /// ADR-0049: the audit feed surfaces a real, just-recorded correction --
    /// found by a unique marker, never an aggregate count.
    #[tokio::test]
    async fn audit_events_route_surfaces_a_real_correction() {
        let pool = test_pool().await;
        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(
            &pool,
            candidate_id,
            "risk",
            "stated risk",
            uuid::Uuid::new_v4(),
            Some(0.7),
            None,
        )
        .await
        .expect("extract candidate");
        extraction::rebuild_candidate_projection(&pool)
            .await
            .expect("rebuild candidate projection");

        let marker = format!("audit-route-marker-{}", uuid::Uuid::new_v4());
        let correct_response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/correct"))
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"statement": marker}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(correct_response.status(), axum::http::StatusCode::OK);

        let audit_response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/audit-events?limit=200")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(audit_response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(audit_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let rows = parsed.as_array().unwrap();
        assert!(
            rows.iter()
                .any(|row| row["new_state"]["statement"] == marker),
            "the correction's audit row must be present in the feed"
        );
    }
}
