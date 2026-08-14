use crate::embedding_adapter::EmbeddingConfig;
use crate::extraction::{self, CandidateProjection, ModelExtractionError};
use crate::graph;
use crate::model_adapter::ModelConfig;
use crate::obligation;
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
/// plus the semantic search route (ADR-0019), plus the Daily Brief route
/// (ADR-0022), plus the accept/reject candidate routes (ADR-0024), plus the
/// node/edge write API and traversal routes (ADR-0025), plus the promote
/// route (ADR-0027), plus the Time Horizon route (ADR-0029).
pub fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/obligations", get(list_obligations))
        .route("/api/daily-brief", get(daily_brief))
        .route("/api/time-horizon", get(time_horizon))
        .route("/api/candidates", get(list_candidates))
        .route("/api/candidates/:id/accept", post(accept_candidate))
        .route("/api/candidates/:id/reject", post(reject_candidate))
        .route("/api/candidates/:id/promote", post(promote_candidate))
        .route("/api/source-fragments/:id/extract", post(extract_source_fragment))
        .route("/api/search", get(search))
        .route("/api/nodes", get(list_nodes_route).post(create_node_route))
        .route("/api/nodes/:id", get(get_node_detail).patch(update_node_route))
        .route("/api/edges", post(create_edge_route))
        .with_state(pool)
}

async fn health() -> &'static str {
    "OK"
}

/// Reads the current `obligation_projection` rows (ADR-0005/ADR-0012). Never
/// writes; the projection remains the sole source this route reflects. Joins
/// read-only against the immutable `source_fragments` table for evidence
/// (ADR-0023), the same treatment ADR-0015 already gave `GET /api/candidates`.
async fn list_obligations(State(pool): State<PgPool>) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    #[allow(clippy::type_complexity)]
    let rows: Vec<(
        uuid::Uuid,
        String,
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<uuid::Uuid>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT op.obligation_id, op.status, op.updated_at, op.hard_due_at, op.soft_due_at, \
                op.source_fragment_id, sf.text \
         FROM obligation_projection op \
         LEFT JOIN source_fragments sf ON sf.id = op.source_fragment_id \
         ORDER BY op.updated_at DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    let body = rows
        .into_iter()
        .map(|(obligation_id, status, updated_at, hard_due_at, soft_due_at, source_fragment_id, source_text)| {
            json!({
                "obligation_id": obligation_id,
                "status": status,
                "updated_at": updated_at.to_rfc3339(),
                "hard_due_at": hard_due_at.map(|value| value.to_rfc3339()),
                "soft_due_at": soft_due_at.map(|value| value.to_rfc3339()),
                "source_fragment_id": source_fragment_id,
                "source_text": source_text,
            })
        })
        .collect::<Vec<_>>();

    Ok(Json(json!(body)))
}

/// A deterministic reason for one Daily Brief item (ADR-0022), with a second
/// evidence clause added by ADR-0023: cites the linked source fragment's
/// text when present, or states plainly that none is recorded. Never
/// fabricates evidence or groups obligations together.
fn daily_brief_reason(
    status: &str,
    hard_due_at: Option<chrono::DateTime<chrono::Utc>>,
    soft_due_at: Option<chrono::DateTime<chrono::Utc>>,
    source_text: Option<&str>,
) -> String {
    let due_clause = if status == "at_risk" {
        "Marked at risk.".to_string()
    } else if let Some(due) = hard_due_at {
        let days = (due - chrono::Utc::now()).num_days();
        if days < 0 {
            format!("Overdue by {} day(s).", -days)
        } else {
            format!("Due in {days} day(s).")
        }
    } else if let Some(due) = soft_due_at {
        format!("Expected around {}.", due.format("%Y-%m-%d"))
    } else {
        "No due date recorded.".to_string()
    };

    let evidence_clause = match source_text {
        Some(text) => {
            let truncated: String = text.chars().take(80).collect();
            format!("Last evidence: \"{truncated}\".")
        }
        None => "No evidence recorded.".to_string(),
    };

    format!("{due_clause} {evidence_clause}")
}

/// Ranks non-closed obligations by urgency and states a plain, deterministic
/// reason for each (ADR-0022): at-risk first, then soonest hard_due_at, then
/// soonest soft_due_at, then most-recently-updated. Read-only; a plain SQL
/// ORDER BY, not a scoring model. Joins read-only against `source_fragments`
/// for evidence (ADR-0023).
async fn daily_brief(State(pool): State<PgPool>) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    #[allow(clippy::type_complexity)]
    let rows: Vec<(
        uuid::Uuid,
        String,
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<uuid::Uuid>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT op.obligation_id, op.status, op.updated_at, op.hard_due_at, op.soft_due_at, \
                op.source_fragment_id, sf.text \
         FROM obligation_projection op \
         LEFT JOIN source_fragments sf ON sf.id = op.source_fragment_id \
         WHERE op.status <> 'closed' \
         ORDER BY (op.status = 'at_risk') DESC, op.hard_due_at ASC NULLS LAST, \
                  op.soft_due_at ASC NULLS LAST, op.updated_at DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    let body = rows
        .into_iter()
        .map(|(obligation_id, status, updated_at, hard_due_at, soft_due_at, source_fragment_id, source_text)| {
            let reason = daily_brief_reason(&status, hard_due_at, soft_due_at, source_text.as_deref());
            json!({
                "obligation_id": obligation_id,
                "status": status,
                "updated_at": updated_at.to_rfc3339(),
                "hard_due_at": hard_due_at.map(|value| value.to_rfc3339()),
                "soft_due_at": soft_due_at.map(|value| value.to_rfc3339()),
                "source_fragment_id": source_fragment_id,
                "reason": reason,
            })
        })
        .collect::<Vec<_>>();

    Ok(Json(json!(body)))
}

const TIME_HORIZON_BUCKETS: [&str; 5] = ["overdue", "next_7_days", "next_30_days", "next_90_days", "beyond"];

/// Buckets one Obligation by its effective due date (ADR-0029): hard_due_at
/// if present, else soft_due_at, else none. An at_risk Obligation with no
/// date at all lands in "overdue" (the one exception to pure date
/// bucketing); every other combination buckets purely by date.
fn time_horizon_bucket(status: &str, hard_due_at: Option<chrono::DateTime<chrono::Utc>>, soft_due_at: Option<chrono::DateTime<chrono::Utc>>) -> &'static str {
    let effective_due_at = hard_due_at.or(soft_due_at);
    let Some(due) = effective_due_at else {
        return if status == "at_risk" { "overdue" } else { "beyond" };
    };
    let days = (due - chrono::Utc::now()).num_days();
    if days < 0 {
        "overdue"
    } else if days <= 7 {
        "next_7_days"
    } else if days <= 30 {
        "next_30_days"
    } else if days <= 90 {
        "next_90_days"
    } else {
        "beyond"
    }
}

/// Groups non-closed Obligations by due-date window (ADR-0029): Overdue,
/// Next 7/30/90 days, Beyond/no date. Read-only; reuses the exact same
/// evidence join and `daily_brief_reason` the Daily Brief already uses --
/// this is a different lens (when it's due) on the same data, not a new
/// scoring model. Soonest-due-first within each bucket; an empty bucket is
/// simply omitted from the response.
async fn time_horizon(State(pool): State<PgPool>) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    #[allow(clippy::type_complexity)]
    let rows: Vec<(
        uuid::Uuid,
        String,
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<uuid::Uuid>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT op.obligation_id, op.status, op.updated_at, op.hard_due_at, op.soft_due_at, \
                op.source_fragment_id, sf.text \
         FROM obligation_projection op \
         LEFT JOIN source_fragments sf ON sf.id = op.source_fragment_id \
         WHERE op.status <> 'closed' \
         ORDER BY COALESCE(op.hard_due_at, op.soft_due_at) ASC NULLS LAST, op.updated_at DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    let mut buckets: std::collections::HashMap<&'static str, Vec<JsonValue>> = std::collections::HashMap::new();
    for (obligation_id, status, updated_at, hard_due_at, soft_due_at, source_fragment_id, source_text) in rows {
        let bucket = time_horizon_bucket(&status, hard_due_at, soft_due_at);
        let reason = daily_brief_reason(&status, hard_due_at, soft_due_at, source_text.as_deref());
        buckets.entry(bucket).or_default().push(json!({
            "obligation_id": obligation_id,
            "status": status,
            "updated_at": updated_at.to_rfc3339(),
            "hard_due_at": hard_due_at.map(|value| value.to_rfc3339()),
            "soft_due_at": soft_due_at.map(|value| value.to_rfc3339()),
            "source_fragment_id": source_fragment_id,
            "reason": reason,
        }));
    }

    let body: JsonValue = TIME_HORIZON_BUCKETS
        .into_iter()
        .filter_map(|bucket| buckets.remove(bucket).map(|items| (bucket.to_string(), json!(items))))
        .collect::<serde_json::Map<String, JsonValue>>()
        .into();

    Ok(Json(body))
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
/// Never writes.
async fn list_candidates(State(pool): State<PgPool>) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let rows: Vec<CandidateWithSource> = sqlx::query_as(
        "SELECT cp.candidate_id, cp.candidate_type, cp.statement, cp.validation_state, cp.confidence, \
                cp.source_fragment_id, cp.promoted_obligation_id, sf.text AS source_text, sf.speaker \
         FROM candidate_projection cp \
         LEFT JOIN source_fragments sf ON sf.id = cp.source_fragment_id \
         ORDER BY cp.candidate_id",
    )
    .fetch_all(&pool)
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    Ok(Json(json!(rows.iter().map(candidate_with_source_json).collect::<Vec<_>>())))
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
            format!("candidate is already \"{}\", not \"candidate\"", current.validation_state),
        ));
    }

    extraction::transition_candidate(pool, id, event_type, json!({}))
        .await
        .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    extraction::rebuild_candidate_projection(pool)
        .await
        .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

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
async fn accept_candidate(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    transition_candidate_route(&pool, id, "accepted").await
}

/// Rejects a candidate still in the `candidate` state (ADR-0024).
async fn reject_candidate(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    transition_candidate_route(&pool, id, "rejected").await
}

/// Promotes an `accepted` candidate into a new Obligation (ADR-0027).
/// `409` unless the candidate is currently `accepted` -- promotion is
/// one-way, matching accept/reject's own one-way 409 semantics. The new
/// Obligation carries the candidate's `source_fragment_id` forward as its
/// own evidence link (ADR-0023); no due date is implied by a candidate.
async fn promote_candidate(
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

    if current.validation_state != "accepted" {
        return Err((
            axum::http::StatusCode::CONFLICT,
            format!("candidate is \"{}\", not \"accepted\"", current.validation_state),
        ));
    }

    let obligation_id = Uuid::new_v4();
    obligation::append_event(
        &pool,
        obligation_id,
        obligation::ObligationEventType::Created,
        json!({"status": "open", "source_fragment_id": current.source_fragment_id}),
    )
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    extraction::transition_candidate(&pool, id, "promoted", json!({"obligation_id": obligation_id}))
        .await
        .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    obligation::rebuild_projection(&pool)
        .await
        .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    extraction::rebuild_candidate_projection(&pool)
        .await
        .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    #[allow(clippy::type_complexity)]
    let (obligation_id, status, updated_at, hard_due_at, soft_due_at, source_fragment_id, source_text): (
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
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

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

#[derive(Debug, Deserialize)]
struct NodeQuery {
    node_type: Option<String>,
}

/// Lists nodes, optionally filtered by `?node_type=` (ADR-0025). Read-only.
async fn list_nodes_route(State(pool): State<PgPool>, Query(params): Query<NodeQuery>) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let nodes = graph::list_nodes(&pool, params.node_type.as_deref())
        .await
        .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok(Json(json!(nodes)))
}

#[derive(Debug, Deserialize)]
struct CreateNodeRequest {
    node_type: String,
    canonical_text: String,
    attributes: Option<JsonValue>,
}

/// Creates one node (ADR-0025), the graph substrate's first write route.
async fn create_node_route(State(pool): State<PgPool>, Json(body): Json<CreateNodeRequest>) -> Result<Response, (axum::http::StatusCode, String)> {
    let id = graph::create_node(&pool, &body.node_type, &body.canonical_text, body.attributes.unwrap_or_else(|| json!({})))
        .await
        .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let node = graph::get_node(&pool, id).await.map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok((axum::http::StatusCode::CREATED, Json(json!(node))).into_response())
}

#[derive(Debug, FromRow)]
struct NeighborRow {
    id: Uuid,
    from_id: Uuid,
    to_id: Uuid,
    edge_type: String,
    confidence: Option<f32>,
    neighbor_id: Option<Uuid>,
    neighbor_node_type: Option<String>,
    neighbor_canonical_text: Option<String>,
    obligation_id: Option<Uuid>,
    obligation_status: Option<String>,
    obligation_hard_due_at: Option<chrono::DateTime<chrono::Utc>>,
    obligation_soft_due_at: Option<chrono::DateTime<chrono::Utc>>,
    obligation_source_text: Option<String>,
}

/// Orders like the Daily Brief's own `ASC NULLS LAST`: a due date sorts
/// before no due date at all, regardless of which of the two fields it is.
fn due_date_sort_key(value: Option<chrono::DateTime<chrono::Utc>>) -> (u8, i64) {
    match value {
        Some(due) => (0, due.timestamp()),
        None => (1, 0),
    }
}

/// Reads one node plus its one-hop neighborhood of edges (ADR-0025): every
/// edge touching this node, paired with a summary of the node on the other
/// end when that end is itself a `nodes` row. An edge into an Obligation id
/// resolves against `obligation_projection` instead (ADR-0028, amending
/// ADR-0025's own scope): status, due dates, and the same `reason` string
/// the Daily Brief shows. An id found in neither table still reports a
/// null neighbor, unchanged from ADR-0025. A `person` node additionally
/// gets a `relationship` object grouping its resolved Obligations into
/// `at_risk`/`open` (closed excluded), ordered the same way the Daily
/// Brief orders them.
async fn get_node_detail(State(pool): State<PgPool>, Path(id): Path<Uuid>) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let node = graph::get_node(&pool, id).await.map_err(|error| match error {
        sqlx::Error::RowNotFound => (axum::http::StatusCode::NOT_FOUND, "node not found".to_string()),
        other => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
    })?;

    let neighbors: Vec<NeighborRow> = sqlx::query_as(
        "SELECT e.id, e.from_id, e.to_id, e.edge_type, e.confidence, \
                n.id AS neighbor_id, n.node_type AS neighbor_node_type, n.canonical_text AS neighbor_canonical_text, \
                op.obligation_id AS obligation_id, op.status AS obligation_status, \
                op.hard_due_at AS obligation_hard_due_at, op.soft_due_at AS obligation_soft_due_at, \
                sf.text AS obligation_source_text \
         FROM edges e \
         LEFT JOIN nodes n ON n.id = (CASE WHEN e.from_id = $1 THEN e.to_id ELSE e.from_id END) \
         LEFT JOIN obligation_projection op ON op.obligation_id = (CASE WHEN e.from_id = $1 THEN e.to_id ELSE e.from_id END) \
         LEFT JOIN source_fragments sf ON sf.id = op.source_fragment_id \
         WHERE e.from_id = $1 OR e.to_id = $1",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    let neighbors_json: Vec<JsonValue> = neighbors
        .iter()
        .map(|row| {
            let neighbor = if let Some(neighbor_id) = row.neighbor_id {
                Some(json!({
                    "id": neighbor_id,
                    "node_type": row.neighbor_node_type,
                    "canonical_text": row.neighbor_canonical_text,
                }))
            } else {
                row.obligation_id.map(|obligation_id| {
                    json!({
                        "id": obligation_id,
                        "type": "obligation",
                        "status": row.obligation_status,
                        "hard_due_at": row.obligation_hard_due_at.map(|value| value.to_rfc3339()),
                        "soft_due_at": row.obligation_soft_due_at.map(|value| value.to_rfc3339()),
                        "reason": daily_brief_reason(
                            row.obligation_status.as_deref().unwrap_or(""),
                            row.obligation_hard_due_at,
                            row.obligation_soft_due_at,
                            row.obligation_source_text.as_deref(),
                        ),
                    })
                })
            };
            json!({
                "edge_id": row.id,
                "from_id": row.from_id,
                "to_id": row.to_id,
                "edge_type": row.edge_type,
                "confidence": row.confidence,
                "neighbor": neighbor,
            })
        })
        .collect();

    let relationship = if node.node_type == "person" {
        let mut linked: Vec<&NeighborRow> = neighbors
            .iter()
            .filter(|row| matches!(row.obligation_status.as_deref(), Some(status) if status != "closed"))
            .collect();
        linked.sort_by_key(|row| (due_date_sort_key(row.obligation_hard_due_at), due_date_sort_key(row.obligation_soft_due_at)));

        let mut at_risk: Vec<JsonValue> = Vec::new();
        let mut open: Vec<JsonValue> = Vec::new();
        for row in linked {
            let entry = json!({
                "obligation_id": row.obligation_id,
                "status": row.obligation_status,
                "hard_due_at": row.obligation_hard_due_at.map(|value| value.to_rfc3339()),
                "soft_due_at": row.obligation_soft_due_at.map(|value| value.to_rfc3339()),
                "reason": daily_brief_reason(
                    row.obligation_status.as_deref().unwrap_or(""),
                    row.obligation_hard_due_at,
                    row.obligation_soft_due_at,
                    row.obligation_source_text.as_deref(),
                ),
            });
            if row.obligation_status.as_deref() == Some("at_risk") {
                at_risk.push(entry);
            } else {
                open.push(entry);
            }
        }
        Some(json!({ "at_risk": at_risk, "open": open }))
    } else {
        None
    };

    Ok(Json(json!({
        "id": node.id,
        "node_type": node.node_type,
        "canonical_text": node.canonical_text,
        "attributes": node.attributes,
        "lifecycle_state": node.lifecycle_state,
        "neighbors": neighbors_json,
        "relationship": relationship,
    })))
}

#[derive(Debug, Deserialize)]
struct UpdateNodeRequest {
    canonical_text: Option<String>,
    lifecycle_state: Option<String>,
    attributes: Option<JsonValue>,
}

/// Enriches an existing node (ADR-0025): a shallow merge of `attributes`,
/// never a wholesale replace. `404` for an unknown id.
async fn update_node_route(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateNodeRequest>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let node = graph::update_node(&pool, id, body.canonical_text.as_deref(), body.lifecycle_state.as_deref(), body.attributes.as_ref())
        .await
        .map_err(|error| match error {
            sqlx::Error::RowNotFound => (axum::http::StatusCode::NOT_FOUND, "node not found".to_string()),
            other => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
        })?;
    Ok(Json(json!(node)))
}

#[derive(Debug, Deserialize)]
struct CreateEdgeRequest {
    from_id: Uuid,
    to_id: Uuid,
    edge_type: String,
    confidence: Option<f32>,
}

/// Creates one edge between any two entity ids (ADR-0025/ADR-0009).
async fn create_edge_route(State(pool): State<PgPool>, Json(body): Json<CreateEdgeRequest>) -> Result<Response, (axum::http::StatusCode, String)> {
    let id = graph::create_edge(&pool, body.from_id, body.to_id, &body.edge_type, body.confidence)
        .await
        .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let edge = graph::get_edge(&pool, id).await.map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok((axum::http::StatusCode::CREATED, Json(json!(edge))).into_response())
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

    /// ADR-0020: GET /api/obligations must surface hard_due_at/soft_due_at.
    #[tokio::test]
    async fn obligations_route_includes_due_date_fields() {
        let pool = test_pool().await;
        let obligation_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            obligation_id,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open", "hard_due_at": "2026-09-01T00:00:00Z"}),
        )
        .await
        .expect("append created event with a due date");
        crate::obligation::rebuild_projection(&pool).await.expect("rebuild projection");

        let response = app(pool.clone())
            .oneshot(Request::builder().uri("/api/obligations").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let row = parsed
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["obligation_id"] == obligation_id.to_string())
            .expect("the just-created obligation must be present");
        assert_eq!(row["hard_due_at"], "2026-09-01T00:00:00+00:00");
        assert!(row["soft_due_at"].is_null(), "an unset soft_due_at must serialize as null, not be omitted");
    }

    /// ADR-0023: GET /api/obligations must surface source_fragment_id/source_text.
    #[tokio::test]
    async fn obligations_route_includes_source_fragment_evidence() {
        let pool = test_pool().await;
        let fragment_id = graph::create_source_fragment(
            &pool,
            uuid::Uuid::new_v4(),
            "We committed to a two-week transition plan.",
            "obligation-evidence-test-hash",
        )
        .await
        .expect("create source fragment");
        let obligation_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            obligation_id,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open", "source_fragment_id": fragment_id.to_string()}),
        )
        .await
        .expect("append created event with a linked source fragment");
        crate::obligation::rebuild_projection(&pool).await.expect("rebuild projection");

        let response = app(pool.clone())
            .oneshot(Request::builder().uri("/api/obligations").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let row = parsed
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["obligation_id"] == obligation_id.to_string())
            .expect("the just-created obligation must be present");
        assert_eq!(row["source_fragment_id"], fragment_id.to_string());
        assert_eq!(row["source_text"], "We committed to a two-week transition plan.");
    }

    /// ADR-0023: the Daily Brief's reason cites linked evidence, or says
    /// plainly that none is recorded -- it never fabricates either.
    #[tokio::test]
    async fn daily_brief_reason_cites_evidence_when_linked_and_states_none_when_not() {
        let pool = test_pool().await;
        let fragment_id = graph::create_source_fragment(
            &pool,
            uuid::Uuid::new_v4(),
            "We committed to a two-week transition plan.",
            "daily-brief-evidence-test-hash",
        )
        .await
        .expect("create source fragment");

        let with_evidence = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            with_evidence,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open", "source_fragment_id": fragment_id.to_string()}),
        )
        .await
        .expect("append obligation linked to evidence");

        let without_evidence = uuid::Uuid::new_v4();
        crate::obligation::append_event(&pool, without_evidence, crate::obligation::ObligationEventType::Created, json!({"status": "open"}))
            .await
            .expect("append obligation without evidence");

        crate::obligation::rebuild_projection(&pool).await.expect("rebuild projection");

        let response = app(pool.clone())
            .oneshot(Request::builder().uri("/api/daily-brief").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let rows = parsed.as_array().unwrap();
        let with_row = rows.iter().find(|row| row["obligation_id"] == with_evidence.to_string()).expect("linked obligation present");
        let without_row = rows.iter().find(|row| row["obligation_id"] == without_evidence.to_string()).expect("unlinked obligation present");
        assert_eq!(with_row["reason"], "No due date recorded. Last evidence: \"We committed to a two-week transition plan.\".");
        assert_eq!(without_row["reason"], "No due date recorded. No evidence recorded.");
    }

    /// ADR-0022: at-risk outranks any due date, closed is excluded entirely.
    #[tokio::test]
    async fn daily_brief_ranks_at_risk_first_and_excludes_closed() {
        let pool = test_pool().await;

        let far_future_open = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            far_future_open,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open", "hard_due_at": "2030-01-01T00:00:00Z"}),
        )
        .await
        .expect("append open obligation with a distant due date");

        let at_risk_no_date = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            at_risk_no_date,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "at_risk"}),
        )
        .await
        .expect("append at_risk obligation with no due date");

        let closed = uuid::Uuid::new_v4();
        crate::obligation::append_event(&pool, closed, crate::obligation::ObligationEventType::Created, json!({"status": "open"}))
            .await
            .expect("append obligation to be closed");
        crate::obligation::append_event(&pool, closed, crate::obligation::ObligationEventType::Closed, json!({}))
            .await
            .expect("close it");

        crate::obligation::rebuild_projection(&pool).await.expect("rebuild projection");

        let response = app(pool.clone())
            .oneshot(Request::builder().uri("/api/daily-brief").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let rows = parsed.as_array().unwrap();

        assert!(
            rows.iter().all(|row| row["obligation_id"] != closed.to_string()),
            "a closed obligation must never appear in the Daily Brief"
        );

        let at_risk_index = rows.iter().position(|row| row["obligation_id"] == at_risk_no_date.to_string());
        let far_future_index = rows.iter().position(|row| row["obligation_id"] == far_future_open.to_string());
        assert!(at_risk_index.is_some() && far_future_index.is_some(), "both open items must be present");
        assert!(
            at_risk_index.unwrap() < far_future_index.unwrap(),
            "at_risk must outrank an open obligation with a due date, however distant"
        );
        assert_eq!(rows[at_risk_index.unwrap()]["reason"], "Marked at risk. No evidence recorded.");
    }

    /// ADR-0023: the reason cites the linked source fragment's text, or
    /// states plainly that none is recorded.
    #[tokio::test]
    async fn daily_brief_reason_cites_linked_evidence() {
        let pool = test_pool().await;
        let meeting_id = uuid::Uuid::new_v4();
        let fragment_id = graph::create_source_fragment(&pool, meeting_id, "Roopa: please send the transition plan.", "brief-evidence-hash")
            .await
            .expect("create source fragment");

        let with_evidence = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            with_evidence,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "at_risk", "source_fragment_id": fragment_id.to_string()}),
        )
        .await
        .expect("append obligation with linked evidence");

        let without_evidence = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            without_evidence,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "at_risk"}),
        )
        .await
        .expect("append obligation with no linked evidence");

        crate::obligation::rebuild_projection(&pool).await.expect("rebuild projection");

        let response = app(pool.clone())
            .oneshot(Request::builder().uri("/api/daily-brief").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let rows = parsed.as_array().unwrap();

        let evidenced_row = rows.iter().find(|row| row["obligation_id"] == with_evidence.to_string()).expect("present");
        assert_eq!(evidenced_row["reason"], "Marked at risk. Last evidence: \"Roopa: please send the transition plan.\".");
        assert_eq!(evidenced_row["source_fragment_id"], fragment_id.to_string());

        let unevidenced_row = rows.iter().find(|row| row["obligation_id"] == without_evidence.to_string()).expect("present");
        assert_eq!(unevidenced_row["reason"], "Marked at risk. No evidence recorded.");
        assert!(unevidenced_row["source_fragment_id"].is_null());
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
    async fn accept_route_transitions_a_candidate_still_in_the_candidate_state() {
        let pool = test_pool().await;
        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(&pool, candidate_id, "risk", "stated risk", uuid::Uuid::new_v4(), Some(0.7), None)
            .await
            .expect("extract candidate");
        extraction::rebuild_candidate_projection(&pool).await.expect("rebuild candidate projection");

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

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert_eq!(parsed.get("validation_state").and_then(|v| v.as_str()), Some("accepted"));
    }

    #[tokio::test]
    async fn reject_route_transitions_a_candidate_still_in_the_candidate_state() {
        let pool = test_pool().await;
        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(&pool, candidate_id, "risk", "stated risk", uuid::Uuid::new_v4(), Some(0.7), None)
            .await
            .expect("extract candidate");
        extraction::rebuild_candidate_projection(&pool).await.expect("rebuild candidate projection");

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

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert_eq!(parsed.get("validation_state").and_then(|v| v.as_str()), Some("rejected"));
    }

    #[tokio::test]
    async fn accept_route_returns_409_for_an_already_transitioned_candidate() {
        let pool = test_pool().await;
        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(&pool, candidate_id, "risk", "stated risk", uuid::Uuid::new_v4(), Some(0.7), None)
            .await
            .expect("extract candidate");
        extraction::transition_candidate(&pool, candidate_id, "accepted", json!({}))
            .await
            .expect("append accepted event");
        extraction::rebuild_candidate_projection(&pool).await.expect("rebuild candidate projection");

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
        let fragment_id = graph::create_source_fragment(&pool, uuid::Uuid::new_v4(), "We will migrate the pipeline by Q3.", "promote-test-hash")
            .await
            .expect("create source fragment");
        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(&pool, candidate_id, "commitment", "migrate the pipeline", fragment_id, Some(0.9), None)
            .await
            .expect("extract candidate");
        extraction::transition_candidate(&pool, candidate_id, "accepted", json!({}))
            .await
            .expect("append accepted event");
        extraction::rebuild_candidate_projection(&pool).await.expect("rebuild candidate projection");

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

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let created: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert_eq!(created.get("status").and_then(|v| v.as_str()), Some("open"));
        assert_eq!(created.get("source_fragment_id").and_then(|v| v.as_str()), Some(fragment_id.to_string().as_str()));
        let obligation_id = created.get("obligation_id").and_then(|v| v.as_str()).expect("obligation_id present").to_string();

        let candidates_response = app(pool)
            .oneshot(Request::builder().uri("/api/candidates").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let candidates_body = axum::body::to_bytes(candidates_response.into_body(), usize::MAX).await.unwrap();
        let candidates: JsonValue = serde_json::from_slice(&candidates_body).expect("valid json body");
        let candidate_row = candidates
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row.get("candidate_id").and_then(|v| v.as_str()) == Some(&candidate_id.to_string()))
            .expect("the promoted candidate must still be present");
        assert_eq!(candidate_row.get("validation_state").and_then(|v| v.as_str()), Some("promoted"));
        assert_eq!(candidate_row.get("promoted_obligation_id").and_then(|v| v.as_str()), Some(obligation_id.as_str()));
    }

    #[tokio::test]
    async fn promote_route_returns_409_for_a_candidate_not_yet_accepted() {
        let pool = test_pool().await;
        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(&pool, candidate_id, "risk", "stated risk", uuid::Uuid::new_v4(), Some(0.7), None)
            .await
            .expect("extract candidate");
        extraction::rebuild_candidate_projection(&pool).await.expect("rebuild candidate projection");

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
    async fn promote_route_returns_409_for_an_already_promoted_candidate() {
        let pool = test_pool().await;
        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(&pool, candidate_id, "risk", "stated risk", uuid::Uuid::new_v4(), Some(0.7), None)
            .await
            .expect("extract candidate");
        extraction::transition_candidate(&pool, candidate_id, "accepted", json!({}))
            .await
            .expect("append accepted event");
        extraction::transition_candidate(&pool, candidate_id, "promoted", json!({"obligation_id": uuid::Uuid::new_v4()}))
            .await
            .expect("append promoted event");
        extraction::rebuild_candidate_projection(&pool).await.expect("rebuild candidate projection");

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

    /// ADR-0025: create, list (filtered), enrich, and detail-with-neighbors
    /// round-trip through the HTTP routes, not just graph.rs directly.
    #[tokio::test]
    async fn node_create_list_enrich_and_detail_round_trip() {
        let pool = test_pool().await;

        let create_response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/nodes")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"node_type": "person", "canonical_text": "Node Route Test Person", "attributes": {"role": "manager"}}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_response.status(), axum::http::StatusCode::CREATED);
        let body = axum::body::to_bytes(create_response.into_body(), usize::MAX).await.unwrap();
        let created: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let node_id = created["id"].as_str().expect("created node has an id").to_string();

        let list_response = app(pool.clone())
            .oneshot(Request::builder().uri("/api/nodes?node_type=person").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(list_response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(list_response.into_body(), usize::MAX).await.unwrap();
        let listed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert!(listed.as_array().unwrap().iter().any(|row| row["id"] == node_id), "the just-created node must be listed");

        let patch_response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/nodes/{node_id}"))
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"attributes": {"team": "platform"}}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(patch_response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(patch_response.into_body(), usize::MAX).await.unwrap();
        let patched: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert_eq!(patched["attributes"]["role"], "manager", "enrichment must not clobber a previously-recorded attribute");
        assert_eq!(patched["attributes"]["team"], "platform", "the newly-enriched attribute must be present");

        let detail_response = app(pool.clone())
            .oneshot(Request::builder().uri(format!("/api/nodes/{node_id}")).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(detail_response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(detail_response.into_body(), usize::MAX).await.unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert!(detail["neighbors"].is_array(), "detail response must include a neighbors array");
    }

    #[tokio::test]
    async fn node_detail_route_returns_404_for_unknown_node() {
        let pool = test_pool().await;
        let response = app(pool)
            .oneshot(Request::builder().uri(format!("/api/nodes/{}", uuid::Uuid::new_v4())).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    /// ADR-0025: an edge's neighbor summary must be present when the other
    /// end is a real node, and null (not a failed join) when it is not --
    /// ADR-0028 narrows "not" to mean "resolves against neither `nodes` nor
    /// `obligation_projection`", proven by
    /// `node_detail_resolves_a_real_linked_obligation_with_status_and_reason`
    /// below.
    #[tokio::test]
    async fn node_detail_includes_neighbor_summary_and_handles_a_non_node_edge_target() {
        let pool = test_pool().await;
        let person_id = graph::create_node(&pool, "person", "Neighbor Test Person", json!({})).await.expect("create person");
        let risk_id = graph::create_node(&pool, "risk", "Neighbor Test Risk", json!({})).await.expect("create risk");
        graph::create_edge(&pool, person_id, risk_id, "flagged", None).await.expect("create edge to a real node");
        let obligation_id = uuid::Uuid::new_v4(); // a genuinely unknown id: neither a nodes row nor a real Obligation.
        graph::create_edge(&pool, person_id, obligation_id, "made", None).await.expect("create edge to a non-node id");

        let response = app(pool)
            .oneshot(Request::builder().uri(format!("/api/nodes/{person_id}")).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let neighbors = detail["neighbors"].as_array().unwrap();

        let to_risk = neighbors.iter().find(|edge| edge["to_id"] == risk_id.to_string()).expect("edge to the risk node present");
        assert_eq!(to_risk["neighbor"]["id"], risk_id.to_string());
        assert_eq!(to_risk["neighbor"]["canonical_text"], "Neighbor Test Risk");

        let to_obligation = neighbors.iter().find(|edge| edge["to_id"] == obligation_id.to_string()).expect("edge to the non-node id present");
        assert!(to_obligation["neighbor"].is_null(), "an edge whose other end is not a nodes row must report a null neighbor, not fail");
    }

    /// ADR-0028: an edge into a *real* Obligation resolves with its status
    /// and the same `reason` text the Daily Brief shows, and a person's
    /// linked, non-closed Obligations are grouped into at_risk/open.
    #[tokio::test]
    async fn node_detail_resolves_a_real_linked_obligation_with_status_and_reason() {
        let pool = test_pool().await;
        let person_id = graph::create_node(&pool, "person", "Relationship Test Person", json!({})).await.expect("create person");

        let at_risk_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            at_risk_id,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "at_risk"}),
        )
        .await
        .expect("append an at-risk obligation");
        graph::create_edge(&pool, person_id, at_risk_id, "owns", None).await.expect("link person to the at-risk obligation");

        let closed_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(&pool, closed_id, crate::obligation::ObligationEventType::Created, json!({"status": "open"}))
            .await
            .expect("append an obligation to be closed");
        crate::obligation::append_event(&pool, closed_id, crate::obligation::ObligationEventType::Closed, json!({}))
            .await
            .expect("close it");
        graph::create_edge(&pool, person_id, closed_id, "owns", None).await.expect("link person to the closed obligation");

        crate::obligation::rebuild_projection(&pool).await.expect("rebuild projection");

        let response = app(pool)
            .oneshot(Request::builder().uri(format!("/api/nodes/{person_id}")).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");

        let neighbors = detail["neighbors"].as_array().unwrap();
        let to_at_risk = neighbors.iter().find(|edge| edge["to_id"] == at_risk_id.to_string()).expect("edge to the at-risk obligation present");
        assert_eq!(to_at_risk["neighbor"]["type"], "obligation");
        assert_eq!(to_at_risk["neighbor"]["status"], "at_risk");
        assert_eq!(to_at_risk["neighbor"]["reason"], "Marked at risk. No evidence recorded.");

        let relationship = &detail["relationship"];
        let at_risk_group = relationship["at_risk"].as_array().unwrap();
        assert!(at_risk_group.iter().any(|entry| entry["obligation_id"] == at_risk_id.to_string()), "the at-risk obligation must appear in the at_risk group");
        let open_group = relationship["open"].as_array().unwrap();
        assert!(
            !open_group.iter().any(|entry| entry["obligation_id"] == closed_id.to_string()),
            "a closed obligation must never appear in either relationship group"
        );
        assert!(
            !at_risk_group.iter().any(|entry| entry["obligation_id"] == closed_id.to_string()),
            "a closed obligation must never appear in either relationship group"
        );
    }

    /// ADR-0028: only person nodes get a `relationship` grouping.
    #[tokio::test]
    async fn node_detail_omits_relationship_grouping_for_non_person_nodes() {
        let pool = test_pool().await;
        let risk_id = graph::create_node(&pool, "risk", "Non-Person Relationship Test", json!({})).await.expect("create risk node");

        let response = app(pool)
            .oneshot(Request::builder().uri(format!("/api/nodes/{risk_id}")).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert!(detail["relationship"].is_null(), "a non-person node must not get a relationship grouping");
    }

    #[tokio::test]
    async fn edge_create_route_round_trips() {
        let pool = test_pool().await;
        let from_id = graph::create_node(&pool, "person", "Edge Route Test From", json!({})).await.expect("create from-node");
        let to_id = graph::create_node(&pool, "person", "Edge Route Test To", json!({})).await.expect("create to-node");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/edges")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"from_id": from_id, "to_id": to_id, "edge_type": "collaborates_with"}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let created: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert_eq!(created["edge_type"], "collaborates_with");
    }

    /// ADR-0029: buckets by effective due date, at-risk-with-no-date is the
    /// one exception (lands in overdue), and closed Obligations never appear.
    #[tokio::test]
    async fn time_horizon_buckets_by_due_date_with_the_at_risk_no_date_exception() {
        let pool = test_pool().await;

        let overdue_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            overdue_id,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open", "hard_due_at": (chrono::Utc::now() - chrono::Duration::days(3)).to_rfc3339()}),
        )
        .await
        .expect("append overdue obligation");

        let next_7_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            next_7_id,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open", "hard_due_at": (chrono::Utc::now() + chrono::Duration::days(3)).to_rfc3339()}),
        )
        .await
        .expect("append next-7-days obligation");

        let at_risk_no_date_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(&pool, at_risk_no_date_id, crate::obligation::ObligationEventType::Created, json!({"status": "at_risk"}))
            .await
            .expect("append at-risk obligation with no date");

        let closed_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(&pool, closed_id, crate::obligation::ObligationEventType::Created, json!({"status": "open"}))
            .await
            .expect("append obligation to close");
        crate::obligation::append_event(&pool, closed_id, crate::obligation::ObligationEventType::Closed, json!({}))
            .await
            .expect("close it");

        crate::obligation::rebuild_projection(&pool).await.expect("rebuild projection");

        let response = app(pool)
            .oneshot(Request::builder().uri("/api/time-horizon").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");

        let overdue = parsed["overdue"].as_array().expect("overdue bucket present");
        assert!(overdue.iter().any(|row| row["obligation_id"] == overdue_id.to_string()), "a past-due obligation must land in overdue");
        assert!(
            overdue.iter().any(|row| row["obligation_id"] == at_risk_no_date_id.to_string()),
            "an at_risk obligation with no date must land in overdue, not beyond"
        );

        let next_7 = parsed["next_7_days"].as_array().expect("next_7_days bucket present");
        assert!(next_7.iter().any(|row| row["obligation_id"] == next_7_id.to_string()));

        for bucket in ["overdue", "next_7_days", "next_30_days", "next_90_days", "beyond"] {
            if let Some(items) = parsed.get(bucket).and_then(|value| value.as_array()) {
                assert!(items.iter().all(|row| row["obligation_id"] != closed_id.to_string()), "a closed obligation must never appear in any bucket");
            }
        }
    }
}
