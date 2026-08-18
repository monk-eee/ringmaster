use super::{clamp_list_params, ListQuery};
use crate::audit;
use crate::extraction::{self, CandidateProjection, ModelExtractionError};
use crate::graph;
use crate::model_adapter::ModelConfig;
use crate::obligation;
use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value as JsonValue};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

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
    promoted_obligation_id: Option<Uuid>,
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
        "promoted_obligation_id": row.promoted_obligation_id,
        "source_text": row.source_text,
        "speaker": row.speaker,
    })
}

/// Reads the current `candidate_projection` rows, joined read-only against
/// the immutable `source_fragments` table for evidence (ADR-0013/ADR-0015).
/// Never writes. `limit`/`offset` of `None` fetch every row unchanged
/// (ADR-0059); a given `limit` is clamped to `[1, MAX_LIST_LIMIT]` rather
/// than rejected, matching ADR-0049's audit-limit precedent.
pub(super) async fn list_candidates(
    State(pool): State<PgPool>,
    Query(params): Query<ListQuery>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let (limit, offset) = clamp_list_params(&params);
    let rows: Vec<CandidateWithSource> = sqlx::query_as(
        "SELECT cp.candidate_id, cp.candidate_type, cp.statement, cp.validation_state, cp.confidence, \
                cp.source_fragment_id, cp.promoted_obligation_id, sf.text AS source_text, sf.speaker \
         FROM candidate_projection cp \
         LEFT JOIN source_fragments sf ON sf.id = cp.source_fragment_id \
         ORDER BY cp.candidate_id \
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&pool)
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    Ok(Json(json!(rows
        .iter()
        .map(candidate_with_source_json)
        .collect::<Vec<_>>())))
}

/// Shared body for accept/reject (ADR-0024): both are a plain state
/// transition with no field changes, differing only in the event type.
/// `409` when the candidate isn't currently in the `candidate` state stops
/// a stale UI from double-transitioning something already resolved.
async fn transition_candidate_route(
    pool: &PgPool,
    id: Uuid,
    event_type: &'static str,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let current: CandidateWithSource = sqlx::query_as(
        "SELECT cp.candidate_id, cp.candidate_type, cp.statement, cp.validation_state, cp.confidence, \
                cp.source_fragment_id, cp.promoted_obligation_id, sf.text AS source_text, sf.speaker \
         FROM candidate_projection cp \
         LEFT JOIN source_fragments sf ON sf.id = cp.source_fragment_id \
         WHERE cp.candidate_id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|error| match error {
        sqlx::Error::RowNotFound => (axum::http::StatusCode::NOT_FOUND, "candidate not found".to_string()),
        other => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
    })?;

    if current.validation_state != "candidate" {
        return Err((
            axum::http::StatusCode::CONFLICT,
            format!(
                "candidate is already \"{}\", not \"candidate\"",
                current.validation_state
            ),
        ));
    }

    // ADR-0038: the state-change event and its audit row commit atomically --
    // a failure between the two can never leave the action un-audited.
    let action = if event_type == "accepted" {
        "candidate_accepted"
    } else {
        "candidate_rejected"
    };
    let mut tx = pool.begin().await.map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;
    extraction::transition_candidate(&mut *tx, id, event_type, json!({}))
        .await
        .map_err(|error| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                error.to_string(),
            )
        })?;
    audit::record(
        &mut *tx,
        "local-operator",
        action,
        Some(json!({"validation_state": current.validation_state})),
        Some(json!({"validation_state": event_type})),
        "http_api",
        "allowed",
    )
    .await
    .map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;
    tx.commit().await.map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;

    extraction::rebuild_candidate_projection(pool)
        .await
        .map_err(|error| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                error.to_string(),
            )
        })?;

    let updated: CandidateWithSource = sqlx::query_as(
        "SELECT cp.candidate_id, cp.candidate_type, cp.statement, cp.validation_state, cp.confidence, \
                cp.source_fragment_id, cp.promoted_obligation_id, sf.text AS source_text, sf.speaker \
         FROM candidate_projection cp \
         LEFT JOIN source_fragments sf ON sf.id = cp.source_fragment_id \
         WHERE cp.candidate_id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    Ok(Json(candidate_with_source_json(&updated)))
}

/// Accepts a candidate still in the `candidate` state (ADR-0024). Plain
/// accept, no field changes; use a future correction control for edits.
pub(super) async fn accept_candidate(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    transition_candidate_route(&pool, id, "accepted").await
}

/// Rejects a candidate still in the `candidate` state (ADR-0024).
pub(super) async fn reject_candidate(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    transition_candidate_route(&pool, id, "rejected").await
}

#[derive(Debug, Deserialize)]
pub(super) struct CorrectCandidateRequest {
    candidate_type: Option<String>,
    statement: Option<String>,
}

/// Corrects a candidate still in the `candidate` state (ADR-0045): edits
/// `candidate_type` and/or `statement` before transitioning to `corrected`,
/// a distinct outcome from `accepted` (PRODUCT-SPEC.md ┬º6.4) that still
/// promotes exactly like `accepted` does. At least one field must actually
/// change, or the request is rejected as a meaningless correction rather
/// than a silent no-op event.
pub(super) async fn correct_candidate(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(body): Json<CorrectCandidateRequest>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let current: CandidateWithSource = sqlx::query_as(
        "SELECT cp.candidate_id, cp.candidate_type, cp.statement, cp.validation_state, cp.confidence, \
                cp.source_fragment_id, cp.promoted_obligation_id, sf.text AS source_text, sf.speaker \
         FROM candidate_projection cp \
         LEFT JOIN source_fragments sf ON sf.id = cp.source_fragment_id \
         WHERE cp.candidate_id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .map_err(|error| match error {
        sqlx::Error::RowNotFound => (axum::http::StatusCode::NOT_FOUND, "candidate not found".to_string()),
        other => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
    })?;

    if current.validation_state != "candidate" {
        return Err((
            axum::http::StatusCode::CONFLICT,
            format!(
                "candidate is already \"{}\", not \"candidate\"",
                current.validation_state
            ),
        ));
    }

    if let Some(candidate_type) = &body.candidate_type {
        if !extraction::ALLOWED_CANDIDATE_TYPES.contains(&candidate_type.as_str()) {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                format!(
                    "candidate_type must be one of {:?}, got {:?}",
                    extraction::ALLOWED_CANDIDATE_TYPES,
                    candidate_type
                ),
            ));
        }
    }

    let type_changed = body
        .candidate_type
        .as_deref()
        .is_some_and(|value| value != current.candidate_type);
    let statement_changed = body
        .statement
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty() && value != current.statement);
    if !type_changed && !statement_changed {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "a correction must actually change candidate_type or statement".to_string(),
        ));
    }

    let mut payload = serde_json::Map::new();
    if type_changed {
        payload.insert("candidate_type".to_string(), json!(body.candidate_type));
    }
    if statement_changed {
        payload.insert("statement".to_string(), json!(body.statement));
    }

    // ADR-0038: the correction event and its audit row commit atomically.
    let mut tx = pool.begin().await.map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;
    extraction::transition_candidate(
        &mut *tx,
        id,
        "corrected",
        JsonValue::Object(payload.clone()),
    )
    .await
    .map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;
    audit::record(
        &mut *tx,
        "local-operator",
        "candidate_corrected",
        Some(json!({"candidate_type": current.candidate_type, "statement": current.statement})),
        Some(JsonValue::Object(payload)),
        "http_api",
        "allowed",
    )
    .await
    .map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;
    tx.commit().await.map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;

    extraction::rebuild_candidate_projection(&pool)
        .await
        .map_err(|error| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                error.to_string(),
            )
        })?;

    let updated: CandidateWithSource = sqlx::query_as(
        "SELECT cp.candidate_id, cp.candidate_type, cp.statement, cp.validation_state, cp.confidence, \
                cp.source_fragment_id, cp.promoted_obligation_id, sf.text AS source_text, sf.speaker \
         FROM candidate_projection cp \
         LEFT JOIN source_fragments sf ON sf.id = cp.source_fragment_id \
         WHERE cp.candidate_id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    Ok(Json(candidate_with_source_json(&updated)))
}

/// Promotes an `accepted` or `corrected` candidate into a new Obligation
/// (ADR-0027, extended by ADR-0045 to also accept `corrected`). `409`
/// unless the candidate is currently in one of those two states --
/// promotion is one-way, matching accept/reject's own one-way 409
/// semantics. The new Obligation carries the candidate's
/// `source_fragment_id` forward as its own evidence link (ADR-0023); no
/// due date is implied by a candidate.
pub(super) async fn promote_candidate(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Response, (axum::http::StatusCode, String)> {
    let current: CandidateWithSource = sqlx::query_as(
        "SELECT cp.candidate_id, cp.candidate_type, cp.statement, cp.validation_state, cp.confidence, \
                cp.source_fragment_id, cp.promoted_obligation_id, sf.text AS source_text, sf.speaker \
         FROM candidate_projection cp \
         LEFT JOIN source_fragments sf ON sf.id = cp.source_fragment_id \
         WHERE cp.candidate_id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .map_err(|error| match error {
        sqlx::Error::RowNotFound => (axum::http::StatusCode::NOT_FOUND, "candidate not found".to_string()),
        other => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
    })?;

    if current.validation_state != "accepted" && current.validation_state != "corrected" {
        return Err((
            axum::http::StatusCode::CONFLICT,
            format!(
                "candidate is \"{}\", not \"accepted\" or \"corrected\"",
                current.validation_state
            ),
        ));
    }

    let obligation_id = Uuid::new_v4();
    // ADR-0058: a candidate extracted with a stated deadline seeds the new
    // Obligation's soft (advisory) due date; absent one, this is None and the
    // Obligation is dateless exactly as before.
    let due_at = extraction::candidate_extracted_due_at(&pool, id)
        .await
        .map_err(|error| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                error.to_string(),
            )
        })?;
    // ADR-0060: a candidate extracted with a stated owner resolves, by exact
    // case-insensitive name match only, against an existing Person node.
    // No match (including no owner_name at all) creates nothing extra.
    let owner_name = extraction::candidate_extracted_owner_name(&pool, id)
        .await
        .map_err(|error| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                error.to_string(),
            )
        })?;
    let owner_person_id: Option<Uuid> = match &owner_name {
        Some(name) => sqlx::query_scalar("SELECT id FROM nodes WHERE node_type = 'person' AND lower(canonical_text) = lower($1) LIMIT 1")
            .bind(name)
            .fetch_optional(&pool)
            .await
            .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?,
        None => None,
    };
    let mut created_payload = serde_json::Map::new();
    created_payload.insert("status".to_string(), json!("open"));
    created_payload.insert(
        "source_fragment_id".to_string(),
        json!(current.source_fragment_id),
    );
    if let Some(due_at) = due_at {
        created_payload.insert("soft_due_at".to_string(), json!(due_at.to_rfc3339()));
    }
    // ADR-0038: the Obligation creation, the candidate's own promoted
    // transition, and the audit row all commit atomically.
    let mut tx = pool.begin().await.map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;
    obligation::append_event(
        &mut *tx,
        obligation_id,
        obligation::ObligationEventType::Created,
        JsonValue::Object(created_payload),
    )
    .await
    .map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;

    // ADR-0060: the owns edge commits in the same transaction as the
    // Obligation it names an owner for.
    if let Some(person_id) = owner_person_id {
        graph::create_edge(&mut *tx, person_id, obligation_id, "owns", None)
            .await
            .map_err(|error| {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    error.to_string(),
                )
            })?;
    }

    extraction::transition_candidate(
        &mut *tx,
        id,
        "promoted",
        json!({"obligation_id": obligation_id}),
    )
    .await
    .map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;

    audit::record(
        &mut *tx,
        "local-operator",
        "candidate_promoted",
        Some(json!({"validation_state": current.validation_state})),
        Some(json!({"validation_state": "promoted", "obligation_id": obligation_id})),
        "http_api",
        "allowed",
    )
    .await
    .map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;
    tx.commit().await.map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;

    obligation::rebuild_projection(&pool)
        .await
        .map_err(|error| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                error.to_string(),
            )
        })?;
    extraction::rebuild_candidate_projection(&pool)
        .await
        .map_err(|error| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                error.to_string(),
            )
        })?;

    #[allow(clippy::type_complexity)]
    let (
        obligation_id,
        status,
        updated_at,
        hard_due_at,
        soft_due_at,
        source_fragment_id,
        source_text,
    ): (
        Uuid,
        String,
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<Uuid>,
        Option<String>,
    ) = sqlx::query_as(
        "SELECT op.obligation_id, op.status, op.updated_at, op.hard_due_at, op.soft_due_at, \
                op.source_fragment_id, sf.text \
         FROM obligation_projection op \
         LEFT JOIN source_fragments sf ON sf.id = op.source_fragment_id \
         WHERE op.obligation_id = $1",
    )
    .bind(obligation_id)
    .fetch_one(&pool)
    .await
    .map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(json!({
            "obligation_id": obligation_id,
            "status": status,
            "updated_at": updated_at.to_rfc3339(),
            "hard_due_at": hard_due_at.map(|value| value.to_rfc3339()),
            "soft_due_at": soft_due_at.map(|value| value.to_rfc3339()),
            "source_fragment_id": source_fragment_id,
            "source_text": source_text,
        })),
    )
        .into_response())
}

/// Triggers extraction for one named source fragment (ADR-0013): explicit
/// and synchronous, never automatic on ingestion. Translates the model
/// adapter's typed errors into HTTP statuses instead of panicking.
pub(super) async fn extract_source_fragment(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Response, (axum::http::StatusCode, String)> {
    let fragment = graph::get_source_fragment(&pool, id)
        .await
        .map_err(|error| match error {
            sqlx::Error::RowNotFound => (
                axum::http::StatusCode::NOT_FOUND,
                "source fragment not found".to_string(),
            ),
            other => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                other.to_string(),
            ),
        })?;

    let Some(config) = ModelConfig::from_env() else {
        return Err((
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "RINGMASTER_LLM_URL is not set; extraction is disabled".to_string(),
        ));
    };

    match extraction::extract_candidate_via_model(
        &pool,
        &config,
        fragment.id,
        &fragment.text,
        chrono::Utc::now(),
    )
    .await
    {
        Ok(Some(candidate_id)) => {
            extraction::rebuild_candidate_projection(&pool)
                .await
                .map_err(|error| {
                    (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        error.to_string(),
                    )
                })?;
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
        Err(ModelExtractionError::Model(error)) => Err((
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            error.to_string(),
        )),
        Err(other) => Err((
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            other.to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{app, test_pool};
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

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
        extraction::rebuild_candidate_projection(&pool)
            .await
            .expect("rebuild candidate projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/candidates")
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
            !parsed.as_array().unwrap().is_empty(),
            "must include the just-appended candidate"
        );
    }

    /// ADR-0059: `?limit=`/`?offset=` page GET /api/candidates; omitting
    /// both keeps returning every row.
    #[tokio::test]
    async fn candidates_route_applies_limit_and_offset() {
        let pool = test_pool().await;
        for _ in 0..2 {
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
        }
        extraction::rebuild_candidate_projection(&pool)
            .await
            .expect("rebuild candidate projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/candidates?limit=1")
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
        assert_eq!(
            parsed.as_array().unwrap().len(),
            1,
            "limit=1 must return exactly one row"
        );
    }

    #[tokio::test]
    async fn extract_route_returns_404_for_unknown_fragment() {
        let pool = test_pool().await;
        let response = app(pool)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/source-fragments/{}/extract",
                        uuid::Uuid::new_v4()
                    ))
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

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert!(
            parsed.get("candidate_id").is_some(),
            "response must include the created candidate_id"
        );
    }

    #[tokio::test]
    async fn candidates_route_includes_source_fragment_evidence() {
        let pool = test_pool().await;
        let ingested = crate::transcript::ingest_transcript(
            &pool,
            &crate::transcript::MeetingMetadata {
                title: "api-test meeting".to_string(),
                occurred_at: None,
                organiser: None,
                participants: vec![],
            },
            "Roopa: please send me a transition plan by Friday.",
        )
        .await
        .expect("ingest transcript");
        let fragment_id = ingested.fragment_ids[0];

        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(
            &pool,
            candidate_id,
            "request",
            "send a transition plan",
            fragment_id,
            Some(0.8),
            None,
        )
        .await
        .expect("extract candidate");
        extraction::rebuild_candidate_projection(&pool)
            .await
            .expect("rebuild candidate projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/candidates")
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
        let row = parsed
            .as_array()
            .unwrap()
            .iter()
            .find(|row| {
                row.get("candidate_id").and_then(|v| v.as_str()) == Some(&candidate_id.to_string())
            })
            .expect("the just-created candidate must be in the response");

        assert_eq!(
            row.get("source_fragment_id").and_then(|v| v.as_str()),
            Some(fragment_id.to_string().as_str())
        );
        assert_eq!(
            row.get("source_text").and_then(|v| v.as_str()),
            Some("please send me a transition plan by Friday.")
        );
        assert_eq!(row.get("speaker").and_then(|v| v.as_str()), Some("Roopa"));
    }

    #[tokio::test]
    async fn accept_route_transitions_a_candidate_still_in_the_candidate_state() {
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

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/accept"))
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
        assert_eq!(
            parsed.get("validation_state").and_then(|v| v.as_str()),
            Some("accepted")
        );
    }

    #[tokio::test]
    async fn reject_route_transitions_a_candidate_still_in_the_candidate_state() {
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

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/reject"))
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
        assert_eq!(
            parsed.get("validation_state").and_then(|v| v.as_str()),
            Some("rejected")
        );
    }

    /// ADR-0038: accepting writes an immutable audit row in the same
    /// transaction as the state change, with the honestly-labeled
    /// single-user placeholder actor -- never a fabricated identity.
    #[tokio::test]
    async fn accept_route_writes_an_audit_row_with_the_honest_placeholder_actor() {
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

        let (before,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM audit_events WHERE action = 'candidate_accepted'")
                .fetch_one(&pool)
                .await
                .unwrap();

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/accept"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let (after,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM audit_events WHERE action = 'candidate_accepted'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            after,
            before + 1,
            "exactly one audit row must be written for this acceptance"
        );

        let (actor,): (String,) =
            sqlx::query_as("SELECT actor FROM audit_events WHERE action = 'candidate_accepted' ORDER BY recorded_at DESC LIMIT 1")
                .fetch_one(&pool)
                .await
                .expect("an audit row must exist for this acceptance");
        assert_eq!(
            actor, "local-operator",
            "actor must be the honest single-user placeholder, not a fabricated identity"
        );
    }

    /// ADR-0038: rejecting writes an immutable audit row in the same
    /// transaction as the state change.
    #[tokio::test]
    async fn reject_route_writes_an_audit_row() {
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

        let (before,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM audit_events WHERE action = 'candidate_rejected'")
                .fetch_one(&pool)
                .await
                .unwrap();

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/reject"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let (after,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM audit_events WHERE action = 'candidate_rejected'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            after,
            before + 1,
            "exactly one audit row must be written for this rejection"
        );
    }

    /// ADR-0045: correcting the statement alone transitions to `corrected`
    /// and applies exactly the changed field, leaving candidate_type as-is.
    #[tokio::test]
    async fn correct_route_changes_statement_and_transitions_to_corrected() {
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

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/correct"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"statement": "the actual, corrected risk statement"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert_eq!(
            parsed.get("validation_state").and_then(|v| v.as_str()),
            Some("corrected")
        );
        assert_eq!(
            parsed.get("statement").and_then(|v| v.as_str()),
            Some("the actual, corrected risk statement")
        );
        assert_eq!(
            parsed.get("candidate_type").and_then(|v| v.as_str()),
            Some("risk"),
            "an unchanged field must not be altered"
        );
    }

    /// ADR-0045: correcting candidate_type alone leaves statement as-is.
    #[tokio::test]
    async fn correct_route_changes_candidate_type_and_transitions_to_corrected() {
        let pool = test_pool().await;
        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(
            &pool,
            candidate_id,
            "risk",
            "actually a commitment",
            uuid::Uuid::new_v4(),
            Some(0.7),
            None,
        )
        .await
        .expect("extract candidate");
        extraction::rebuild_candidate_projection(&pool)
            .await
            .expect("rebuild candidate projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/correct"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"candidate_type": "commitment"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert_eq!(
            parsed.get("validation_state").and_then(|v| v.as_str()),
            Some("corrected")
        );
        assert_eq!(
            parsed.get("candidate_type").and_then(|v| v.as_str()),
            Some("commitment")
        );
        assert_eq!(
            parsed.get("statement").and_then(|v| v.as_str()),
            Some("actually a commitment"),
            "an unchanged field must not be altered"
        );
    }

    #[tokio::test]
    async fn correct_route_rejects_a_no_op_change() {
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

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/correct"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"candidate_type": "risk", "statement": "stated risk"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn correct_route_rejects_an_invalid_candidate_type() {
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

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/correct"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"candidate_type": "not-a-real-type"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn correct_route_returns_409_for_an_already_transitioned_candidate() {
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
        extraction::transition_candidate(&pool, candidate_id, "accepted", json!({}))
            .await
            .expect("append accepted event");
        extraction::rebuild_candidate_projection(&pool)
            .await
            .expect("rebuild candidate projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/correct"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"statement": "too late to correct"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn correct_route_returns_404_for_an_unknown_candidate() {
        let pool = test_pool().await;
        let unknown_id = uuid::Uuid::new_v4();
        let response = app(pool)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{unknown_id}/correct"))
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"statement": "irrelevant"}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    /// ADR-0038: correcting writes an immutable audit row in the same
    /// transaction as the state change.
    #[tokio::test]
    async fn correct_route_writes_an_audit_row() {
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

        let (before,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM audit_events WHERE action = 'candidate_corrected'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/correct"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"statement": "corrected via audited route"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let (after,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM audit_events WHERE action = 'candidate_corrected'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            after,
            before + 1,
            "exactly one audit row must be written for this correction"
        );
    }

    /// ADR-0045: promotion accepts a corrected candidate exactly like an
    /// accepted one -- both mean a human has validated it.
    #[tokio::test]
    async fn promote_route_accepts_a_corrected_candidate() {
        let pool = test_pool().await;
        let fragment_id = graph::create_source_fragment(
            &pool,
            uuid::Uuid::new_v4(),
            "We will migrate the pipeline by Q3.",
            "correct-then-promote-hash",
        )
        .await
        .expect("create source fragment");
        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(
            &pool,
            candidate_id,
            "risk",
            "migrate the pipeline",
            fragment_id,
            Some(0.9),
            None,
        )
        .await
        .expect("extract candidate");
        extraction::transition_candidate(
            &pool,
            candidate_id,
            "corrected",
            json!({"candidate_type": "commitment"}),
        )
        .await
        .expect("append corrected event");
        extraction::rebuild_candidate_projection(&pool)
            .await
            .expect("rebuild candidate projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/promote"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::CREATED);
    }

    #[tokio::test]
    async fn accept_route_returns_409_for_an_already_transitioned_candidate() {
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
        extraction::transition_candidate(&pool, candidate_id, "accepted", json!({}))
            .await
            .expect("append accepted event");
        extraction::rebuild_candidate_projection(&pool)
            .await
            .expect("rebuild candidate projection");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/accept"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn accept_route_returns_404_for_an_unknown_candidate() {
        let pool = test_pool().await;
        let response = app(pool)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{}/accept", uuid::Uuid::new_v4()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    /// ADR-0027: promoting an accepted candidate creates a new, open
    /// Obligation that carries the candidate's source_fragment_id forward,
    /// and the candidate itself becomes "promoted" with the new id linked.
    #[tokio::test]
    async fn promote_route_creates_an_obligation_from_an_accepted_candidate() {
        let pool = test_pool().await;
        let fragment_id = graph::create_source_fragment(
            &pool,
            uuid::Uuid::new_v4(),
            "We will migrate the pipeline by Q3.",
            "promote-test-hash",
        )
        .await
        .expect("create source fragment");
        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(
            &pool,
            candidate_id,
            "commitment",
            "migrate the pipeline",
            fragment_id,
            Some(0.9),
            None,
        )
        .await
        .expect("extract candidate");
        extraction::transition_candidate(&pool, candidate_id, "accepted", json!({}))
            .await
            .expect("append accepted event");
        extraction::rebuild_candidate_projection(&pool)
            .await
            .expect("rebuild candidate projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/promote"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert_eq!(created.get("status").and_then(|v| v.as_str()), Some("open"));
        assert_eq!(
            created.get("source_fragment_id").and_then(|v| v.as_str()),
            Some(fragment_id.to_string().as_str())
        );
        let obligation_id = created
            .get("obligation_id")
            .and_then(|v| v.as_str())
            .expect("obligation_id present")
            .to_string();

        let candidates_response = app(pool)
            .oneshot(
                Request::builder()
                    .uri("/api/candidates")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let candidates_body = axum::body::to_bytes(candidates_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let candidates: JsonValue =
            serde_json::from_slice(&candidates_body).expect("valid json body");
        let candidate_row = candidates
            .as_array()
            .unwrap()
            .iter()
            .find(|row| {
                row.get("candidate_id").and_then(|v| v.as_str()) == Some(&candidate_id.to_string())
            })
            .expect("the promoted candidate must still be present");
        assert_eq!(
            candidate_row
                .get("validation_state")
                .and_then(|v| v.as_str()),
            Some("promoted")
        );
        assert_eq!(
            candidate_row
                .get("promoted_obligation_id")
                .and_then(|v| v.as_str()),
            Some(obligation_id.as_str())
        );
    }

    /// ADR-0058: a candidate extracted with a due date carries that date into
    /// the promoted Obligation as its soft (advisory) due date, so Today can
    /// rank it by real urgency instead of "No due date recorded".
    #[tokio::test]
    async fn promote_carries_extracted_due_date_into_soft_due_at() {
        let pool = test_pool().await;
        let fragment_id = graph::create_source_fragment(
            &pool,
            uuid::Uuid::new_v4(),
            "Send the transition plan by Friday.",
            "due-date-carry-hash",
        )
        .await
        .expect("create source fragment");
        let candidate_id = uuid::Uuid::new_v4();
        let due = chrono::DateTime::parse_from_rfc3339("2026-08-21T17:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        extraction::extract_candidate_with_due_at(
            &pool,
            candidate_id,
            "request",
            "send the transition plan",
            fragment_id,
            Some(0.8),
            None,
            Some(due),
            None,
        )
        .await
        .expect("extract candidate with a due date");
        extraction::transition_candidate(&pool, candidate_id, "accepted", json!({}))
            .await
            .expect("append accepted event");
        extraction::rebuild_candidate_projection(&pool)
            .await
            .expect("rebuild candidate projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/promote"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let got = created
            .get("soft_due_at")
            .and_then(|v| v.as_str())
            .expect("promoted obligation must carry the extracted due date as soft_due_at");
        let got = chrono::DateTime::parse_from_rfc3339(got)
            .unwrap()
            .with_timezone(&chrono::Utc);
        assert_eq!(
            got, due,
            "the soft due date must equal the candidate's extracted due_at"
        );
        assert!(
            created
                .get("hard_due_at")
                .map(|v| v.is_null())
                .unwrap_or(true),
            "a model-inferred date must not become a hard due date"
        );
    }

    /// ADR-0060: a candidate extracted with an owner_name that exactly
    /// (case-insensitively) matches an existing Person node's
    /// canonical_text creates an owns edge in the same transaction as
    /// promotion.
    #[tokio::test]
    async fn promotion_creates_owns_edge_on_exact_owner_match() {
        let pool = test_pool().await;
        // A unique-per-run name: this database is long-lived across many test
        // runs, so a fixed literal name would eventually collide with an
        // older run's own person node and the unordered `LIMIT 1` lookup
        // could resolve to that one instead of this run's.
        let unique_name = format!("Owner Match Test Person {}", uuid::Uuid::new_v4());
        let person_id = graph::create_node(&pool, "person", &unique_name, json!({}))
            .await
            .expect("create person");
        let fragment_id = graph::create_source_fragment(
            &pool,
            uuid::Uuid::new_v4(),
            "a stated commitment",
            "owner-match-hash",
        )
        .await
        .expect("create source fragment");
        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate_with_due_at(
            &pool,
            candidate_id,
            "commitment",
            "send the plan",
            fragment_id,
            Some(0.8),
            None,
            None,
            Some(&unique_name.to_lowercase()),
        )
        .await
        .expect("extract candidate with a stated owner");
        extraction::transition_candidate(&pool, candidate_id, "accepted", json!({}))
            .await
            .expect("append accepted event");
        extraction::rebuild_candidate_projection(&pool)
            .await
            .expect("rebuild candidate projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/promote"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let obligation_id: uuid::Uuid = created
            .get("obligation_id")
            .and_then(|v| v.as_str())
            .unwrap()
            .parse()
            .unwrap();

        let detail_response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/obligations/{obligation_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let detail_body = axum::body::to_bytes(detail_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: JsonValue = serde_json::from_slice(&detail_body).expect("valid json body");
        let linked = detail
            .get("linked_nodes")
            .and_then(|v| v.as_array())
            .expect("linked_nodes present");
        assert!(
            linked.iter().any(|node| node["edge_type"] == "owns" && node["node_id"] == person_id.to_string()),
            "an owns edge from the exactly-matched person must exist: {linked:?}"
        );
        // ADR-0061: health is attached alongside risk_signals on this route too.
        assert_eq!(detail["health"], "Healthy");
    }

    /// ADR-0060: no owner_name, or one matching no existing Person, promotes
    /// exactly as before -- no edge, no fabricated Person node.
    #[tokio::test]
    async fn promotion_creates_no_owns_edge_without_an_exact_match() {
        let pool = test_pool().await;
        let fragment_id = graph::create_source_fragment(
            &pool,
            uuid::Uuid::new_v4(),
            "Someone unnamed will send the plan.",
            "owner-no-match-hash",
        )
        .await
        .expect("create source fragment");
        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate_with_due_at(
            &pool,
            candidate_id,
            "commitment",
            "send the plan",
            fragment_id,
            Some(0.8),
            None,
            None,
            Some("Nobody Registered"),
        )
        .await
        .expect("extract candidate with an unresolvable owner");
        extraction::transition_candidate(&pool, candidate_id, "accepted", json!({}))
            .await
            .expect("append accepted event");
        extraction::rebuild_candidate_projection(&pool)
            .await
            .expect("rebuild candidate projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/promote"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let obligation_id: uuid::Uuid = created
            .get("obligation_id")
            .and_then(|v| v.as_str())
            .unwrap()
            .parse()
            .unwrap();

        let detail_response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/obligations/{obligation_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let detail_body = axum::body::to_bytes(detail_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: JsonValue = serde_json::from_slice(&detail_body).expect("valid json body");
        let linked = detail
            .get("linked_nodes")
            .and_then(|v| v.as_array())
            .expect("linked_nodes present");
        assert!(
            linked.is_empty(),
            "no owns edge (or any edge) must exist without an exact owner match: {linked:?}"
        );
    }

    #[tokio::test]
    async fn promote_route_returns_409_for_a_candidate_not_yet_accepted() {
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

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/promote"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
    }

    /// ADR-0038: promoting writes an immutable audit row in the same
    /// transaction as the Obligation creation and candidate transition.
    #[tokio::test]
    async fn promote_route_writes_an_audit_row() {
        let pool = test_pool().await;
        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(
            &pool,
            candidate_id,
            "commitment",
            "will migrate the pipeline",
            uuid::Uuid::new_v4(),
            Some(0.8),
            None,
        )
        .await
        .expect("extract candidate");
        extraction::transition_candidate(&pool, candidate_id, "accepted", json!({}))
            .await
            .expect("append accepted event");
        extraction::rebuild_candidate_projection(&pool)
            .await
            .expect("rebuild candidate projection");

        let (before,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM audit_events WHERE action = 'candidate_promoted'")
                .fetch_one(&pool)
                .await
                .unwrap();

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/promote"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::CREATED);

        let (after,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM audit_events WHERE action = 'candidate_promoted'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            after,
            before + 1,
            "exactly one audit row must be written for this promotion"
        );
    }

    #[tokio::test]
    async fn promote_route_returns_409_for_an_already_promoted_candidate() {
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
        extraction::transition_candidate(&pool, candidate_id, "accepted", json!({}))
            .await
            .expect("append accepted event");
        extraction::transition_candidate(
            &pool,
            candidate_id,
            "promoted",
            json!({"obligation_id": uuid::Uuid::new_v4()}),
        )
        .await
        .expect("append promoted event");
        extraction::rebuild_candidate_projection(&pool)
            .await
            .expect("rebuild candidate projection");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/promote"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn promote_route_returns_404_for_an_unknown_candidate() {
        let pool = test_pool().await;
        let response = app(pool)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{}/promote", uuid::Uuid::new_v4()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }
}
