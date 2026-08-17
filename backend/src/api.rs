use crate::audit;
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
/// route (ADR-0027), plus the Time Horizon route (ADR-0029), plus the
/// Suggested Focus Blocks route (ADR-0031), plus the atomic meeting-ingestion
/// route (ADR-0034), plus the meeting detail read (ADR-0036), plus the
/// meeting-scoped candidate listing route (ADR-0037), plus the dated,
/// any-source-type ingestion route (ADR-0040).
pub fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/obligations", get(list_obligations))
        .route("/api/obligations/:id", get(get_obligation_detail))
        .route("/api/daily-brief", get(daily_brief))
        .route("/api/time-horizon", get(time_horizon))
        .route("/api/focus-blocks", get(focus_blocks))
        .route("/api/meetings/ingest", post(ingest_meeting))
        .route("/api/meetings/:id", get(get_meeting_detail))
        .route("/api/meetings/:id/candidates", get(get_meeting_candidates))
        .route("/api/sources/ingest", post(ingest_source_route))
        .route("/api/candidates", get(list_candidates))
        .route("/api/candidates/:id/accept", post(accept_candidate))
        .route("/api/candidates/:id/reject", post(reject_candidate))
        .route("/api/candidates/:id/correct", post(correct_candidate))
        .route("/api/candidates/:id/promote", post(promote_candidate))
        .route("/api/source-fragments/:id/extract", post(extract_source_fragment))
        .route("/api/search", get(search))
        .route("/api/audit-events", get(list_audit_events))
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

#[derive(Debug, FromRow)]
struct ObligationLinkedNodeRow {
    edge_id: Uuid,
    edge_type: String,
    neighbor_id: Option<Uuid>,
    neighbor_node_type: Option<String>,
    neighbor_canonical_text: Option<String>,
}

/// Reads one Obligation by id (ADR-0047): the same fields
/// `GET /api/daily-brief` already returns per row (including `risk_signals`,
/// computed by the exact same function and `has_owner` subquery -- zero new
/// reasoning), plus `linked_nodes`: every edge touching this id, with the
/// other end resolved against `nodes` the same way `GET /api/nodes/:id`
/// resolves a neighbor (ADR-0025) -- an edge whose other end isn't a `nodes`
/// row reports a null neighbor, the same honest fallback. `404` for an
/// unknown id. Read-only.
async fn get_obligation_detail(State(pool): State<PgPool>, Path(id): Path<Uuid>) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    #[allow(clippy::type_complexity)]
    let row: (
        String,
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<uuid::Uuid>,
        Option<String>,
        bool,
        bool,
    ) = sqlx::query_as(
        "SELECT op.status, op.updated_at, op.hard_due_at, op.soft_due_at, \
                op.source_fragment_id, sf.text, \
                EXISTS ( \
                    SELECT 1 FROM edges e JOIN nodes n ON n.id = e.from_id \
                    WHERE e.to_id = op.obligation_id AND e.edge_type = 'owns' AND n.node_type = 'person' \
                ) AS has_owner, \
                EXISTS ( \
                    SELECT 1 FROM edges e WHERE e.from_id = op.obligation_id OR e.to_id = op.obligation_id \
                ) AS has_edges \
         FROM obligation_projection op \
         LEFT JOIN source_fragments sf ON sf.id = op.source_fragment_id \
         WHERE op.obligation_id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .map_err(|error| match error {
        sqlx::Error::RowNotFound => (axum::http::StatusCode::NOT_FOUND, "obligation not found".to_string()),
        other => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
    })?;
    let (status, updated_at, hard_due_at, soft_due_at, source_fragment_id, source_text, has_owner, has_edges) = row;

    let linked_rows: Vec<ObligationLinkedNodeRow> = sqlx::query_as(
        "SELECT e.id AS edge_id, e.edge_type, \
                n.id AS neighbor_id, n.node_type AS neighbor_node_type, n.canonical_text AS neighbor_canonical_text \
         FROM edges e \
         LEFT JOIN nodes n ON n.id = (CASE WHEN e.from_id = $1 THEN e.to_id ELSE e.from_id END) \
         WHERE e.from_id = $1 OR e.to_id = $1",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    let linked_nodes: Vec<JsonValue> = linked_rows
        .into_iter()
        .map(|linked| {
            json!({
                "edge_id": linked.edge_id,
                "edge_type": linked.edge_type,
                "node_id": linked.neighbor_id,
                "node_type": linked.neighbor_node_type,
                "canonical_text": linked.neighbor_canonical_text,
            })
        })
        .collect();

    Ok(Json(json!({
        "obligation_id": id,
        "status": status,
        "updated_at": updated_at.to_rfc3339(),
        "hard_due_at": hard_due_at.map(|value| value.to_rfc3339()),
        "soft_due_at": soft_due_at.map(|value| value.to_rfc3339()),
        "source_fragment_id": source_fragment_id,
        "source_text": source_text,
        "risk_signals": risk_signals(hard_due_at, soft_due_at, updated_at, source_fragment_id, has_owner, has_edges),
        "linked_nodes": linked_nodes,
    })))
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

const STALE_THRESHOLD_DAYS: i64 = 14;
const DATE_COMPRESSION_WINDOW_DAYS: i64 = 7;

/// Risk Engine v1 (ADR-0041): the two of PRODUCT-SPEC.md §7.1's nine
/// signals derivable today with zero schema change and zero fabricated
/// data. Each signal is independent and additive -- no combined severity
/// score is computed here, since weighting them together needs a model
/// this ADR does not decide. `has_owner` (ADR-0046) and `has_edges`
/// (ADR-0054, Congruence Engine v1) are computed by the caller from the
/// existing `edges` table, not by this function -- it stays pure and
/// directly unit-testable.
fn risk_signals(
    hard_due_at: Option<chrono::DateTime<chrono::Utc>>,
    soft_due_at: Option<chrono::DateTime<chrono::Utc>>,
    updated_at: chrono::DateTime<chrono::Utc>,
    source_fragment_id: Option<Uuid>,
    has_owner: bool,
    has_edges: bool,
) -> Vec<JsonValue> {
    let mut signals = Vec::new();

    if let (Some(due), None) = (hard_due_at.or(soft_due_at), source_fragment_id) {
        let days = (due - chrono::Utc::now()).num_days();
        if days <= DATE_COMPRESSION_WINDOW_DAYS {
            let explanation = if days < 0 {
                format!("Overdue by {} day(s) with no evidence linked.", -days)
            } else {
                format!("Due in {days} day(s) with no evidence linked.")
            };
            signals.push(json!({ "signal": "date_compression", "explanation": explanation }));
        }
    }

    let stale_days = (chrono::Utc::now() - updated_at).num_days();
    if stale_days > STALE_THRESHOLD_DAYS {
        signals.push(json!({
            "signal": "stale",
            "explanation": format!("No update in {stale_days} day(s) (stale threshold: {STALE_THRESHOLD_DAYS})."),
        }));
    }

    if !has_owner {
        signals.push(json!({ "signal": "unowned", "explanation": "No owner linked." }));
    }

    if !has_edges {
        signals.push(json!({ "signal": "isolated", "explanation": "Not linked to anyone or anything." }));
    }

    signals
}

/// Ranks non-closed obligations by urgency and states a plain, deterministic
/// reason for each (ADR-0022): at-risk first, then soonest hard_due_at, then
/// soonest soft_due_at, then most-recently-updated. Read-only; a plain SQL
/// ORDER BY, not a scoring model. Joins read-only against `source_fragments`
/// for evidence (ADR-0023). Also attaches `risk_signals` (ADR-0041/ADR-0046).
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
        bool,
        bool,
    )> = sqlx::query_as(
        "SELECT op.obligation_id, op.status, op.updated_at, op.hard_due_at, op.soft_due_at, \
                op.source_fragment_id, sf.text, \
                EXISTS ( \
                    SELECT 1 FROM edges e JOIN nodes n ON n.id = e.from_id \
                    WHERE e.to_id = op.obligation_id AND e.edge_type = 'owns' AND n.node_type = 'person' \
                ) AS has_owner, \
                EXISTS ( \
                    SELECT 1 FROM edges e WHERE e.from_id = op.obligation_id OR e.to_id = op.obligation_id \
                ) AS has_edges \
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
        .map(|(obligation_id, status, updated_at, hard_due_at, soft_due_at, source_fragment_id, source_text, has_owner, has_edges)| {
            let reason = daily_brief_reason(&status, hard_due_at, soft_due_at, source_text.as_deref());
            json!({
                "obligation_id": obligation_id,
                "status": status,
                "updated_at": updated_at.to_rfc3339(),
                "hard_due_at": hard_due_at.map(|value| value.to_rfc3339()),
                "soft_due_at": soft_due_at.map(|value| value.to_rfc3339()),
                "source_fragment_id": source_fragment_id,
                "source_text": source_text,
                "reason": reason,
                "risk_signals": risk_signals(hard_due_at, soft_due_at, updated_at, source_fragment_id, has_owner, has_edges),
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
/// simply omitted from the response. Also attaches `risk_signals` (ADR-0041/ADR-0046).
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
        bool,
        bool,
    )> = sqlx::query_as(
        "SELECT op.obligation_id, op.status, op.updated_at, op.hard_due_at, op.soft_due_at, \
                op.source_fragment_id, sf.text, \
                EXISTS ( \
                    SELECT 1 FROM edges e JOIN nodes n ON n.id = e.from_id \
                    WHERE e.to_id = op.obligation_id AND e.edge_type = 'owns' AND n.node_type = 'person' \
                ) AS has_owner, \
                EXISTS ( \
                    SELECT 1 FROM edges e WHERE e.from_id = op.obligation_id OR e.to_id = op.obligation_id \
                ) AS has_edges \
         FROM obligation_projection op \
         LEFT JOIN source_fragments sf ON sf.id = op.source_fragment_id \
         WHERE op.status <> 'closed' \
         ORDER BY COALESCE(op.hard_due_at, op.soft_due_at) ASC NULLS LAST, op.updated_at DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    let mut buckets: std::collections::HashMap<&'static str, Vec<JsonValue>> = std::collections::HashMap::new();
    for (obligation_id, status, updated_at, hard_due_at, soft_due_at, source_fragment_id, source_text, has_owner, has_edges) in rows {
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
            "risk_signals": risk_signals(hard_due_at, soft_due_at, updated_at, source_fragment_id, has_owner, has_edges),
        }));
    }

    let body: JsonValue = TIME_HORIZON_BUCKETS
        .into_iter()
        .filter_map(|bucket| buckets.remove(bucket).map(|items| (bucket.to_string(), json!(items))))
        .collect::<serde_json::Map<String, JsonValue>>()
        .into();

    Ok(Json(body))
}

#[derive(Debug, Clone, FromRow)]
struct FocusBlockRow {
    node_id: Uuid,
    node_type: String,
    canonical_text: String,
    obligation_id: Uuid,
    status: String,
    hard_due_at: Option<chrono::DateTime<chrono::Utc>>,
    soft_due_at: Option<chrono::DateTime<chrono::Utc>>,
    source_text: Option<String>,
}

/// Groups non-closed Obligations that share both a linked node *and* a Time
/// Horizon bucket into a Suggested Focus Block (ADR-0031, amended by
/// ADR-0052): a node linked to Obligations spanning several buckets now
/// forms one block per bucket, not one block spanning all of them --
/// "these belong together" is true in both the graph and the calendar
/// sense. Reuses `time_horizon_bucket`/`daily_brief_reason` verbatim -- no
/// new bucketing or reasoning logic, no schema change. A (node, bucket)
/// pair linked to fewer than two non-closed Obligations forms no block; a
/// closed Obligation is never counted. Blocks are ordered by urgency
/// (ADR-0050): any block containing an at_risk Obligation sorts first,
/// then soonest effective due date among its Obligations, then Obligation
/// count descending as a final tiebreak. No estimated time, no "Start
/// Focus Session" -- neither has real backing data.
async fn focus_blocks(State(pool): State<PgPool>) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let rows: Vec<FocusBlockRow> = sqlx::query_as(
        "SELECT n.id AS node_id, n.node_type, n.canonical_text, \
                op.obligation_id, op.status, op.hard_due_at, op.soft_due_at, sf.text AS source_text \
         FROM nodes n \
         JOIN edges e ON e.from_id = n.id OR e.to_id = n.id \
         JOIN obligation_projection op ON op.obligation_id = (CASE WHEN e.from_id = n.id THEN e.to_id ELSE e.from_id END) \
         LEFT JOIN source_fragments sf ON sf.id = op.source_fragment_id \
         WHERE op.status <> 'closed'",
    )
    .fetch_all(&pool)
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    struct Block {
        node_type: String,
        canonical_text: String,
        bucket: &'static str,
        obligations: std::collections::HashMap<Uuid, JsonValue>,
        has_at_risk: bool,
        soonest_due: Option<chrono::DateTime<chrono::Utc>>,
    }

    let mut blocks: std::collections::HashMap<(Uuid, &'static str), Block> = std::collections::HashMap::new();
    for row in rows {
        let bucket = time_horizon_bucket(&row.status, row.hard_due_at, row.soft_due_at);
        let reason = daily_brief_reason(&row.status, row.hard_due_at, row.soft_due_at, row.source_text.as_deref());
        let effective_due = row.hard_due_at.or(row.soft_due_at);
        let entry = blocks.entry((row.node_id, bucket)).or_insert_with(|| Block {
            node_type: row.node_type.clone(),
            canonical_text: row.canonical_text.clone(),
            bucket,
            obligations: std::collections::HashMap::new(),
            has_at_risk: false,
            soonest_due: None,
        });
        entry.has_at_risk = entry.has_at_risk || row.status == "at_risk";
        entry.soonest_due = match (entry.soonest_due, effective_due) {
            (Some(current), Some(candidate)) => Some(current.min(candidate)),
            (None, Some(candidate)) => Some(candidate),
            (current, None) => current,
        };
        entry.obligations.insert(
            row.obligation_id,
            json!({
                "obligation_id": row.obligation_id,
                "status": row.status,
                "hard_due_at": row.hard_due_at.map(|value| value.to_rfc3339()),
                "soft_due_at": row.soft_due_at.map(|value| value.to_rfc3339()),
                "reason": reason,
            }),
        );
    }

    let mut result: Vec<(bool, Option<chrono::DateTime<chrono::Utc>>, usize, JsonValue)> = blocks
        .into_iter()
        .filter(|(_, block)| block.obligations.len() >= 2)
        .map(|((node_id, _), block)| {
            let count = block.obligations.len();
            (
                block.has_at_risk,
                block.soonest_due,
                count,
                json!({
                    "node_id": node_id,
                    "node_type": block.node_type,
                    "canonical_text": block.canonical_text,
                    "time_horizon_bucket": block.bucket,
                    "obligations": block.obligations.into_values().collect::<Vec<_>>(),
                }),
            )
        })
        .collect();

    result.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| match (a.1, b.1) {
                (Some(left), Some(right)) => left.cmp(&right),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            })
            .then_with(|| b.2.cmp(&a.2))
    });

    Ok(Json(json!(result.into_iter().map(|(_, _, _, value)| value).collect::<Vec<_>>())))
}


#[derive(Debug, Deserialize)]
struct IngestMeetingRequest {
    title: String,
    occurred_at: Option<String>,
    organiser: Option<String>,
    #[serde(default)]
    participants: Vec<String>,
    transcript: String,
}

/// Ingests one meeting transcript atomically (ADR-0034): validates a
/// non-blank title/transcript before any write, then delegates to the
/// existing transcript module, which now wraps the Meeting node and every
/// fragment in one transaction -- a storage failure partway through can
/// never leave partial meeting memory. Never invokes a model, extraction,
/// or embedding; evidence capture stays available even when none is
/// configured (ADR-0011/ADR-0013's posture). Requires a structured
/// `occurred_at` (ADR-0040), the real-world event time, distinct from
/// when Ringmaster stored it.
async fn ingest_meeting(State(pool): State<PgPool>, Json(body): Json<IngestMeetingRequest>) -> Result<Response, (axum::http::StatusCode, String)> {
    if body.title.trim().is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, "title must not be blank".to_string()));
    }
    if body.transcript.trim().is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, "transcript must not be blank".to_string()));
    }
    let occurred_at = body
        .occurred_at
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or((axum::http::StatusCode::BAD_REQUEST, "occurred_at must not be blank".to_string()))?;
    let occurred_at = chrono::DateTime::parse_from_rfc3339(occurred_at)
        .map(|value| value.with_timezone(&chrono::Utc))
        .map_err(|_| (axum::http::StatusCode::BAD_REQUEST, "occurred_at must be a valid RFC3339 datetime".to_string()))?;

    let metadata = crate::transcript::MeetingMetadata {
        title: body.title,
        occurred_at: Some(occurred_at),
        organiser: body.organiser,
        participants: body.participants,
    };

    let ingested = crate::transcript::ingest_transcript(&pool, &metadata, &body.transcript)
        .await
        .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(json!({
            "meeting_id": ingested.meeting_id,
            "fragment_ids": ingested.fragment_ids,
        })),
    )
        .into_response())
}

#[derive(Debug, Deserialize)]
struct IngestSourceRequest {
    source_type: String,
    title: String,
    occurred_at: Option<String>,
    #[serde(default)]
    participants: Vec<String>,
    text: String,
}

/// Ingests one dated source of any kind atomically via the shared
/// `ingest_source` function (ADR-0040) -- the general-purpose sibling of
/// `POST /api/meetings/ingest`, for email/note/Teams-message/etc. text that
/// doesn't carry a transcript's speaker-turn shape. Never invokes a model,
/// extraction, or embedding.
async fn ingest_source_route(State(pool): State<PgPool>, Json(body): Json<IngestSourceRequest>) -> Result<Response, (axum::http::StatusCode, String)> {
    if body.source_type.trim().is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, "source_type must not be blank".to_string()));
    }
    if body.title.trim().is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, "title must not be blank".to_string()));
    }
    if body.text.trim().is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, "text must not be blank".to_string()));
    }
    let occurred_at = body
        .occurred_at
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or((axum::http::StatusCode::BAD_REQUEST, "occurred_at must not be blank".to_string()))?;
    let occurred_at = chrono::DateTime::parse_from_rfc3339(occurred_at)
        .map(|value| value.with_timezone(&chrono::Utc))
        .map_err(|_| (axum::http::StatusCode::BAD_REQUEST, "occurred_at must be a valid RFC3339 datetime".to_string()))?;

    let metadata = crate::transcript::SourceMetadata {
        source_type: body.source_type,
        title: body.title,
        occurred_at,
        participants: body.participants,
    };

    let ingested = crate::transcript::ingest_source(&pool, &metadata, &body.text)
        .await
        .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(json!({
            "node_id": ingested.node_id,
            "fragment_ids": ingested.fragment_ids,
        })),
    )
        .into_response())
}

/// Reads one meeting and its transcript fragments in turn order (ADR-0036).
/// 404s for an unknown id or a node that isn't a meeting -- this route's
/// contract is specifically a meeting, not any node type. Read-only.
async fn get_meeting_detail(State(pool): State<PgPool>, Path(id): Path<Uuid>) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let node = graph::get_node(&pool, id).await.map_err(|error| match error {
        sqlx::Error::RowNotFound => (axum::http::StatusCode::NOT_FOUND, "meeting not found".to_string()),
        other => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
    })?;
    if node.node_type != "meeting" {
        return Err((axum::http::StatusCode::NOT_FOUND, "meeting not found".to_string()));
    }

    let fragments = graph::list_source_fragments_by_meeting(&pool, id)
        .await
        .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    Ok(Json(json!({
        "id": node.id,
        "canonical_text": node.canonical_text,
        "attributes": node.attributes,
        "fragments": fragments.into_iter().map(|fragment| json!({
            "id": fragment.id,
            "text": fragment.text,
            "speaker": fragment.speaker,
            "sequence": fragment.sequence,
            "created_at": fragment.created_at.to_rfc3339(),
        })).collect::<Vec<_>>(),
    })))
}

/// Read model for `GET /api/meetings/:id/candidates` only (ADR-0037): one
/// row per (fragment, candidate) pair, left-joined so a fragment with no
/// candidate yet still appears once with every candidate column NULL.
#[derive(Debug, Clone, FromRow)]
struct MeetingFragmentCandidateRow {
    fragment_id: Uuid,
    sequence: Option<i32>,
    speaker: Option<String>,
    fragment_text: String,
    candidate_id: Option<Uuid>,
    candidate_type: Option<String>,
    statement: Option<String>,
    validation_state: Option<String>,
    confidence: Option<f32>,
}

/// Lists one meeting's fragments with their extracted candidates, if any,
/// plus fragment-level extraction progress (ADR-0037). 404s exactly like
/// `GET /api/meetings/:id` (ADR-0036): unknown id or a non-meeting node.
/// Read-only; triggers no extraction.
async fn get_meeting_candidates(State(pool): State<PgPool>, Path(id): Path<Uuid>) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let node = graph::get_node(&pool, id).await.map_err(|error| match error {
        sqlx::Error::RowNotFound => (axum::http::StatusCode::NOT_FOUND, "meeting not found".to_string()),
        other => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
    })?;
    if node.node_type != "meeting" {
        return Err((axum::http::StatusCode::NOT_FOUND, "meeting not found".to_string()));
    }

    let rows: Vec<MeetingFragmentCandidateRow> = sqlx::query_as(
        "SELECT sf.id AS fragment_id, sf.sequence, sf.speaker, sf.text AS fragment_text, \
                cp.candidate_id, cp.candidate_type, cp.statement, cp.validation_state, cp.confidence \
         FROM source_fragments sf \
         LEFT JOIN candidate_projection cp ON cp.source_fragment_id = sf.id \
         WHERE sf.source_id = $1 \
         ORDER BY sf.sequence ASC NULLS LAST, sf.created_at ASC, sf.id ASC, cp.candidate_id ASC",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    // Groups the flat join back into one entry per fragment, preserving
    // transcript order, without assuming exactly one candidate per fragment.
    let mut fragment_order: Vec<Uuid> = Vec::new();
    let mut fragment_data: std::collections::HashMap<Uuid, (Option<i32>, Option<String>, String, Vec<JsonValue>)> = std::collections::HashMap::new();
    let mut by_validation_state: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();

    for row in &rows {
        let entry = fragment_data.entry(row.fragment_id).or_insert_with(|| {
            fragment_order.push(row.fragment_id);
            (row.sequence, row.speaker.clone(), row.fragment_text.clone(), Vec::new())
        });
        if let Some(candidate_id) = row.candidate_id {
            entry.3.push(json!({
                "candidate_id": candidate_id,
                "candidate_type": row.candidate_type,
                "statement": row.statement,
                "validation_state": row.validation_state,
                "confidence": row.confidence,
            }));
            if let Some(state) = &row.validation_state {
                *by_validation_state.entry(state.clone()).or_insert(0) += 1;
            }
        }
    }

    let fragment_count = fragment_order.len() as i64;
    let extracted_fragment_count = fragment_order.iter().filter(|fragment_id| !fragment_data[fragment_id].3.is_empty()).count() as i64;

    let fragments: Vec<JsonValue> = fragment_order
        .iter()
        .map(|fragment_id| {
            let (sequence, speaker, text, candidates) = &fragment_data[fragment_id];
            json!({
                "fragment_id": fragment_id,
                "sequence": sequence,
                "speaker": speaker,
                "text": text,
                "candidates": candidates,
            })
        })
        .collect();

    Ok(Json(json!({
        "meeting_id": id,
        "fragments": fragments,
        "progress": {
            "fragment_count": fragment_count,
            "extracted_fragment_count": extracted_fragment_count,
            "pending_fragment_count": fragment_count - extracted_fragment_count,
            "by_validation_state": by_validation_state,
        },
    })))
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

    // ADR-0038: the state-change event and its audit row commit atomically --
    // a failure between the two can never leave the action un-audited.
    let action = if event_type == "accepted" { "candidate_accepted" } else { "candidate_rejected" };
    let mut tx = pool.begin().await.map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    extraction::transition_candidate(&mut *tx, id, event_type, json!({}))
        .await
        .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
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
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    tx.commit().await.map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

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

#[derive(Debug, Deserialize)]
struct CorrectCandidateRequest {
    candidate_type: Option<String>,
    statement: Option<String>,
}

/// Corrects a candidate still in the `candidate` state (ADR-0045): edits
/// `candidate_type` and/or `statement` before transitioning to `corrected`,
/// a distinct outcome from `accepted` (PRODUCT-SPEC.md §6.4) that still
/// promotes exactly like `accepted` does. At least one field must actually
/// change, or the request is rejected as a meaningless correction rather
/// than a silent no-op event.
async fn correct_candidate(
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
            format!("candidate is already \"{}\", not \"candidate\"", current.validation_state),
        ));
    }

    if let Some(candidate_type) = &body.candidate_type {
        if !extraction::ALLOWED_CANDIDATE_TYPES.contains(&candidate_type.as_str()) {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                format!("candidate_type must be one of {:?}, got {:?}", extraction::ALLOWED_CANDIDATE_TYPES, candidate_type),
            ));
        }
    }

    let type_changed = body.candidate_type.as_deref().is_some_and(|value| value != current.candidate_type);
    let statement_changed = body.statement.as_deref().is_some_and(|value| !value.trim().is_empty() && value != current.statement);
    if !type_changed && !statement_changed {
        return Err((axum::http::StatusCode::BAD_REQUEST, "a correction must actually change candidate_type or statement".to_string()));
    }

    let mut payload = serde_json::Map::new();
    if type_changed {
        payload.insert("candidate_type".to_string(), json!(body.candidate_type));
    }
    if statement_changed {
        payload.insert("statement".to_string(), json!(body.statement));
    }

    // ADR-0038: the correction event and its audit row commit atomically.
    let mut tx = pool.begin().await.map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    extraction::transition_candidate(&mut *tx, id, "corrected", JsonValue::Object(payload.clone()))
        .await
        .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
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
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    tx.commit().await.map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    extraction::rebuild_candidate_projection(&pool)
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

    if current.validation_state != "accepted" && current.validation_state != "corrected" {
        return Err((
            axum::http::StatusCode::CONFLICT,
            format!("candidate is \"{}\", not \"accepted\" or \"corrected\"", current.validation_state),
        ));
    }

    let obligation_id = Uuid::new_v4();
    // ADR-0058: a candidate extracted with a stated deadline seeds the new
    // Obligation's soft (advisory) due date; absent one, this is None and the
    // Obligation is dateless exactly as before.
    let due_at = extraction::candidate_extracted_due_at(&pool, id)
        .await
        .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let mut created_payload = serde_json::Map::new();
    created_payload.insert("status".to_string(), json!("open"));
    created_payload.insert("source_fragment_id".to_string(), json!(current.source_fragment_id));
    if let Some(due_at) = due_at {
        created_payload.insert("soft_due_at".to_string(), json!(due_at.to_rfc3339()));
    }
    // ADR-0038: the Obligation creation, the candidate's own promoted
    // transition, and the audit row all commit atomically.
    let mut tx = pool.begin().await.map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    obligation::append_event(
        &mut *tx,
        obligation_id,
        obligation::ObligationEventType::Created,
        JsonValue::Object(created_payload),
    )
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    extraction::transition_candidate(&mut *tx, id, "promoted", json!({"obligation_id": obligation_id}))
        .await
        .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

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
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    tx.commit().await.map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

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

    match extraction::extract_candidate_via_model(&pool, &config, fragment.id, &fragment.text, chrono::Utc::now()).await {
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
struct AuditEventsQuery {
    limit: Option<i64>,
}

/// A flat, reverse-chronological feed of recent audit rows (ADR-0049):
/// read-only, no correlation to any specific Obligation or candidate.
async fn list_audit_events(
    State(pool): State<PgPool>,
    Query(params): Query<AuditEventsQuery>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let rows = audit::recent(&pool, params.limit).await.map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
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

#[derive(Debug, Deserialize)]
struct NodeQuery {
    node_type: Option<String>,
    occurred_from: Option<String>,
    occurred_to: Option<String>,
    needs_attention: Option<bool>,
}

/// Parses an optional RFC3339 query param: absent/blank is `Ok(None)`, a
/// present-but-unparseable value is a typed `400` (ADR-0042).
fn parse_optional_rfc3339(label: &str, raw: Option<&str>) -> Result<Option<chrono::DateTime<chrono::Utc>>, (axum::http::StatusCode, String)> {
    match raw.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(None),
        Some(value) => chrono::DateTime::parse_from_rfc3339(value)
            .map(|parsed| Some(parsed.with_timezone(&chrono::Utc)))
            .map_err(|_| (axum::http::StatusCode::BAD_REQUEST, format!("{label} must be a valid RFC3339 datetime"))),
    }
}

/// Lists nodes, optionally filtered by `?node_type=` (ADR-0025), an
/// `occurred_at` range via `?occurred_from=`/`?occurred_to=` (ADR-0042),
/// and/or `?needs_attention=true` (ADR-0051), restricting to nodes with at
/// least one linked open/at-risk Obligation. Omitting all three preserves
/// this route's exact prior behavior. For `?node_type=person` specifically
/// (ADR-0051), each row is additionally enriched with `open_count`,
/// `at_risk_count`, and `last_interaction_at` -- two batched queries
/// keyed by the already-fetched ids/names, never one query per row.
async fn list_nodes_route(State(pool): State<PgPool>, Query(params): Query<NodeQuery>) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let occurred_from = parse_optional_rfc3339("occurred_from", params.occurred_from.as_deref())?;
    let occurred_to = parse_optional_rfc3339("occurred_to", params.occurred_to.as_deref())?;
    let nodes = graph::list_nodes(&pool, params.node_type.as_deref(), occurred_from, occurred_to, params.needs_attention.unwrap_or(false))
        .await
        .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    if params.node_type.as_deref() != Some("person") || nodes.is_empty() {
        return Ok(Json(json!(nodes)));
    }

    #[derive(Debug, FromRow)]
    struct PersonCountRow {
        node_id: Uuid,
        open_count: i64,
        at_risk_count: i64,
    }
    let ids: Vec<Uuid> = nodes.iter().map(|node| node.id).collect();
    let counts: Vec<PersonCountRow> = sqlx::query_as(
        "SELECT n.id AS node_id, \
                COUNT(*) FILTER (WHERE op.status = 'open') AS open_count, \
                COUNT(*) FILTER (WHERE op.status = 'at_risk') AS at_risk_count \
         FROM nodes n \
         JOIN edges e ON e.from_id = n.id OR e.to_id = n.id \
         JOIN obligation_projection op ON op.obligation_id = (CASE WHEN e.from_id = n.id THEN e.to_id ELSE e.from_id END) \
         WHERE n.id = ANY($1) AND op.status IN ('open', 'at_risk') \
         GROUP BY n.id",
    )
    .bind(&ids)
    .fetch_all(&pool)
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    #[derive(Debug, FromRow)]
    struct InteractionRow {
        speaker: String,
        last_interaction_at: Option<chrono::DateTime<chrono::Utc>>,
    }
    let names: Vec<String> = nodes.iter().map(|node| node.canonical_text.clone()).collect();
    let interactions: Vec<InteractionRow> = sqlx::query_as(
        "SELECT sf.speaker, MAX(n.occurred_at) AS last_interaction_at \
         FROM source_fragments sf JOIN nodes n ON n.id = sf.source_id \
         WHERE sf.speaker = ANY($1) \
         GROUP BY sf.speaker",
    )
    .bind(&names)
    .fetch_all(&pool)
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    let body: Vec<JsonValue> = nodes
        .into_iter()
        .map(|node| {
            let counts = counts.iter().find(|row| row.node_id == node.id);
            let last_interaction_at = interactions
                .iter()
                .find(|row| row.speaker == node.canonical_text)
                .and_then(|row| row.last_interaction_at)
                .map(|value| value.to_rfc3339());
            json!({
                "id": node.id,
                "node_type": node.node_type,
                "canonical_text": node.canonical_text,
                "attributes": node.attributes,
                "lifecycle_state": node.lifecycle_state,
                "occurred_at": node.occurred_at.map(|value| value.to_rfc3339()),
                "open_count": counts.map(|row| row.open_count).unwrap_or(0),
                "at_risk_count": counts.map(|row| row.at_risk_count).unwrap_or(0),
                "last_interaction_at": last_interaction_at,
            })
        })
        .collect();
    Ok(Json(json!(body)))
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
    valid_from: Option<chrono::DateTime<chrono::Utc>>,
    valid_to: Option<chrono::DateTime<chrono::Utc>>,
    neighbor_id: Option<Uuid>,
    neighbor_node_type: Option<String>,
    neighbor_canonical_text: Option<String>,
    obligation_id: Option<Uuid>,
    obligation_status: Option<String>,
    obligation_updated_at: Option<chrono::DateTime<chrono::Utc>>,
    obligation_hard_due_at: Option<chrono::DateTime<chrono::Utc>>,
    obligation_soft_due_at: Option<chrono::DateTime<chrono::Utc>>,
    obligation_source_fragment_id: Option<Uuid>,
    obligation_source_text: Option<String>,
    obligation_has_owner: Option<bool>,
    obligation_has_edges: Option<bool>,
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
        "SELECT e.id, e.from_id, e.to_id, e.edge_type, e.confidence, e.valid_from, e.valid_to, \
                n.id AS neighbor_id, n.node_type AS neighbor_node_type, n.canonical_text AS neighbor_canonical_text, \
                op.obligation_id AS obligation_id, op.status AS obligation_status, op.updated_at AS obligation_updated_at, \
                op.hard_due_at AS obligation_hard_due_at, op.soft_due_at AS obligation_soft_due_at, \
                op.source_fragment_id AS obligation_source_fragment_id, sf.text AS obligation_source_text, \
                EXISTS ( \
                    SELECT 1 FROM edges oe JOIN nodes on2 ON on2.id = oe.from_id \
                    WHERE oe.to_id = op.obligation_id AND oe.edge_type = 'owns' AND on2.node_type = 'person' \
                ) AS obligation_has_owner, \
                EXISTS ( \
                    SELECT 1 FROM edges oe WHERE oe.from_id = op.obligation_id OR oe.to_id = op.obligation_id \
                ) AS obligation_has_edges \
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
                "valid_from": row.valid_from.map(|value| value.to_rfc3339()),
                "valid_to": row.valid_to.map(|value| value.to_rfc3339()),
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
                "risk_signals": risk_signals(
                    row.obligation_hard_due_at,
                    row.obligation_soft_due_at,
                    row.obligation_updated_at.unwrap_or_else(chrono::Utc::now),
                    row.obligation_source_fragment_id,
                    row.obligation_has_owner.unwrap_or(false),
                    row.obligation_has_edges.unwrap_or(false),
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

    let last_interaction_at = if node.node_type == "person" {
        let row: (Option<chrono::DateTime<chrono::Utc>>,) = sqlx::query_as(
            "SELECT MAX(n.occurred_at) FROM source_fragments sf \
             JOIN nodes n ON n.id = sf.source_id \
             WHERE sf.speaker = $1",
        )
        .bind(&node.canonical_text)
        .fetch_one(&pool)
        .await
        .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
        row.0.map(|value| value.to_rfc3339())
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
        "last_interaction_at": last_interaction_at,
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
    #[serde(default)]
    valid_from: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    supersede: bool,
}

/// Creates one edge between any two entity ids (ADR-0025/ADR-0009). An
/// opt-in `supersede: true` closes out any prior current edge sharing this
/// `from_id`/`edge_type` (ADR-0032); omitted or false leaves every existing
/// caller's behavior unchanged.
async fn create_edge_route(State(pool): State<PgPool>, Json(body): Json<CreateEdgeRequest>) -> Result<Response, (axum::http::StatusCode, String)> {
    let id = graph::create_edge_with_options(&pool, body.from_id, body.to_id, &body.edge_type, body.confidence, body.valid_from, body.supersede)
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
        crate::guard_test_database(&database_url);
        PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await
            .expect("connect to test database")
    }

    /// ADR-0041: risk_signals is a pure function -- no database, no
    /// flakiness -- covering both signals independently and together.
    #[test]
    fn risk_signals_flags_date_compression_when_due_soon_with_no_evidence() {
        let due = chrono::Utc::now() + chrono::Duration::days(3);
        let signals = risk_signals(Some(due), None, chrono::Utc::now(), None, true, true);
        assert!(signals.iter().any(|signal| signal["signal"] == "date_compression"));
    }

    #[test]
    fn risk_signals_does_not_flag_date_compression_when_evidence_is_linked() {
        let due = chrono::Utc::now() + chrono::Duration::days(3);
        let signals = risk_signals(Some(due), None, chrono::Utc::now(), Some(uuid::Uuid::new_v4()), true, true);
        assert!(signals.iter().all(|signal| signal["signal"] != "date_compression"));
    }

    #[test]
    fn risk_signals_does_not_flag_date_compression_when_due_date_is_far_out() {
        let due = chrono::Utc::now() + chrono::Duration::days(30);
        let signals = risk_signals(Some(due), None, chrono::Utc::now(), None, true, true);
        assert!(signals.is_empty());
    }

    #[test]
    fn risk_signals_flags_stale_when_untouched_past_threshold() {
        let updated_at = chrono::Utc::now() - chrono::Duration::days(20);
        let signals = risk_signals(None, None, updated_at, Some(uuid::Uuid::new_v4()), true, true);
        assert!(signals.iter().any(|signal| signal["signal"] == "stale"));
    }

    #[test]
    fn risk_signals_does_not_flag_stale_within_threshold() {
        let updated_at = chrono::Utc::now() - chrono::Duration::days(2);
        let signals = risk_signals(None, None, updated_at, Some(uuid::Uuid::new_v4()), true, true);
        assert!(signals.is_empty());
    }

    #[test]
    fn risk_signals_can_flag_both_signals_at_once() {
        let due = chrono::Utc::now() - chrono::Duration::days(1);
        let updated_at = chrono::Utc::now() - chrono::Duration::days(20);
        let signals = risk_signals(Some(due), None, updated_at, None, true, true);
        assert_eq!(signals.len(), 2);
    }

    /// ADR-0046: unowned is independent of date_compression/stale --
    /// no due date, recently updated, evidence linked, just no owner.
    #[test]
    fn risk_signals_flags_unowned_when_has_owner_is_false() {
        let signals = risk_signals(None, None, chrono::Utc::now(), Some(uuid::Uuid::new_v4()), false, true);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0]["signal"], "unowned");
    }

    #[test]
    fn risk_signals_does_not_flag_unowned_when_has_owner_is_true() {
        let signals = risk_signals(None, None, chrono::Utc::now(), Some(uuid::Uuid::new_v4()), true, true);
        assert!(signals.is_empty());
    }

    /// ADR-0054 (Congruence Engine v1): isolated is independent of the
    /// other three signals -- no due date, recently updated, evidence
    /// linked, has an owner, just zero edges at all.
    #[test]
    fn risk_signals_flags_isolated_when_has_edges_is_false() {
        let signals = risk_signals(None, None, chrono::Utc::now(), Some(uuid::Uuid::new_v4()), true, false);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0]["signal"], "isolated");
    }

    #[test]
    fn risk_signals_does_not_flag_isolated_when_has_edges_is_true() {
        let signals = risk_signals(None, None, chrono::Utc::now(), Some(uuid::Uuid::new_v4()), true, true);
        assert!(signals.is_empty());
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

    /// ADR-0041: the Daily Brief attaches risk_signals to each row, computed
    /// from the same fields the reason string already uses.
    #[tokio::test]
    async fn daily_brief_route_attaches_risk_signals() {
        let pool = test_pool().await;

        let compressed = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            compressed,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open", "hard_due_at": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339()}),
        )
        .await
        .expect("append obligation due soon with no evidence");

        crate::obligation::rebuild_projection(&pool).await.expect("rebuild projection");

        let response = app(pool.clone())
            .oneshot(Request::builder().uri("/api/daily-brief").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let row = parsed
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["obligation_id"] == compressed.to_string())
            .expect("the just-created obligation must be present");
        let signals = row["risk_signals"].as_array().expect("risk_signals must be an array");
        assert!(signals.iter().any(|signal| signal["signal"] == "date_compression"));
    }

    /// ADR-0044: the Today page's plain-language title is this quote,
    /// falling back to an honest status label only when it's null.
    #[tokio::test]
    async fn daily_brief_route_includes_source_text_evidence() {
        let pool = test_pool().await;
        let fragment_id = graph::create_source_fragment(
            &pool,
            uuid::Uuid::new_v4(),
            "We committed to a two-week transition plan.",
            "daily-brief-source-text-test-hash",
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
            .oneshot(Request::builder().uri("/api/daily-brief").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let row = parsed
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["obligation_id"] == obligation_id.to_string())
            .expect("the just-created obligation must be present");
        assert_eq!(row["source_text"], "We committed to a two-week transition plan.");
    }

    /// ADR-0046: an Obligation with an `owns` edge from a person is never
    /// flagged unowned; one without any such edge is.
    #[tokio::test]
    async fn daily_brief_flags_an_obligation_with_no_owns_edge_as_unowned() {
        let pool = test_pool().await;

        let owned = uuid::Uuid::new_v4();
        crate::obligation::append_event(&pool, owned, crate::obligation::ObligationEventType::Created, json!({"status": "open"}))
            .await
            .expect("append owned obligation");
        let unowned = uuid::Uuid::new_v4();
        crate::obligation::append_event(&pool, unowned, crate::obligation::ObligationEventType::Created, json!({"status": "open"}))
            .await
            .expect("append unowned obligation");
        crate::obligation::rebuild_projection(&pool).await.expect("rebuild projection");

        let person_id = graph::create_node(&pool, "person", "Owner Signal Test Person", json!({})).await.expect("create person");
        graph::create_edge(&pool, person_id, owned, "owns", None).await.expect("link person as owner");

        let response = app(pool)
            .oneshot(Request::builder().uri("/api/daily-brief").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let rows = parsed.as_array().unwrap();

        let owned_row = rows.iter().find(|row| row["obligation_id"] == owned.to_string()).expect("present");
        let owned_signals = owned_row["risk_signals"].as_array().unwrap();
        assert!(owned_signals.iter().all(|signal| signal["signal"] != "unowned"), "an owned obligation must not be flagged unowned");

        let unowned_row = rows.iter().find(|row| row["obligation_id"] == unowned.to_string()).expect("present");
        let unowned_signals = unowned_row["risk_signals"].as_array().unwrap();
        assert!(unowned_signals.iter().any(|signal| signal["signal"] == "unowned"), "an obligation with no owns edge must be flagged unowned");
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

    /// ADR-0038: accepting writes an immutable audit row in the same
    /// transaction as the state change, with the honestly-labeled
    /// single-user placeholder actor -- never a fabricated identity.
    #[tokio::test]
    async fn accept_route_writes_an_audit_row_with_the_honest_placeholder_actor() {
        let pool = test_pool().await;
        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(&pool, candidate_id, "risk", "stated risk", uuid::Uuid::new_v4(), Some(0.7), None)
            .await
            .expect("extract candidate");
        extraction::rebuild_candidate_projection(&pool).await.expect("rebuild candidate projection");

        let (before,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_events WHERE action = 'candidate_accepted'")
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

        let (after,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_events WHERE action = 'candidate_accepted'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(after, before + 1, "exactly one audit row must be written for this acceptance");

        let (actor,): (String,) =
            sqlx::query_as("SELECT actor FROM audit_events WHERE action = 'candidate_accepted' ORDER BY recorded_at DESC LIMIT 1")
                .fetch_one(&pool)
                .await
                .expect("an audit row must exist for this acceptance");
        assert_eq!(actor, "local-operator", "actor must be the honest single-user placeholder, not a fabricated identity");
    }

    /// ADR-0038: rejecting writes an immutable audit row in the same
    /// transaction as the state change.
    #[tokio::test]
    async fn reject_route_writes_an_audit_row() {
        let pool = test_pool().await;
        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(&pool, candidate_id, "risk", "stated risk", uuid::Uuid::new_v4(), Some(0.7), None)
            .await
            .expect("extract candidate");
        extraction::rebuild_candidate_projection(&pool).await.expect("rebuild candidate projection");

        let (before,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_events WHERE action = 'candidate_rejected'")
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

        let (after,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_events WHERE action = 'candidate_rejected'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(after, before + 1, "exactly one audit row must be written for this rejection");
    }

    /// ADR-0045: correcting the statement alone transitions to `corrected`
    /// and applies exactly the changed field, leaving candidate_type as-is.
    #[tokio::test]
    async fn correct_route_changes_statement_and_transitions_to_corrected() {
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
                    .uri(format!("/api/candidates/{candidate_id}/correct"))
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"statement": "the actual, corrected risk statement"}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert_eq!(parsed.get("validation_state").and_then(|v| v.as_str()), Some("corrected"));
        assert_eq!(parsed.get("statement").and_then(|v| v.as_str()), Some("the actual, corrected risk statement"));
        assert_eq!(parsed.get("candidate_type").and_then(|v| v.as_str()), Some("risk"), "an unchanged field must not be altered");
    }

    /// ADR-0045: correcting candidate_type alone leaves statement as-is.
    #[tokio::test]
    async fn correct_route_changes_candidate_type_and_transitions_to_corrected() {
        let pool = test_pool().await;
        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(&pool, candidate_id, "risk", "actually a commitment", uuid::Uuid::new_v4(), Some(0.7), None)
            .await
            .expect("extract candidate");
        extraction::rebuild_candidate_projection(&pool).await.expect("rebuild candidate projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/correct"))
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"candidate_type": "commitment"}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert_eq!(parsed.get("validation_state").and_then(|v| v.as_str()), Some("corrected"));
        assert_eq!(parsed.get("candidate_type").and_then(|v| v.as_str()), Some("commitment"));
        assert_eq!(parsed.get("statement").and_then(|v| v.as_str()), Some("actually a commitment"), "an unchanged field must not be altered");
    }

    #[tokio::test]
    async fn correct_route_rejects_a_no_op_change() {
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
                    .uri(format!("/api/candidates/{candidate_id}/correct"))
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"candidate_type": "risk", "statement": "stated risk"}).to_string()))
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
        extraction::extract_candidate(&pool, candidate_id, "risk", "stated risk", uuid::Uuid::new_v4(), Some(0.7), None)
            .await
            .expect("extract candidate");
        extraction::rebuild_candidate_projection(&pool).await.expect("rebuild candidate projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/correct"))
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"candidate_type": "not-a-real-type"}).to_string()))
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
        extraction::extract_candidate(&pool, candidate_id, "risk", "stated risk", uuid::Uuid::new_v4(), Some(0.7), None)
            .await
            .expect("extract candidate");
        extraction::transition_candidate(&pool, candidate_id, "accepted", json!({})).await.expect("append accepted event");
        extraction::rebuild_candidate_projection(&pool).await.expect("rebuild candidate projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/correct"))
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"statement": "too late to correct"}).to_string()))
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
        extraction::extract_candidate(&pool, candidate_id, "risk", "stated risk", uuid::Uuid::new_v4(), Some(0.7), None)
            .await
            .expect("extract candidate");
        extraction::rebuild_candidate_projection(&pool).await.expect("rebuild candidate projection");

        let (before,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_events WHERE action = 'candidate_corrected'")
            .fetch_one(&pool)
            .await
            .unwrap();

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/candidates/{candidate_id}/correct"))
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"statement": "corrected via audited route"}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let (after,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_events WHERE action = 'candidate_corrected'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(after, before + 1, "exactly one audit row must be written for this correction");
    }

    /// ADR-0045: promotion accepts a corrected candidate exactly like an
    /// accepted one -- both mean a human has validated it.
    #[tokio::test]
    async fn promote_route_accepts_a_corrected_candidate() {
        let pool = test_pool().await;
        let fragment_id = graph::create_source_fragment(&pool, uuid::Uuid::new_v4(), "We will migrate the pipeline by Q3.", "correct-then-promote-hash")
            .await
            .expect("create source fragment");
        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(&pool, candidate_id, "risk", "migrate the pipeline", fragment_id, Some(0.9), None)
            .await
            .expect("extract candidate");
        extraction::transition_candidate(&pool, candidate_id, "corrected", json!({"candidate_type": "commitment"}))
            .await
            .expect("append corrected event");
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
    }

    /// ADR-0049: the audit feed surfaces a real, just-recorded correction --
    /// found by a unique marker, never an aggregate count.
    #[tokio::test]
    async fn audit_events_route_surfaces_a_real_correction() {
        let pool = test_pool().await;
        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(&pool, candidate_id, "risk", "stated risk", uuid::Uuid::new_v4(), Some(0.7), None)
            .await
            .expect("extract candidate");
        extraction::rebuild_candidate_projection(&pool).await.expect("rebuild candidate projection");

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
            .oneshot(Request::builder().uri("/api/audit-events?limit=200").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(audit_response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(audit_response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let rows = parsed.as_array().unwrap();
        assert!(
            rows.iter().any(|row| row["new_state"]["statement"] == marker),
            "the correction's audit row must be present in the feed"
        );
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

    /// ADR-0058: a candidate extracted with a due date carries that date into
    /// the promoted Obligation as its soft (advisory) due date, so Today can
    /// rank it by real urgency instead of "No due date recorded".
    #[tokio::test]
    async fn promote_carries_extracted_due_date_into_soft_due_at() {
        let pool = test_pool().await;
        let fragment_id = graph::create_source_fragment(&pool, uuid::Uuid::new_v4(), "Send the transition plan by Friday.", "due-date-carry-hash")
            .await
            .expect("create source fragment");
        let candidate_id = uuid::Uuid::new_v4();
        let due = chrono::DateTime::parse_from_rfc3339("2026-08-21T17:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        extraction::extract_candidate_with_due_at(&pool, candidate_id, "request", "send the transition plan", fragment_id, Some(0.8), None, Some(due))
            .await
            .expect("extract candidate with a due date");
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
        let got = created
            .get("soft_due_at")
            .and_then(|v| v.as_str())
            .expect("promoted obligation must carry the extracted due date as soft_due_at");
        let got = chrono::DateTime::parse_from_rfc3339(got).unwrap().with_timezone(&chrono::Utc);
        assert_eq!(got, due, "the soft due date must equal the candidate's extracted due_at");
        assert!(created.get("hard_due_at").map(|v| v.is_null()).unwrap_or(true), "a model-inferred date must not become a hard due date");
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

    /// ADR-0038: promoting writes an immutable audit row in the same
    /// transaction as the Obligation creation and candidate transition.
    #[tokio::test]
    async fn promote_route_writes_an_audit_row() {
        let pool = test_pool().await;
        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(&pool, candidate_id, "commitment", "will migrate the pipeline", uuid::Uuid::new_v4(), Some(0.8), None)
            .await
            .expect("extract candidate");
        extraction::transition_candidate(&pool, candidate_id, "accepted", json!({}))
            .await
            .expect("append accepted event");
        extraction::rebuild_candidate_projection(&pool).await.expect("rebuild candidate projection");

        let (before,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_events WHERE action = 'candidate_promoted'")
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

        let (after,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_events WHERE action = 'candidate_promoted'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(after, before + 1, "exactly one audit row must be written for this promotion");
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

    /// ADR-0042: occurred_at was write-only since ADR-0040 -- this proves
    /// it round-trips through the read routes and can filter/exclude by
    /// range, not just get silently dropped.
    #[tokio::test]
    async fn nodes_route_filters_by_occurred_at_range_and_rejects_an_unparseable_bound() {
        let pool = test_pool().await;
        let in_range = graph::create_node(&pool, "note", "In Range Note", json!({})).await.expect("create in-range node");
        let out_of_range = graph::create_node(&pool, "note", "Out Of Range Note", json!({})).await.expect("create out-of-range node");
        sqlx::query("UPDATE nodes SET occurred_at = $2 WHERE id = $1")
            .bind(in_range)
            .bind(chrono::Utc::now() - chrono::Duration::days(3))
            .execute(&pool)
            .await
            .expect("set in-range occurred_at");
        sqlx::query("UPDATE nodes SET occurred_at = $2 WHERE id = $1")
            .bind(out_of_range)
            .bind(chrono::Utc::now())
            .execute(&pool)
            .await
            .expect("set out-of-range occurred_at");

        let from = (chrono::Utc::now() - chrono::Duration::days(4)).to_rfc3339().replace('+', "%2B");
        let to = (chrono::Utc::now() - chrono::Duration::days(2)).to_rfc3339().replace('+', "%2B");
        let response = app(pool.clone())
            .oneshot(Request::builder().uri(format!("/api/nodes?occurred_from={from}&occurred_to={to}")).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let listed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let ids: Vec<&str> = listed.as_array().unwrap().iter().map(|row| row["id"].as_str().unwrap()).collect();
        assert!(ids.contains(&in_range.to_string().as_str()), "in-range node must be listed");
        assert!(!ids.contains(&out_of_range.to_string().as_str()), "out-of-range node must be excluded");
        assert!(listed.as_array().unwrap().iter().any(|row| !row["occurred_at"].is_null()), "occurred_at must round-trip through the response, not be dropped");

        let bad_response = app(pool)
            .oneshot(Request::builder().uri("/api/nodes?occurred_from=not-a-date").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(bad_response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    /// ADR-0051: a `?node_type=person` list response is enriched with
    /// `open_count`/`at_risk_count`/`last_interaction_at`; `?needs_attention=true`
    /// excludes a person with no linked open/at_risk Obligation.
    #[tokio::test]
    async fn nodes_route_person_list_is_enriched_and_needs_attention_filters() {
        let pool = test_pool().await;
        let owed_person = graph::create_node(&pool, "person", "Route Enrichment Test Owed", json!({})).await.expect("create owed person");
        let idle_person = graph::create_node(&pool, "person", "Route Enrichment Test Idle", json!({})).await.expect("create idle person");

        let obligation_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(&pool, obligation_id, crate::obligation::ObligationEventType::Created, json!({"status": "at_risk"}))
            .await
            .expect("append at_risk obligation");
        graph::create_edge(&pool, owed_person, obligation_id, "owns", None).await.expect("link owed person");
        crate::obligation::rebuild_projection(&pool).await.expect("rebuild projection");

        let response = app(pool.clone())
            .oneshot(Request::builder().uri("/api/nodes?node_type=person").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let listed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let owed_row = listed
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == owed_person.to_string())
            .expect("the owed person must be present");
        assert_eq!(owed_row["at_risk_count"], 1);
        assert_eq!(owed_row["open_count"], 0);
        let idle_row = listed.as_array().unwrap().iter().find(|row| row["id"] == idle_person.to_string()).expect("the idle person must be present");
        assert_eq!(idle_row["at_risk_count"], 0);
        assert_eq!(idle_row["open_count"], 0);

        let filtered_response = app(pool)
            .oneshot(Request::builder().uri("/api/nodes?node_type=person&needs_attention=true").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let filtered_body = axum::body::to_bytes(filtered_response.into_body(), usize::MAX).await.unwrap();
        let filtered: JsonValue = serde_json::from_slice(&filtered_body).expect("valid json body");
        let filtered_ids: Vec<&str> = filtered.as_array().unwrap().iter().map(|row| row["id"].as_str().unwrap()).collect();
        assert!(filtered_ids.contains(&owed_person.to_string().as_str()), "a person needing attention must be included");
        assert!(!filtered_ids.contains(&idle_person.to_string().as_str()), "a person needing nothing must be excluded");
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

    /// ADR-0051: each Obligation in a person's relationship grouping
    /// carries risk_signals, the same computation Daily Brief/Time Horizon
    /// already use -- an owned obligation must not be flagged unowned.
    #[tokio::test]
    async fn node_detail_relationship_obligations_include_risk_signals() {
        let pool = test_pool().await;
        let person_id = graph::create_node(&pool, "person", "Relationship Risk Signal Test Person", json!({})).await.expect("create person");

        let stale_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(&pool, stale_id, crate::obligation::ObligationEventType::Created, json!({"status": "open"}))
            .await
            .expect("append a stale obligation");
        graph::create_edge(&pool, person_id, stale_id, "owns", None).await.expect("link person to the stale obligation");
        crate::obligation::rebuild_projection(&pool).await.expect("rebuild projection");
        sqlx::query("UPDATE obligation_projection SET updated_at = now() - interval '30 days' WHERE obligation_id = $1")
            .bind(stale_id)
            .execute(&pool)
            .await
            .expect("backdate updated_at");

        let response = app(pool)
            .oneshot(Request::builder().uri(format!("/api/nodes/{person_id}")).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");

        let open_group = detail["relationship"]["open"].as_array().unwrap();
        let entry = open_group.iter().find(|entry| entry["obligation_id"] == stale_id.to_string()).expect("the stale obligation must be present");
        let signals = entry["risk_signals"].as_array().expect("risk_signals is an array");
        assert!(signals.iter().any(|signal| signal["signal"] == "stale"), "a backdated obligation must be flagged stale");
        assert!(signals.iter().all(|signal| signal["signal"] != "unowned"), "an owned obligation must not be flagged unowned");
    }

    /// ADR-0051: last_interaction_at is the most recent occurred_at among
    /// fragments whose speaker string-matches this person's canonical_text
    /// -- a best-effort name match, not a resolved identity edge.
    #[tokio::test]
    async fn node_detail_includes_last_interaction_at_from_matching_fragment_speaker() {
        let pool = test_pool().await;
        let person_name = "Last Interaction Test Person";
        let person_id = graph::create_node(&pool, "person", person_name, json!({})).await.expect("create person");

        let source_id = uuid::Uuid::new_v4();
        let occurred_at = chrono::Utc::now() - chrono::Duration::days(2);
        sqlx::query("INSERT INTO nodes (id, node_type, canonical_text, attributes, occurred_at) VALUES ($1, 'meeting', 'Interaction Source', '{}'::jsonb, $2)")
            .bind(source_id)
            .bind(occurred_at)
            .execute(&pool)
            .await
            .expect("create a dated source node");
        sqlx::query("INSERT INTO source_fragments (source_id, text, speaker, hash) VALUES ($1, 'hello', $2, 'last-interaction-test-hash')")
            .bind(source_id)
            .bind(person_name)
            .execute(&pool)
            .await
            .expect("create a fragment spoken by this person");

        let response = app(pool)
            .oneshot(Request::builder().uri(format!("/api/nodes/{person_id}")).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");

        let last_interaction_at = detail["last_interaction_at"].as_str().expect("last_interaction_at must be present");
        let parsed = chrono::DateTime::parse_from_rfc3339(last_interaction_at).expect("valid RFC3339");
        assert!((parsed.timestamp() - occurred_at.timestamp()).abs() < 2, "must reflect the matching fragment's source occurred_at");
    }

    /// ADR-0051: no matching fragment speaker means an honest null, never a guess.
    #[tokio::test]
    async fn node_detail_last_interaction_at_is_null_with_no_matching_fragment() {
        let pool = test_pool().await;
        let person_id = graph::create_node(&pool, "person", "No Interaction Test Person", json!({})).await.expect("create person");

        let response = app(pool)
            .oneshot(Request::builder().uri(format!("/api/nodes/{person_id}")).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert!(detail["last_interaction_at"].is_null());
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

    /// ADR-0032: supersede omitted (or false) leaves valid_from/valid_to
    /// NULL, unchanged from every pre-ADR-0032 caller's behavior.
    #[tokio::test]
    async fn edge_create_route_without_supersede_leaves_valid_from_and_valid_to_null() {
        let pool = test_pool().await;
        let from_id = graph::create_node(&pool, "person", "Supersede Default Test From", json!({})).await.expect("create from-node");
        let to_id = graph::create_node(&pool, "person", "Supersede Default Test To", json!({})).await.expect("create to-node");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/edges")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"from_id": from_id, "to_id": to_id, "edge_type": "lives_in"}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let created: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert_eq!(created["valid_from"], JsonValue::Null);
        assert_eq!(created["valid_to"], JsonValue::Null);
    }

    /// ADR-0032: supersede: true closes the prior current edge sharing the
    /// same (from_id, edge_type) but leaves a different edge_type on the
    /// same from_id untouched -- matching is (from_id, edge_type) only, not
    /// to_id, matching the LIVES_IN Barcelona -> Madrid example.
    #[tokio::test]
    async fn edge_create_route_with_supersede_closes_the_prior_current_edge_matching_from_id_and_edge_type() {
        let pool = test_pool().await;
        let user_id = graph::create_node(&pool, "person", "Supersede Test User", json!({})).await.expect("create user node");
        let barcelona_id = graph::create_node(&pool, "city", "Barcelona", json!({})).await.expect("create barcelona node");
        let madrid_id = graph::create_node(&pool, "city", "Madrid", json!({})).await.expect("create madrid node");
        let risk_id = graph::create_node(&pool, "risk", "Unrelated Risk", json!({})).await.expect("create risk node");

        let barcelona_edge_id = graph::create_edge(&pool, user_id, barcelona_id, "lives_in", None).await.expect("create initial lives_in edge");
        let flagged_edge_id = graph::create_edge(&pool, user_id, risk_id, "flagged", None).await.expect("create an unrelated edge_type on the same from_id");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/edges")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"from_id": user_id, "to_id": madrid_id, "edge_type": "lives_in", "supersede": true}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let created: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert!(created["valid_from"].is_string(), "new current edge must carry a valid_from");
        assert_eq!(created["valid_to"], JsonValue::Null, "the new edge is current");

        let superseded = graph::get_edge(&pool, barcelona_edge_id).await.expect("fetch the superseded edge");
        assert!(superseded.valid_to.is_some(), "the prior lives_in edge must be closed out");

        let untouched = graph::get_edge(&pool, flagged_edge_id).await.expect("fetch the unrelated edge_type");
        assert_eq!(untouched.valid_to, None, "a different edge_type on the same from_id must not be superseded");
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

    /// ADR-0047: the Obligation detail route returns the same fields Daily
    /// Brief returns, plus risk_signals and linked_nodes resolved from the
    /// existing edges table (an owns edge from a real person).
    #[tokio::test]
    async fn obligation_detail_route_returns_risk_signals_and_linked_nodes() {
        let pool = test_pool().await;

        let obligation_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            obligation_id,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open", "hard_due_at": (chrono::Utc::now() - chrono::Duration::days(2)).to_rfc3339()}),
        )
        .await
        .expect("append obligation");
        crate::obligation::rebuild_projection(&pool).await.expect("rebuild projection");

        let person_id = graph::create_node(&pool, "person", "Obligation Detail Test Person", json!({}))
            .await
            .expect("create person node");
        graph::create_edge(&pool, person_id, obligation_id, "owns", None).await.expect("link owner");
        let unresolved_id = uuid::Uuid::new_v4();
        graph::create_edge(&pool, obligation_id, unresolved_id, "blocks", None).await.expect("link an edge to a non-node id");

        let response = app(pool)
            .oneshot(Request::builder().uri(format!("/api/obligations/{obligation_id}")).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");

        assert_eq!(parsed["obligation_id"], obligation_id.to_string());
        let signals = parsed["risk_signals"].as_array().expect("risk_signals is an array");
        assert!(signals.iter().any(|signal| signal["signal"] == "date_compression"));
        assert!(!signals.iter().any(|signal| signal["signal"] == "unowned"), "an owned obligation must not be flagged unowned");

        let linked = parsed["linked_nodes"].as_array().expect("linked_nodes is an array");
        assert!(linked.iter().any(|node| node["node_id"] == person_id.to_string() && node["edge_type"] == "owns"));
        let unresolved_link = linked.iter().find(|node| node["edge_type"] == "blocks").expect("the edge to a non-node id must still appear");
        assert!(unresolved_link["node_id"].is_null(), "an edge into a non-node id must report a null neighbor, not error");
    }

    #[tokio::test]
    async fn obligation_detail_route_404s_for_an_unknown_id() {
        let pool = test_pool().await;
        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/obligations/{}", uuid::Uuid::new_v4()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    /// ADR-0041: the Time Horizon route attaches the same risk_signals field.
    #[tokio::test]
    async fn time_horizon_route_attaches_risk_signals() {
        let pool = test_pool().await;

        let overdue_no_evidence = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            overdue_no_evidence,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open", "hard_due_at": (chrono::Utc::now() - chrono::Duration::days(1)).to_rfc3339()}),
        )
        .await
        .expect("append overdue obligation with no evidence");

        crate::obligation::rebuild_projection(&pool).await.expect("rebuild projection");

        let response = app(pool.clone())
            .oneshot(Request::builder().uri("/api/time-horizon").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let row = parsed["overdue"]
            .as_array()
            .expect("overdue bucket must be present")
            .iter()
            .find(|row| row["obligation_id"] == overdue_no_evidence.to_string())
            .expect("the just-created obligation must be present in overdue");
        let signals = row["risk_signals"].as_array().expect("risk_signals must be an array");
        assert!(signals.iter().any(|signal| signal["signal"] == "date_compression"));
    }

    /// ADR-0046: mirrors the Daily Brief's own unowned proof, scoped to the
    /// Time Horizon's bucketed response shape.
    #[tokio::test]
    async fn time_horizon_flags_an_obligation_with_no_owns_edge_as_unowned() {
        let pool = test_pool().await;

        let owned = uuid::Uuid::new_v4();
        crate::obligation::append_event(&pool, owned, crate::obligation::ObligationEventType::Created, json!({"status": "open"}))
            .await
            .expect("append owned obligation");
        let unowned = uuid::Uuid::new_v4();
        crate::obligation::append_event(&pool, unowned, crate::obligation::ObligationEventType::Created, json!({"status": "open"}))
            .await
            .expect("append unowned obligation");
        crate::obligation::rebuild_projection(&pool).await.expect("rebuild projection");

        let person_id = graph::create_node(&pool, "person", "Time Horizon Owner Signal Test Person", json!({})).await.expect("create person");
        graph::create_edge(&pool, person_id, owned, "owns", None).await.expect("link person as owner");

        let response = app(pool)
            .oneshot(Request::builder().uri("/api/time-horizon").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let beyond = parsed["beyond"].as_array().expect("beyond bucket must be present -- no date recorded");

        let owned_row = beyond.iter().find(|row| row["obligation_id"] == owned.to_string()).expect("present");
        let owned_signals = owned_row["risk_signals"].as_array().unwrap();
        assert!(owned_signals.iter().all(|signal| signal["signal"] != "unowned"), "an owned obligation must not be flagged unowned");

        let unowned_row = beyond.iter().find(|row| row["obligation_id"] == unowned.to_string()).expect("present");
        let unowned_signals = unowned_row["risk_signals"].as_array().unwrap();
        assert!(unowned_signals.iter().any(|signal| signal["signal"] == "unowned"), "an obligation with no owns edge must be flagged unowned");
    }
    /// (reusing daily_brief_reason verbatim) are present.
    #[tokio::test]
    async fn focus_blocks_route_groups_by_shared_node() {
        let pool = test_pool().await;
        let person_id = graph::create_node(&pool, "person", "Roopa", json!({})).await.expect("create person node");
        let due_soon = (chrono::Utc::now() + chrono::Duration::days(3)).to_rfc3339();

        let obligation_a = uuid::Uuid::new_v4();
        crate::obligation::append_event(&pool, obligation_a, crate::obligation::ObligationEventType::Created, json!({"status": "open", "hard_due_at": due_soon}))
            .await
            .expect("append obligation a");
        let obligation_b = uuid::Uuid::new_v4();
        crate::obligation::append_event(&pool, obligation_b, crate::obligation::ObligationEventType::Created, json!({"status": "at_risk", "hard_due_at": due_soon}))
            .await
            .expect("append obligation b");
        crate::obligation::rebuild_projection(&pool).await.expect("rebuild projection");

        graph::create_edge(&pool, person_id, obligation_a, "owns", None).await.expect("link obligation a");
        graph::create_edge(&pool, obligation_b, person_id, "owns", None).await.expect("link obligation b");

        let response = app(pool)
            .oneshot(Request::builder().uri("/api/focus-blocks").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let blocks = parsed.as_array().expect("response is a json array");
        let block = blocks
            .iter()
            .find(|block| block["node_id"] == person_id.to_string())
            .expect("a block for the shared person node and shared bucket must exist");
        assert_eq!(block["node_type"], "person");
        assert_eq!(block["canonical_text"], "Roopa");
        assert_eq!(block["time_horizon_bucket"], "next_7_days");
        let obligations = block["obligations"].as_array().expect("obligations is an array");
        assert_eq!(obligations.len(), 2);
        assert!(obligations.iter().any(|o| o["obligation_id"] == obligation_a.to_string()));
        assert!(obligations.iter().any(|o| o["obligation_id"] == obligation_b.to_string() && o["reason"] == "Marked at risk. No evidence recorded."));
    }

    /// ADR-0052: a shared node whose Obligations span two Time Horizon
    /// buckets forms one block per bucket, not one block spanning both.
    #[tokio::test]
    async fn focus_blocks_route_splits_by_time_horizon_bucket() {
        let pool = test_pool().await;
        let person_id = graph::create_node(&pool, "person", "Bucket Split Test Person", json!({})).await.expect("create person node");
        let due_soon = (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339();
        let due_later = (chrono::Utc::now() + chrono::Duration::days(60)).to_rfc3339();

        let mut soon_ids = Vec::new();
        for _ in 0..2 {
            let id = uuid::Uuid::new_v4();
            crate::obligation::append_event(&pool, id, crate::obligation::ObligationEventType::Created, json!({"status": "open", "hard_due_at": due_soon}))
                .await
                .expect("append due-soon obligation");
            graph::create_edge(&pool, person_id, id, "owns", None).await.expect("link due-soon obligation");
            soon_ids.push(id);
        }
        let mut later_ids = Vec::new();
        for _ in 0..2 {
            let id = uuid::Uuid::new_v4();
            crate::obligation::append_event(&pool, id, crate::obligation::ObligationEventType::Created, json!({"status": "open", "hard_due_at": due_later}))
                .await
                .expect("append due-later obligation");
            graph::create_edge(&pool, person_id, id, "owns", None).await.expect("link due-later obligation");
            later_ids.push(id);
        }
        crate::obligation::rebuild_projection(&pool).await.expect("rebuild projection");

        let response = app(pool)
            .oneshot(Request::builder().uri("/api/focus-blocks").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let blocks: Vec<&JsonValue> = parsed.as_array().expect("response is a json array").iter().filter(|block| block["node_id"] == person_id.to_string()).collect();
        assert_eq!(blocks.len(), 2, "the shared node's Obligations must form two blocks, one per bucket");

        let soon_block = blocks.iter().find(|block| block["time_horizon_bucket"] == "next_7_days").expect("a next_7_days block must exist");
        let soon_obligations = soon_block["obligations"].as_array().unwrap();
        assert_eq!(soon_obligations.len(), 2);
        assert!(soon_ids.iter().all(|id| soon_obligations.iter().any(|o| o["obligation_id"] == id.to_string())));

        let later_block = blocks.iter().find(|block| block["time_horizon_bucket"] == "next_90_days").expect("a next_90_days block must exist");
        let later_obligations = later_block["obligations"].as_array().unwrap();
        assert_eq!(later_obligations.len(), 2);
        assert!(later_ids.iter().all(|id| later_obligations.iter().any(|o| o["obligation_id"] == id.to_string())));
    }

    #[tokio::test]
    async fn focus_blocks_route_forms_no_block_for_a_single_linked_obligation() {
        let pool = test_pool().await;
        let meeting_id = graph::create_node(&pool, "meeting", "Weekly sync", json!({})).await.expect("create meeting node");

        let obligation_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(&pool, obligation_id, crate::obligation::ObligationEventType::Created, json!({"status": "open"}))
            .await
            .expect("append obligation");
        crate::obligation::rebuild_projection(&pool).await.expect("rebuild projection");
        graph::create_edge(&pool, meeting_id, obligation_id, "discussed", None).await.expect("link obligation");

        let response = app(pool)
            .oneshot(Request::builder().uri("/api/focus-blocks").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let blocks = parsed.as_array().expect("response is a json array");
        assert!(
            blocks.iter().all(|block| block["node_id"] != meeting_id.to_string()),
            "a node linked to only one non-closed obligation must form no block"
        );
    }

    #[tokio::test]
    async fn focus_blocks_route_excludes_a_closed_obligation() {
        let pool = test_pool().await;
        let person_id = graph::create_node(&pool, "person", "Closed Test Person", json!({})).await.expect("create person node");

        let open_a = uuid::Uuid::new_v4();
        crate::obligation::append_event(&pool, open_a, crate::obligation::ObligationEventType::Created, json!({"status": "open"}))
            .await
            .expect("append open obligation a");
        let open_b = uuid::Uuid::new_v4();
        crate::obligation::append_event(&pool, open_b, crate::obligation::ObligationEventType::Created, json!({"status": "open"}))
            .await
            .expect("append open obligation b");
        let closed_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(&pool, closed_id, crate::obligation::ObligationEventType::Created, json!({"status": "open"}))
            .await
            .expect("append obligation to close");
        crate::obligation::append_event(&pool, closed_id, crate::obligation::ObligationEventType::Closed, json!({}))
            .await
            .expect("close it");
        crate::obligation::rebuild_projection(&pool).await.expect("rebuild projection");

        graph::create_edge(&pool, person_id, open_a, "owns", None).await.expect("link open obligation a");
        graph::create_edge(&pool, person_id, open_b, "owns", None).await.expect("link open obligation b");
        graph::create_edge(&pool, person_id, closed_id, "owns", None).await.expect("link closed obligation");

        let response = app(pool)
            .oneshot(Request::builder().uri("/api/focus-blocks").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let blocks = parsed.as_array().expect("response is a json array");
        let block = blocks
            .iter()
            .find(|block| block["node_id"] == person_id.to_string())
            .expect("a block must still form from the two open obligations");
        let obligations = block["obligations"].as_array().expect("obligations is an array");
        assert_eq!(obligations.len(), 2, "the closed obligation must not be counted");
        assert!(obligations.iter().all(|o| o["obligation_id"] != closed_id.to_string()), "the closed obligation must never appear in a block");
    }

    /// ADR-0034: a successful request creates the Meeting node and every
    /// fragment in transcript turn order, and returns them in that order.
    #[tokio::test]
    async fn ingest_meeting_route_creates_a_meeting_and_ordered_fragments() {
        let pool = test_pool().await;
        let request_body = json!({
            "title": "Weekly 1:1",
            "occurred_at": "2026-08-14T00:00:00Z",
            "organiser": "Lyndon",
            "participants": ["Lyndon", "Roopa Venkat"],
            "transcript": "Roopa: Please bring me a transition plan.\nLyndon: I will have it by Friday.",
        });

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/meetings/ingest")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let meeting_id = parsed["meeting_id"].as_str().expect("meeting_id present").to_string();
        let fragment_ids = parsed["fragment_ids"].as_array().expect("fragment_ids is an array");
        assert_eq!(fragment_ids.len(), 2, "one fragment per transcript turn");

        let node = graph::get_node(&pool, uuid::Uuid::parse_str(&meeting_id).unwrap()).await.expect("meeting node exists");
        assert_eq!(node.node_type, "meeting");
        assert_eq!(node.canonical_text, "Weekly 1:1");

        let first_fragment_id = uuid::Uuid::parse_str(fragment_ids[0].as_str().unwrap()).unwrap();
        let first_fragment = graph::get_source_fragment(&pool, first_fragment_id).await.expect("first fragment exists");
        assert_eq!(first_fragment.text, "Please bring me a transition plan.");

        let second_fragment_id = uuid::Uuid::parse_str(fragment_ids[1].as_str().unwrap()).unwrap();
        let second_fragment = graph::get_source_fragment(&pool, second_fragment_id).await.expect("second fragment exists");
        assert_eq!(second_fragment.text, "I will have it by Friday.");
    }

    #[tokio::test]
    async fn ingest_meeting_route_rejects_a_blank_title_with_no_writes() {
        let pool = test_pool().await;
        let marker = format!("blank-title-marker-{}", uuid::Uuid::new_v4());
        let request_body = json!({"title": "   ", "transcript": format!("Roopa: {marker}")});
        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/meetings/ingest")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM source_fragments WHERE text = $1")
            .bind(&marker)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "a blank title must perform zero writes, including zero fragments");
    }

    #[tokio::test]
    async fn ingest_meeting_route_rejects_a_blank_transcript_with_no_writes() {
        let pool = test_pool().await;
        let marker = format!("Blank Transcript Marker {}", uuid::Uuid::new_v4());
        let request_body = json!({"title": marker, "transcript": "   "});
        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/meetings/ingest")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM nodes WHERE node_type = 'meeting' AND canonical_text = $1")
            .bind(&marker)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "a blank transcript must perform zero writes, including zero Meeting nodes");
    }

    /// ADR-0040: occurred_at is the one new hard requirement; missing or
    /// blank must reject with 400 and perform zero writes, matching the
    /// title/transcript blank-check posture exactly.
    #[tokio::test]
    async fn ingest_meeting_route_rejects_a_missing_occurred_at_with_no_writes() {
        let pool = test_pool().await;
        let marker = format!("missing-occurred-at-marker-{}", uuid::Uuid::new_v4());
        let request_body = json!({"title": "Missing Occurred At Test", "transcript": format!("Roopa: {marker}")});
        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/meetings/ingest")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM source_fragments WHERE text = $1")
            .bind(&marker)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "a missing occurred_at must perform zero writes, including zero fragments");
    }

    /// ADR-0040: proves the general-purpose route creates a node of the
    /// given source_type with the paragraph-based split (not the meeting
    /// speaker-turn split), and stores occurred_at as a real column.
    #[tokio::test]
    async fn ingest_source_route_creates_a_node_and_ordered_paragraph_fragments() {
        let pool = test_pool().await;
        let request_body = json!({
            "source_type": "email",
            "title": "Re: transition plan",
            "occurred_at": "2026-08-01T09:00:00Z",
            "participants": ["Roopa"],
            "text": "First paragraph of the email.\n\nSecond paragraph.",
        });

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sources/ingest")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let node_id = parsed["node_id"].as_str().expect("node_id present").to_string();
        let fragment_ids = parsed["fragment_ids"].as_array().expect("fragment_ids is an array");
        assert_eq!(fragment_ids.len(), 2, "one fragment per paragraph");

        let node = graph::get_node(&pool, uuid::Uuid::parse_str(&node_id).unwrap()).await.expect("node exists");
        assert_eq!(node.node_type, "email");
        assert_eq!(node.canonical_text, "Re: transition plan");

        let (stored_occurred_at,): (Option<chrono::DateTime<chrono::Utc>>,) =
            sqlx::query_as("SELECT occurred_at FROM nodes WHERE id = $1").bind(uuid::Uuid::parse_str(&node_id).unwrap()).fetch_one(&pool).await.unwrap();
        assert_eq!(stored_occurred_at, Some(chrono::DateTime::parse_from_rfc3339("2026-08-01T09:00:00Z").unwrap().with_timezone(&chrono::Utc)));
    }

    #[tokio::test]
    async fn ingest_source_route_rejects_a_missing_occurred_at_with_no_writes() {
        let pool = test_pool().await;
        let marker = format!("source-missing-occurred-at-{}", uuid::Uuid::new_v4());
        let request_body = json!({"source_type": "note", "title": "Missing Occurred At Test", "text": marker});
        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sources/ingest")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM source_fragments WHERE text = $1")
            .bind(&marker)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "a missing occurred_at must perform zero writes, including zero fragments");
    }

    /// ADR-0040: this route must never trigger extraction or embedding
    /// implicitly, matching every other ingestion surface's posture.
    #[tokio::test]
    async fn ingest_source_route_never_creates_a_candidate_implicitly() {
        let pool = test_pool().await;
        let marker = format!("source-no-implicit-extraction-{}", uuid::Uuid::new_v4());
        let request_body = json!({"source_type": "note", "title": "No Implicit Extraction", "occurred_at": "2026-08-01T09:00:00Z", "text": marker});
        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sources/ingest")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::CREATED);

        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM candidate_projection cp \
             JOIN source_fragments sf ON sf.id = cp.source_fragment_id \
             WHERE sf.text = $1",
        )
        .bind(&marker)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 0, "ingestion must never create a candidate on its own");
    }

    /// ADR-0034: ingestion never invokes extraction, even implicitly --
    /// the new fragments never gain a candidate unless something separately
    /// calls the existing explicit extraction route.
    #[tokio::test]
    async fn ingest_meeting_route_never_creates_a_candidate_implicitly() {
        let pool = test_pool().await;
        let marker = format!("no-implicit-extraction-marker-{}", uuid::Uuid::new_v4());
        let request_body = json!({"title": "No Implicit Extraction Test", "occurred_at": "2026-08-14T00:00:00Z", "transcript": format!("Roopa: {marker}")});
        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/meetings/ingest")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::CREATED);

        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM candidate_projection cp \
             JOIN source_fragments sf ON sf.id = cp.source_fragment_id \
             WHERE sf.text = $1",
        )
        .bind(&marker)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 0, "ingestion must never create a candidate on its own");
    }

    /// ADR-0036: reads a meeting back with its fragments in transcript turn
    /// order, proving the new `sequence` column (not `created_at` alone,
    /// which can tie within one ingestion transaction) drives the order.
    #[tokio::test]
    async fn get_meeting_detail_route_returns_meeting_and_ordered_fragments() {
        let pool = test_pool().await;
        let request_body = json!({
            "title": "Ordered Fragments Test",
            "occurred_at": "2026-08-14T00:00:00Z",
            "transcript": "Roopa: first turn.\nLyndon: second turn.\nRoopa: third turn.",
        });
        let ingest_response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/meetings/ingest")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ingest_response.status(), axum::http::StatusCode::CREATED);
        let ingest_body = axum::body::to_bytes(ingest_response.into_body(), usize::MAX).await.unwrap();
        let ingested: JsonValue = serde_json::from_slice(&ingest_body).expect("valid json body");
        let meeting_id = ingested["meeting_id"].as_str().expect("meeting_id is a string");

        let response = app(pool)
            .oneshot(Request::builder().uri(format!("/api/meetings/{meeting_id}")).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");

        assert_eq!(parsed["canonical_text"], "Ordered Fragments Test");
        let fragments = parsed["fragments"].as_array().expect("fragments is an array");
        assert_eq!(fragments.len(), 3);
        assert_eq!(fragments[0]["text"], "first turn.");
        assert_eq!(fragments[1]["text"], "second turn.");
        assert_eq!(fragments[2]["text"], "third turn.");
        assert_eq!(fragments[0]["sequence"], 0);
        assert_eq!(fragments[1]["sequence"], 1);
        assert_eq!(fragments[2]["sequence"], 2);
    }

    /// ADR-0036: this route's contract is specifically a meeting, so an
    /// unknown id and an existing-but-wrong-type node both 404.
    #[tokio::test]
    async fn meeting_detail_route_404s_for_a_non_meeting_node() {
        let pool = test_pool().await;

        let unknown_id = uuid::Uuid::new_v4();
        let unknown_response = app(pool.clone())
            .oneshot(Request::builder().uri(format!("/api/meetings/{unknown_id}")).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(unknown_response.status(), axum::http::StatusCode::NOT_FOUND);

        let person_id = graph::create_node(&pool, "person", "Not A Meeting", json!({})).await.expect("create person node");
        let wrong_type_response = app(pool)
            .oneshot(Request::builder().uri(format!("/api/meetings/{person_id}")).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(wrong_type_response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    /// ADR-0037: one fragment has an extracted (still-unreviewed) candidate,
    /// the other has none -- proves fragments with zero candidates still
    /// appear (not silently omitted) and that fragment-level progress counts
    /// extracted/pending fragments, not raw candidate rows.
    #[tokio::test]
    async fn meeting_candidates_route_lists_extracted_and_pending_fragments() {
        let pool = test_pool().await;
        let ingested = crate::transcript::ingest_transcript(
            &pool,
            &crate::transcript::MeetingMetadata { title: "Candidates Progress Test".to_string(), occurred_at: None, organiser: None, participants: vec![] },
            "Roopa: please send me a transition plan.\nLyndon: sure, by Friday.",
        )
        .await
        .expect("ingest transcript");
        assert_eq!(ingested.fragment_ids.len(), 2);

        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(&pool, candidate_id, "request", "send a transition plan", ingested.fragment_ids[0], Some(0.8), None)
            .await
            .expect("extract candidate");
        extraction::rebuild_candidate_projection(&pool).await.expect("rebuild candidate projection");

        let response = app(pool)
            .oneshot(Request::builder().uri(format!("/api/meetings/{}/candidates", ingested.meeting_id)).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");

        let fragments = parsed["fragments"].as_array().expect("fragments is an array");
        assert_eq!(fragments.len(), 2);
        let extracted_fragment = fragments[0]["candidates"].as_array().expect("candidates is an array");
        assert_eq!(extracted_fragment.len(), 1);
        assert_eq!(extracted_fragment[0]["candidate_id"], candidate_id.to_string());
        assert_eq!(extracted_fragment[0]["validation_state"], "candidate");
        let pending_fragment = fragments[1]["candidates"].as_array().expect("candidates is an array");
        assert_eq!(pending_fragment.len(), 0, "a fragment with no candidate yet must still appear, with an empty array");

        assert_eq!(parsed["progress"]["fragment_count"], 2);
        assert_eq!(parsed["progress"]["extracted_fragment_count"], 1);
        assert_eq!(parsed["progress"]["pending_fragment_count"], 1);
        assert_eq!(parsed["progress"]["by_validation_state"]["candidate"], 1);
    }

    /// ADR-0037: mirrors ADR-0036's own 404 contract for this sibling route.
    #[tokio::test]
    async fn meeting_candidates_route_404s_for_a_non_meeting_node() {
        let pool = test_pool().await;

        let unknown_id = uuid::Uuid::new_v4();
        let unknown_response = app(pool.clone())
            .oneshot(Request::builder().uri(format!("/api/meetings/{unknown_id}/candidates")).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(unknown_response.status(), axum::http::StatusCode::NOT_FOUND);

        let person_id = graph::create_node(&pool, "person", "Not A Meeting Either", json!({})).await.expect("create person node");
        let wrong_type_response = app(pool)
            .oneshot(Request::builder().uri(format!("/api/meetings/{person_id}/candidates")).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(wrong_type_response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    /// ADR-0037: this route only reads; calling it must never itself create
    /// a candidate for a never-extracted fragment.
    #[tokio::test]
    async fn meeting_candidates_route_never_triggers_extraction() {
        let pool = test_pool().await;
        let ingested = crate::transcript::ingest_transcript(
            &pool,
            &crate::transcript::MeetingMetadata { title: "No Extraction Test".to_string(), occurred_at: None, organiser: None, participants: vec![] },
            "Roopa: an unextracted turn.",
        )
        .await
        .expect("ingest transcript");

        let response = app(pool)
            .oneshot(Request::builder().uri(format!("/api/meetings/{}/candidates", ingested.meeting_id)).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");

        assert_eq!(parsed["progress"]["extracted_fragment_count"], 0);
        assert_eq!(parsed["progress"]["pending_fragment_count"], 1);
        assert_eq!(parsed["fragments"][0]["candidates"].as_array().unwrap().len(), 0);
    }
}
