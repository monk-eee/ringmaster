use super::obligations::{daily_brief_reason, risk_signals};
use super::MAX_LIST_LIMIT;
use crate::graph;
use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value as JsonValue};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub(super) struct NodeQuery {
    node_type: Option<String>,
    occurred_from: Option<String>,
    occurred_to: Option<String>,
    needs_attention: Option<bool>,
    has_source_fragments: Option<bool>,
    limit: Option<i64>,
    offset: Option<i64>,
}

/// Parses an optional RFC3339 query param: absent/blank is `Ok(None)`, a
/// present-but-unparseable value is a typed `400` (ADR-0042).
fn parse_optional_rfc3339(
    label: &str,
    raw: Option<&str>,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, (axum::http::StatusCode, String)> {
    match raw.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(None),
        Some(value) => chrono::DateTime::parse_from_rfc3339(value)
            .map(|parsed| Some(parsed.with_timezone(&chrono::Utc)))
            .map_err(|_| {
                (
                    axum::http::StatusCode::BAD_REQUEST,
                    format!("{label} must be a valid RFC3339 datetime"),
                )
            }),
    }
}

/// Lists nodes, optionally filtered by `?node_type=` (ADR-0025), an
/// `occurred_at` range via `?occurred_from=`/`?occurred_to=` (ADR-0042),
/// and/or `?needs_attention=true` (ADR-0051), restricting to nodes with at
/// least one linked open/at-risk Obligation, and/or `?has_source_fragments=true`
/// (ADR-0096), restricting to nodes with at least one ingested source
/// fragment -- the type-agnostic way to list "real ingested sources"
/// without naming any specific `node_type` string. `?limit=`/`?offset=`
/// (ADR-0059) page the result; omitting every param preserves this route's
/// exact prior behavior. For `?node_type=person` specifically (ADR-0051), each row is
/// additionally enriched with `open_count`, `at_risk_count`, and
/// `last_interaction_at` -- three batched queries keyed by the already-fetched
/// ids/names, never one query per row. `last_interaction_at` (ADR-0070) is the
/// max of a `participated_in` edge-backed date and the legacy speaker-string
/// match, so neither newly-linked nor unbackfilled historical evidence is lost.
pub(super) async fn list_nodes_route(
    State(pool): State<PgPool>,
    Query(params): Query<NodeQuery>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let occurred_from = parse_optional_rfc3339("occurred_from", params.occurred_from.as_deref())?;
    let occurred_to = parse_optional_rfc3339("occurred_to", params.occurred_to.as_deref())?;
    let limit = params.limit.map(|value| value.clamp(1, MAX_LIST_LIMIT));
    let offset = params.offset.map(|value| value.max(0));
    let nodes = graph::list_nodes_filtered(
        &pool,
        params.node_type.as_deref(),
        None,
        occurred_from,
        occurred_to,
        params.needs_attention.unwrap_or(false),
        params.has_source_fragments.unwrap_or(false),
        limit,
        offset,
    )
    .await
    .map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;

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
        person_id: Uuid,
        last_interaction_at: Option<chrono::DateTime<chrono::Utc>>,
    }

    // ADR-0070: one batched query combines authoritative participated_in
    // edges with the legacy speaker fallback for every listed Person id.
    let interactions: Vec<InteractionRow> = sqlx::query_as(
        "SELECT evidence.person_id, MAX(evidence.occurred_at) AS last_interaction_at \
         FROM ( \
             SELECT e.from_id AS person_id, source.occurred_at \
             FROM edges e JOIN nodes source ON source.id = e.to_id \
             WHERE e.from_id = ANY($1) AND e.edge_type = 'participated_in' \
             UNION ALL \
             SELECT person.id AS person_id, source.occurred_at \
             FROM nodes person \
             JOIN source_fragments sf ON sf.speaker = person.canonical_text \
             JOIN nodes source ON source.id = sf.source_id \
             WHERE person.id = ANY($1) \
         ) evidence \
         GROUP BY evidence.person_id",
    )
    .bind(&ids)
    .fetch_all(&pool)
    .await
    .map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;

    let body: Vec<JsonValue> = nodes
        .into_iter()
        .map(|node| {
            let counts = counts.iter().find(|row| row.node_id == node.id);
            let last_interaction_at = interactions
                .iter()
                .find(|row| row.person_id == node.id)
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
pub(super) struct CreateNodeRequest {
    node_type: String,
    canonical_text: String,
    attributes: Option<JsonValue>,
}

/// Creates one node (ADR-0025), the graph substrate's first write route.
pub(super) async fn create_node_route(
    State(pool): State<PgPool>,
    Json(body): Json<CreateNodeRequest>,
) -> Result<Response, (axum::http::StatusCode, String)> {
    let id = graph::create_node(
        &pool,
        &body.node_type,
        &body.canonical_text,
        body.attributes.unwrap_or_else(|| json!({})),
    )
    .await
    .map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;
    let node = graph::get_node(&pool, id).await.map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;
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
pub(super) async fn get_node_detail(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let node = graph::get_node(&pool, id)
        .await
        .map_err(|error| match error {
            sqlx::Error::RowNotFound => (
                axum::http::StatusCode::NOT_FOUND,
                "node not found".to_string(),
            ),
            other => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                other.to_string(),
            ),
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
        linked.sort_by_key(|row| {
            (
                due_date_sort_key(row.obligation_hard_due_at),
                due_date_sort_key(row.obligation_soft_due_at),
            )
        });

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
        // ADR-0070: max of participated_in edge evidence (covers every source
        // type) and the legacy speaker-string match (pre-ADR-0069 sources
        // that were never backfilled), so neither path can hide the other.
        let row: (Option<chrono::DateTime<chrono::Utc>>,) = sqlx::query_as(
            "SELECT GREATEST(edge_path.last_interaction_at, legacy_path.last_interaction_at) \
             FROM ( \
                 SELECT MAX(source.occurred_at) AS last_interaction_at \
                 FROM edges e JOIN nodes source ON source.id = e.to_id \
                 WHERE e.from_id = $1 AND e.edge_type = 'participated_in' \
             ) edge_path, ( \
                 SELECT MAX(n.occurred_at) AS last_interaction_at \
                 FROM source_fragments sf JOIN nodes n ON n.id = sf.source_id \
                 WHERE sf.speaker = $2 \
             ) legacy_path",
        )
        .bind(node.id)
        .bind(&node.canonical_text)
        .fetch_one(&pool)
        .await
        .map_err(|error| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                error.to_string(),
            )
        })?;
        row.0.map(|value| value.to_rfc3339())
    } else {
        None
    };

    // ADR-0071: a bounded, source-cited Past section. One evidence union
    // covers both participated_in and legacy speaker paths, deduplicated by
    // source id with participated_in taking precedence, newest-first, capped
    // at 10 with an honest total -- non-person nodes get an empty collection.
    #[derive(Debug, FromRow)]
    struct RecentInteractionRow {
        source_id: Uuid,
        source_type: String,
        title: String,
        occurred_at: chrono::DateTime<chrono::Utc>,
        evidence_mode: String,
        total_count: i64,
    }
    let (recent_interactions, recent_interactions_total) = if node.node_type == "person" {
        let rows: Vec<RecentInteractionRow> = sqlx::query_as(
            "WITH evidence AS ( \
                 SELECT source.id AS source_id, source.node_type AS source_type, source.canonical_text AS title, \
                        source.occurred_at, 'participated_in' AS evidence_mode, 1 AS precedence \
                 FROM edges e JOIN nodes source ON source.id = e.to_id \
                 WHERE e.from_id = $1 AND e.edge_type = 'participated_in' AND source.occurred_at IS NOT NULL \
                 UNION ALL \
                 SELECT source.id, source.node_type, source.canonical_text, \
                        source.occurred_at, 'legacy_speaker', 2 \
                 FROM source_fragments sf JOIN nodes source ON source.id = sf.source_id \
                 WHERE sf.speaker = $2 AND source.occurred_at IS NOT NULL \
             ), \
             deduped AS ( \
                 SELECT DISTINCT ON (source_id) source_id, source_type, title, occurred_at, evidence_mode \
                 FROM evidence \
                 ORDER BY source_id, precedence \
             ) \
             SELECT source_id, source_type, title, occurred_at, evidence_mode, COUNT(*) OVER() AS total_count \
             FROM deduped ORDER BY occurred_at DESC, source_id LIMIT 10",
        )
        .bind(node.id)
        .bind(&node.canonical_text)
        .fetch_all(&pool)
        .await
        .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

        let total = rows.first().map(|row| row.total_count).unwrap_or(0);
        let capped: Vec<JsonValue> = rows
            .into_iter()
            .map(|row| {
                json!({
                    "source_id": row.source_id,
                    "source_type": row.source_type,
                    "title": row.title,
                    "occurred_at": row.occurred_at.to_rfc3339(),
                    "evidence_mode": row.evidence_mode,
                })
            })
            .collect();
        (capped, total)
    } else {
        (Vec::new(), 0)
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
        "recent_interactions": recent_interactions,
        "recent_interactions_total": recent_interactions_total,
    })))
}

#[derive(Debug, FromRow)]
struct BriefObligationRow {
    obligation_id: Uuid,
    status: String,
    updated_at: chrono::DateTime<chrono::Utc>,
    hard_due_at: Option<chrono::DateTime<chrono::Utc>>,
    soft_due_at: Option<chrono::DateTime<chrono::Utc>>,
    source_fragment_id: Option<Uuid>,
    source_text: Option<String>,
    has_owner: bool,
    has_edges: bool,
}

#[derive(Debug, FromRow)]
struct RecentAskRow {
    candidate_id: Uuid,
    candidate_type: String,
    statement: String,
    validation_state: String,
    confidence: Option<f32>,
    source_text: Option<String>,
    speaker: Option<String>,
    occurred_at: Option<chrono::DateTime<chrono::Utc>>,
    total_count: i64,
}

/// ADR-0083: composes a person's open commitments, recent asks, and
/// outstanding risks into one read -- reusing `risk_signals`/
/// `daily_brief_reason` verbatim for the same "outstanding risk" and
/// "reason" definitions Daily Brief/Time Horizon/Person detail already
/// share, plus one genuinely new join for "recent asks" (candidate ->
/// source fragment -> meeting -> `participated_in` -> person) that exists
/// nowhere else. Exposed over both HTTP (this function is the route
/// handler directly) and the `prepare_meeting_brief` MCP tool, which calls
/// this exact function rather than duplicating the queries.
pub async fn person_brief(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let node = graph::get_node(&pool, id).await.map_err(|error| match error {
        sqlx::Error::RowNotFound => (axum::http::StatusCode::NOT_FOUND, "person not found".to_string()),
        other => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
    })?;
    if node.node_type != "person" {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            format!("node {id} is a {:?}, not a person", node.node_type),
        ));
    }

    let mut obligation_rows: Vec<BriefObligationRow> = sqlx::query_as(
        "SELECT op.obligation_id, op.status, op.updated_at, op.hard_due_at, op.soft_due_at, \
                op.source_fragment_id, sf.text AS source_text, \
                EXISTS ( \
                    SELECT 1 FROM edges oe JOIN nodes on2 ON on2.id = oe.from_id \
                    WHERE oe.to_id = op.obligation_id AND oe.edge_type = 'owns' AND on2.node_type = 'person' \
                ) AS has_owner, \
                EXISTS ( \
                    SELECT 1 FROM edges oe WHERE oe.from_id = op.obligation_id OR oe.to_id = op.obligation_id \
                ) AS has_edges \
         FROM edges e \
         JOIN obligation_projection op ON op.obligation_id = (CASE WHEN e.from_id = $1 THEN e.to_id ELSE e.from_id END) \
         LEFT JOIN source_fragments sf ON sf.id = op.source_fragment_id \
         WHERE (e.from_id = $1 OR e.to_id = $1) AND op.status != 'closed'",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    obligation_rows.sort_by_key(|row| (due_date_sort_key(row.hard_due_at), due_date_sort_key(row.soft_due_at)));

    let open_commitments: Vec<JsonValue> = obligation_rows
        .iter()
        .map(|row| {
            json!({
                "obligation_id": row.obligation_id,
                "status": row.status,
                "hard_due_at": row.hard_due_at.map(|value| value.to_rfc3339()),
                "soft_due_at": row.soft_due_at.map(|value| value.to_rfc3339()),
                "reason": daily_brief_reason(&row.status, row.hard_due_at, row.soft_due_at, row.source_text.as_deref()),
                "risk_signals": risk_signals(
                    row.hard_due_at,
                    row.soft_due_at,
                    row.updated_at,
                    row.source_fragment_id,
                    row.has_owner,
                    row.has_edges,
                ),
            })
        })
        .collect();

    // ADR-0083: candidates from meetings this person participated in, still
    // open to action -- excludes rejected (not a genuine management object,
    // ADR-0045) and promoted (already represented in open_commitments).
    // Capped at 10 with an honest total, matching ADR-0071's precedent.
    let ask_rows: Vec<RecentAskRow> = sqlx::query_as(
        "WITH evidence AS ( \
             SELECT cp.candidate_id, cp.candidate_type, cp.statement, cp.validation_state, cp.confidence, \
                    sf.text AS source_text, sf.speaker, src.occurred_at \
             FROM candidate_projection cp \
             JOIN source_fragments sf ON sf.id = cp.source_fragment_id \
             JOIN edges e ON e.to_id = sf.source_id AND e.edge_type = 'participated_in' \
             JOIN nodes src ON src.id = sf.source_id \
             WHERE e.from_id = $1 AND cp.validation_state NOT IN ('rejected', 'promoted') \
         ) \
         SELECT candidate_id, candidate_type, statement, validation_state, confidence, source_text, speaker, occurred_at, \
                COUNT(*) OVER() AS total_count \
         FROM evidence \
         ORDER BY occurred_at DESC NULLS LAST, candidate_id \
         LIMIT 10",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    let recent_asks_total = ask_rows.first().map(|row| row.total_count).unwrap_or(0);
    let recent_asks: Vec<JsonValue> = ask_rows
        .into_iter()
        .map(|row| {
            json!({
                "candidate_id": row.candidate_id,
                "candidate_type": row.candidate_type,
                "statement": row.statement,
                "validation_state": row.validation_state,
                "confidence": row.confidence,
                "source_text": row.source_text,
                "speaker": row.speaker,
                "occurred_at": row.occurred_at.map(|value| value.to_rfc3339()),
            })
        })
        .collect();

    Ok(Json(json!({
        "person": { "id": node.id, "canonical_text": node.canonical_text },
        "open_commitments": open_commitments,
        "recent_asks": recent_asks,
        "recent_asks_total": recent_asks_total,
    })))
}

#[derive(Debug, FromRow)]
struct CareerHistoryRow {
    obligation_id: Uuid,
    updated_at: chrono::DateTime<chrono::Utc>,
    source_text: Option<String>,
}

/// A closed-obligation-only reason (ADR-0088): `daily_brief_reason`'s
/// due-date clause ("Due in N days.") reads nonsensically for an
/// already-closed item, so this reuses only its evidence-clause wording.
fn career_history_reason(source_text: Option<&str>) -> String {
    match source_text {
        Some(text) => {
            let truncated: String = text.chars().take(80).collect();
            format!("Last evidence: \"{truncated}\".")
        }
        None => "No evidence recorded.".to_string(),
    }
}

/// ADR-0088: a person's completed obligation history, for a Career/Connect
/// export -- the exact opposite filter of `person_brief`'s
/// `open_commitments` and `get_node_detail`'s `relationship` grouping,
/// both of which explicitly exclude `status = 'closed'` rows. No stored
/// People/Delivery/Leadership/Operational category exists to filter this
/// further (ADR-0082/ADR-0085), so every closed Obligation linked to the
/// person is returned, honestly unfiltered by category.
pub async fn person_career_history(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let node = graph::get_node(&pool, id).await.map_err(|error| match error {
        sqlx::Error::RowNotFound => (axum::http::StatusCode::NOT_FOUND, "person not found".to_string()),
        other => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
    })?;
    if node.node_type != "person" {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            format!("node {id} is a {:?}, not a person", node.node_type),
        ));
    }

    let rows: Vec<CareerHistoryRow> = sqlx::query_as(
        "SELECT op.obligation_id, op.updated_at, sf.text AS source_text \
         FROM edges e \
         JOIN obligation_projection op ON op.obligation_id = (CASE WHEN e.from_id = $1 THEN e.to_id ELSE e.from_id END) \
         LEFT JOIN source_fragments sf ON sf.id = op.source_fragment_id \
         WHERE (e.from_id = $1 OR e.to_id = $1) AND op.status = 'closed' \
         ORDER BY op.updated_at DESC",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    let completed: Vec<JsonValue> = rows
        .iter()
        .map(|row| {
            json!({
                "obligation_id": row.obligation_id,
                "updated_at": row.updated_at.to_rfc3339(),
                "reason": career_history_reason(row.source_text.as_deref()),
            })
        })
        .collect();

    Ok(Json(json!({
        "person": { "id": node.id, "canonical_text": node.canonical_text },
        "completed": completed,
    })))
}

#[derive(Debug, Deserialize)]
pub(super) struct UpdateNodeRequest {
    canonical_text: Option<String>,
    lifecycle_state: Option<String>,
    attributes: Option<JsonValue>,
}

/// Enriches an existing node (ADR-0025): a shallow merge of `attributes`,
/// never a wholesale replace. `404` for an unknown id.
pub(super) async fn update_node_route(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateNodeRequest>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let node = graph::update_node(
        &pool,
        id,
        body.canonical_text.as_deref(),
        body.lifecycle_state.as_deref(),
        body.attributes.as_ref(),
    )
    .await
    .map_err(|error| match error {
        sqlx::Error::RowNotFound => (
            axum::http::StatusCode::NOT_FOUND,
            "node not found".to_string(),
        ),
        other => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            other.to_string(),
        ),
    })?;
    Ok(Json(json!(node)))
}

#[derive(Debug, Deserialize)]
pub(super) struct CreateEdgeRequest {
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
pub(super) async fn create_edge_route(
    State(pool): State<PgPool>,
    Json(body): Json<CreateEdgeRequest>,
) -> Result<Response, (axum::http::StatusCode, String)> {
    let id = graph::create_edge_with_options(
        &pool,
        body.from_id,
        body.to_id,
        &body.edge_type,
        body.confidence,
        body.valid_from,
        body.supersede,
    )
    .await
    .map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;
    let edge = graph::get_edge(&pool, id).await.map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;
    Ok((axum::http::StatusCode::CREATED, Json(json!(edge))).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{app, test_pool};
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

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
        let body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let node_id = created["id"]
            .as_str()
            .expect("created node has an id")
            .to_string();

        let list_response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/nodes?node_type=person")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(list_response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(list_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let listed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert!(
            listed
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["id"] == node_id),
            "the just-created node must be listed"
        );

        let patch_response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/nodes/{node_id}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"attributes": {"team": "platform"}}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(patch_response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(patch_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let patched: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert_eq!(
            patched["attributes"]["role"], "manager",
            "enrichment must not clobber a previously-recorded attribute"
        );
        assert_eq!(
            patched["attributes"]["team"], "platform",
            "the newly-enriched attribute must be present"
        );

        let detail_response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri(format!("/api/nodes/{node_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(detail_response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(detail_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert!(
            detail["neighbors"].is_array(),
            "detail response must include a neighbors array"
        );
    }

    /// ADR-0042: occurred_at was write-only since ADR-0040 -- this proves
    /// it round-trips through the read routes and can filter/exclude by
    /// range, not just get silently dropped.
    #[tokio::test]
    async fn nodes_route_filters_by_occurred_at_range_and_rejects_an_unparseable_bound() {
        let pool = test_pool().await;
        let in_range = graph::create_node(&pool, "note", "In Range Note", json!({}))
            .await
            .expect("create in-range node");
        let out_of_range = graph::create_node(&pool, "note", "Out Of Range Note", json!({}))
            .await
            .expect("create out-of-range node");
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

        let from = (chrono::Utc::now() - chrono::Duration::days(4))
            .to_rfc3339()
            .replace('+', "%2B");
        let to = (chrono::Utc::now() - chrono::Duration::days(2))
            .to_rfc3339()
            .replace('+', "%2B");
        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri(format!("/api/nodes?occurred_from={from}&occurred_to={to}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let listed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let ids: Vec<&str> = listed
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["id"].as_str().unwrap())
            .collect();
        assert!(
            ids.contains(&in_range.to_string().as_str()),
            "in-range node must be listed"
        );
        assert!(
            !ids.contains(&out_of_range.to_string().as_str()),
            "out-of-range node must be excluded"
        );
        assert!(
            listed
                .as_array()
                .unwrap()
                .iter()
                .any(|row| !row["occurred_at"].is_null()),
            "occurred_at must round-trip through the response, not be dropped"
        );

        let bad_response = app(pool)
            .oneshot(
                Request::builder()
                    .uri("/api/nodes?occurred_from=not-a-date")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bad_response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    /// ADR-0059: `?limit=`/`?offset=` page GET /api/nodes; omitting both
    /// keeps returning every matching row (this route's exact prior
    /// behavior, unchanged).
    #[tokio::test]
    async fn nodes_route_applies_limit_and_offset() {
        let pool = test_pool().await;
        graph::create_node(
            &pool,
            "note",
            "Nodes Route Pagination Test First",
            json!({}),
        )
        .await
        .expect("create first");
        graph::create_node(
            &pool,
            "note",
            "Nodes Route Pagination Test Second",
            json!({}),
        )
        .await
        .expect("create second");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/nodes?node_type=note&limit=1")
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

    /// ADR-0051: a `?node_type=person` list response is enriched with
    /// `open_count`/`at_risk_count`/`last_interaction_at`; `?needs_attention=true`
    /// excludes a person with no linked open/at_risk Obligation.
    #[tokio::test]
    async fn nodes_route_person_list_is_enriched_and_needs_attention_filters() {
        let pool = test_pool().await;
        let owed_person =
            graph::create_node(&pool, "person", "Route Enrichment Test Owed", json!({}))
                .await
                .expect("create owed person");
        let idle_person =
            graph::create_node(&pool, "person", "Route Enrichment Test Idle", json!({}))
                .await
                .expect("create idle person");

        let obligation_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            obligation_id,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "at_risk"}),
        )
        .await
        .expect("append at_risk obligation");
        graph::create_edge(&pool, owed_person, obligation_id, "owns", None)
            .await
            .expect("link owed person");
        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/nodes?node_type=person")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let listed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let owed_row = listed
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == owed_person.to_string())
            .expect("the owed person must be present");
        assert_eq!(owed_row["at_risk_count"], 1);
        assert_eq!(owed_row["open_count"], 0);
        let idle_row = listed
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == idle_person.to_string())
            .expect("the idle person must be present");
        assert_eq!(idle_row["at_risk_count"], 0);
        assert_eq!(idle_row["open_count"], 0);

        let filtered_response = app(pool)
            .oneshot(
                Request::builder()
                    .uri("/api/nodes?node_type=person&needs_attention=true")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let filtered_body = axum::body::to_bytes(filtered_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let filtered: JsonValue = serde_json::from_slice(&filtered_body).expect("valid json body");
        let filtered_ids: Vec<&str> = filtered
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["id"].as_str().unwrap())
            .collect();
        assert!(
            filtered_ids.contains(&owed_person.to_string().as_str()),
            "a person needing attention must be included"
        );
        assert!(
            !filtered_ids.contains(&idle_person.to_string().as_str()),
            "a person needing nothing must be excluded"
        );
    }

    #[tokio::test]
    async fn node_detail_route_returns_404_for_unknown_node() {
        let pool = test_pool().await;
        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/nodes/{}", uuid::Uuid::new_v4()))
                    .body(Body::empty())
                    .unwrap(),
            )
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
        let person_id = graph::create_node(&pool, "person", "Neighbor Test Person", json!({}))
            .await
            .expect("create person");
        let risk_id = graph::create_node(&pool, "risk", "Neighbor Test Risk", json!({}))
            .await
            .expect("create risk");
        graph::create_edge(&pool, person_id, risk_id, "flagged", None)
            .await
            .expect("create edge to a real node");
        let obligation_id = uuid::Uuid::new_v4(); // a genuinely unknown id: neither a nodes row nor a real Obligation.
        graph::create_edge(&pool, person_id, obligation_id, "made", None)
            .await
            .expect("create edge to a non-node id");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/nodes/{person_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let neighbors = detail["neighbors"].as_array().unwrap();

        let to_risk = neighbors
            .iter()
            .find(|edge| edge["to_id"] == risk_id.to_string())
            .expect("edge to the risk node present");
        assert_eq!(to_risk["neighbor"]["id"], risk_id.to_string());
        assert_eq!(to_risk["neighbor"]["canonical_text"], "Neighbor Test Risk");

        let to_obligation = neighbors
            .iter()
            .find(|edge| edge["to_id"] == obligation_id.to_string())
            .expect("edge to the non-node id present");
        assert!(
            to_obligation["neighbor"].is_null(),
            "an edge whose other end is not a nodes row must report a null neighbor, not fail"
        );
    }

    /// ADR-0028: an edge into a *real* Obligation resolves with its status
    /// and the same `reason` text the Daily Brief shows, and a person's
    /// linked, non-closed Obligations are grouped into at_risk/open.
    #[tokio::test]
    async fn node_detail_resolves_a_real_linked_obligation_with_status_and_reason() {
        let pool = test_pool().await;
        let person_id = graph::create_node(&pool, "person", "Relationship Test Person", json!({}))
            .await
            .expect("create person");

        let at_risk_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            at_risk_id,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "at_risk"}),
        )
        .await
        .expect("append an at-risk obligation");
        graph::create_edge(&pool, person_id, at_risk_id, "owns", None)
            .await
            .expect("link person to the at-risk obligation");

        let closed_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            closed_id,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open"}),
        )
        .await
        .expect("append an obligation to be closed");
        crate::obligation::append_event(
            &pool,
            closed_id,
            crate::obligation::ObligationEventType::Closed,
            json!({}),
        )
        .await
        .expect("close it");
        graph::create_edge(&pool, person_id, closed_id, "owns", None)
            .await
            .expect("link person to the closed obligation");

        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/nodes/{person_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");

        let neighbors = detail["neighbors"].as_array().unwrap();
        let to_at_risk = neighbors
            .iter()
            .find(|edge| edge["to_id"] == at_risk_id.to_string())
            .expect("edge to the at-risk obligation present");
        assert_eq!(to_at_risk["neighbor"]["type"], "obligation");
        assert_eq!(to_at_risk["neighbor"]["status"], "at_risk");
        assert_eq!(
            to_at_risk["neighbor"]["reason"],
            "Marked at risk. No evidence recorded."
        );

        let relationship = &detail["relationship"];
        let at_risk_group = relationship["at_risk"].as_array().unwrap();
        assert!(
            at_risk_group
                .iter()
                .any(|entry| entry["obligation_id"] == at_risk_id.to_string()),
            "the at-risk obligation must appear in the at_risk group"
        );
        let open_group = relationship["open"].as_array().unwrap();
        assert!(
            !open_group
                .iter()
                .any(|entry| entry["obligation_id"] == closed_id.to_string()),
            "a closed obligation must never appear in either relationship group"
        );
        assert!(
            !at_risk_group
                .iter()
                .any(|entry| entry["obligation_id"] == closed_id.to_string()),
            "a closed obligation must never appear in either relationship group"
        );
    }

    /// ADR-0051: each Obligation in a person's relationship grouping
    /// carries risk_signals, the same computation Daily Brief/Time Horizon
    /// already use -- an owned obligation must not be flagged unowned.
    #[tokio::test]
    async fn node_detail_relationship_obligations_include_risk_signals() {
        let pool = test_pool().await;
        let person_id = graph::create_node(
            &pool,
            "person",
            "Relationship Risk Signal Test Person",
            json!({}),
        )
        .await
        .expect("create person");

        let stale_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            stale_id,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open"}),
        )
        .await
        .expect("append a stale obligation");
        graph::create_edge(&pool, person_id, stale_id, "owns", None)
            .await
            .expect("link person to the stale obligation");
        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");
        sqlx::query("UPDATE obligation_projection SET updated_at = now() - interval '30 days' WHERE obligation_id = $1")
            .bind(stale_id)
            .execute(&pool)
            .await
            .expect("backdate updated_at");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/nodes/{person_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");

        let open_group = detail["relationship"]["open"].as_array().unwrap();
        let entry = open_group
            .iter()
            .find(|entry| entry["obligation_id"] == stale_id.to_string())
            .expect("the stale obligation must be present");
        let signals = entry["risk_signals"]
            .as_array()
            .expect("risk_signals is an array");
        assert!(
            signals.iter().any(|signal| signal["signal"] == "stale"),
            "a backdated obligation must be flagged stale"
        );
        assert!(
            signals.iter().all(|signal| signal["signal"] != "unowned"),
            "an owned obligation must not be flagged unowned"
        );
    }

    /// ADR-0051: last_interaction_at is the most recent occurred_at among
    /// fragments whose speaker string-matches this person's canonical_text
    /// -- a best-effort name match, not a resolved identity edge.
    #[tokio::test]
    async fn node_detail_includes_last_interaction_at_from_matching_fragment_speaker() {
        let pool = test_pool().await;
        let person_name = "Last Interaction Test Person";
        let person_id = graph::create_node(&pool, "person", person_name, json!({}))
            .await
            .expect("create person");

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
            .oneshot(
                Request::builder()
                    .uri(format!("/api/nodes/{person_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");

        let last_interaction_at = detail["last_interaction_at"]
            .as_str()
            .expect("last_interaction_at must be present");
        let parsed =
            chrono::DateTime::parse_from_rfc3339(last_interaction_at).expect("valid RFC3339");
        assert!(
            (parsed.timestamp() - occurred_at.timestamp()).abs() < 2,
            "must reflect the matching fragment's source occurred_at"
        );
    }

    /// ADR-0051: no matching fragment speaker means an honest null, never a guess.
    #[tokio::test]
    async fn node_detail_last_interaction_at_is_null_with_no_matching_fragment() {
        let pool = test_pool().await;
        let person_id =
            graph::create_node(&pool, "person", "No Interaction Test Person", json!({}))
                .await
                .expect("create person");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/nodes/{person_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert!(detail["last_interaction_at"].is_null());
    }

    /// ADR-0070: a `participated_in` edge alone (no matching fragment speaker
    /// at all) still derives `last_interaction_at` on Person detail.
    #[tokio::test]
    async fn person_detail_uses_participation_edge_for_last_interaction_at() {
        let pool = test_pool().await;
        let marker = uuid::Uuid::new_v4();
        let person_id = graph::create_node(
            &pool,
            "person",
            &format!("Edge Recency Person {marker}"),
            json!({}),
        )
        .await
        .expect("create person");
        let source_id = uuid::Uuid::new_v4();
        let occurred_at = chrono::Utc::now() - chrono::Duration::days(3);
        sqlx::query("INSERT INTO nodes (id, node_type, canonical_text, attributes, occurred_at) VALUES ($1, 'note', 'Edge Recency Source', '{}'::jsonb, $2)")
            .bind(source_id)
            .bind(occurred_at)
            .execute(&pool)
            .await
            .expect("create a dated source node");
        graph::create_edge(&pool, person_id, source_id, "participated_in", Some(1.0))
            .await
            .expect("link person to source by edge");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/nodes/{person_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");

        let last_interaction_at = detail["last_interaction_at"]
            .as_str()
            .expect("last_interaction_at must be present from the edge alone");
        let parsed =
            chrono::DateTime::parse_from_rfc3339(last_interaction_at).expect("valid RFC3339");
        assert!(
            (parsed.timestamp() - occurred_at.timestamp()).abs() < 2,
            "must reflect the participated_in source's occurred_at"
        );
    }

    /// ADR-0070: the `?node_type=person` list route derives the same
    /// edge-backed `last_interaction_at` in its batched query, not just detail.
    #[tokio::test]
    async fn person_list_uses_participation_edge_for_last_interaction_at() {
        let pool = test_pool().await;
        let marker = uuid::Uuid::new_v4();
        let person_id = graph::create_node(
            &pool,
            "person",
            &format!("Edge Recency List Person {marker}"),
            json!({}),
        )
        .await
        .expect("create person");
        let source_id = uuid::Uuid::new_v4();
        let occurred_at = chrono::Utc::now() - chrono::Duration::days(4);
        sqlx::query("INSERT INTO nodes (id, node_type, canonical_text, attributes, occurred_at) VALUES ($1, 'note', 'Edge Recency List Source', '{}'::jsonb, $2)")
            .bind(source_id)
            .bind(occurred_at)
            .execute(&pool)
            .await
            .expect("create a dated source node");
        graph::create_edge(&pool, person_id, source_id, "participated_in", Some(1.0))
            .await
            .expect("link person to source by edge");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri("/api/nodes?node_type=person")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let listed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let row = listed
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == person_id.to_string())
            .expect("the person must be present");

        let last_interaction_at = row["last_interaction_at"]
            .as_str()
            .expect("last_interaction_at must be present from the edge alone");
        let parsed =
            chrono::DateTime::parse_from_rfc3339(last_interaction_at).expect("valid RFC3339");
        assert!(
            (parsed.timestamp() - occurred_at.timestamp()).abs() < 2,
            "must reflect the participated_in source's occurred_at"
        );
    }

    /// ADR-0070: a pre-ADR-0069 source with only an exact legacy speaker
    /// match (no `participated_in` edge at all) still contributes to
    /// `last_interaction_at` in the batched People list route, not only detail.
    #[tokio::test]
    async fn person_list_uses_legacy_speaker_fallback_with_no_participation_edge() {
        let pool = test_pool().await;
        let marker = uuid::Uuid::new_v4();
        let person_name = format!("Legacy Fallback List Person {marker}");
        let person_id = graph::create_node(&pool, "person", &person_name, json!({}))
            .await
            .expect("create person");

        let source_id = uuid::Uuid::new_v4();
        let occurred_at = chrono::Utc::now() - chrono::Duration::days(5);
        sqlx::query("INSERT INTO nodes (id, node_type, canonical_text, attributes, occurred_at) VALUES ($1, 'meeting', 'Legacy Fallback List Source', '{}'::jsonb, $2)")
            .bind(source_id)
            .bind(occurred_at)
            .execute(&pool)
            .await
            .expect("create a dated legacy source node");
        sqlx::query("INSERT INTO source_fragments (source_id, text, speaker, hash) VALUES ($1, 'hello', $2, 'legacy-fallback-list-hash')")
            .bind(source_id)
            .bind(&person_name)
            .execute(&pool)
            .await
            .expect("create a fragment spoken by this person, with no participated_in edge");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri("/api/nodes?node_type=person")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let listed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let row = listed
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == person_id.to_string())
            .expect("the person must be present");

        let last_interaction_at = row["last_interaction_at"]
            .as_str()
            .expect("last_interaction_at must be present from the legacy speaker match alone");
        let parsed =
            chrono::DateTime::parse_from_rfc3339(last_interaction_at).expect("valid RFC3339");
        assert!(
            (parsed.timestamp() - occurred_at.timestamp()).abs() < 2,
            "must reflect the legacy-matched source's occurred_at with no edge present"
        );
    }

    /// ADR-0070: when both an edge-backed source and a legacy speaker-matched
    /// source exist, the newest `occurred_at` wins regardless of which path
    /// it comes from -- proven both ways round.
    #[tokio::test]
    async fn newest_interaction_wins_between_edge_and_legacy_paths() {
        let pool = test_pool().await;
        let marker = uuid::Uuid::new_v4();
        let person_name = format!("Newest Wins Person {marker}");
        let person_id = graph::create_node(&pool, "person", &person_name, json!({}))
            .await
            .expect("create person");

        let older_edge_source = uuid::Uuid::new_v4();
        let older_at = chrono::Utc::now() - chrono::Duration::days(20);
        sqlx::query("INSERT INTO nodes (id, node_type, canonical_text, attributes, occurred_at) VALUES ($1, 'note', 'Older Edge Source', '{}'::jsonb, $2)")
            .bind(older_edge_source)
            .bind(older_at)
            .execute(&pool)
            .await
            .expect("create older edge-linked source");
        graph::create_edge(
            &pool,
            person_id,
            older_edge_source,
            "participated_in",
            Some(1.0),
        )
        .await
        .expect("link older edge source");

        let newer_legacy_source = uuid::Uuid::new_v4();
        let newer_at = chrono::Utc::now() - chrono::Duration::days(1);
        sqlx::query("INSERT INTO nodes (id, node_type, canonical_text, attributes, occurred_at) VALUES ($1, 'meeting', 'Newer Legacy Source', '{}'::jsonb, $2)")
            .bind(newer_legacy_source)
            .bind(newer_at)
            .execute(&pool)
            .await
            .expect("create newer legacy source");
        sqlx::query("INSERT INTO source_fragments (source_id, text, speaker, hash) VALUES ($1, 'hello', $2, 'newest-wins-legacy-hash')")
            .bind(newer_legacy_source)
            .bind(&person_name)
            .execute(&pool)
            .await
            .expect("create a fragment spoken by this person");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri(format!("/api/nodes/{person_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let last_interaction_at = detail["last_interaction_at"]
            .as_str()
            .expect("last_interaction_at must be present");
        let parsed =
            chrono::DateTime::parse_from_rfc3339(last_interaction_at).expect("valid RFC3339");
        assert!(
            (parsed.timestamp() - newer_at.timestamp()).abs() < 2,
            "the newer legacy-path date must win over the older edge-path date"
        );

        let list_response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/nodes?node_type=person")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let list_body = axum::body::to_bytes(list_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let listed: JsonValue = serde_json::from_slice(&list_body).expect("valid json body");
        let row = listed
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == person_id.to_string())
            .expect("the person must be present");
        let row_last_interaction_at = row["last_interaction_at"]
            .as_str()
            .expect("last_interaction_at must be present");
        let row_parsed =
            chrono::DateTime::parse_from_rfc3339(row_last_interaction_at).expect("valid RFC3339");
        assert!(
            (row_parsed.timestamp() - newer_at.timestamp()).abs() < 2,
            "the batched list route must also prefer the newer date across both paths"
        );

        let newest_edge_source = uuid::Uuid::new_v4();
        let newest_edge_at = chrono::Utc::now();
        sqlx::query("INSERT INTO nodes (id, node_type, canonical_text, attributes, occurred_at) VALUES ($1, 'note', 'Newest Edge Source', '{}'::jsonb, $2)")
            .bind(newest_edge_source)
            .bind(newest_edge_at)
            .execute(&pool)
            .await
            .expect("create newest edge-linked source");
        graph::create_edge(
            &pool,
            person_id,
            newest_edge_source,
            "participated_in",
            Some(1.0),
        )
        .await
        .expect("link newest edge source");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri(format!("/api/nodes/{person_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let last_interaction_at = detail["last_interaction_at"]
            .as_str()
            .expect("last_interaction_at must be present");
        let parsed =
            chrono::DateTime::parse_from_rfc3339(last_interaction_at).expect("valid RFC3339");
        assert!(
            (parsed.timestamp() - newest_edge_at.timestamp()).abs() < 2,
            "the newer edge-path date must win over the older legacy-path date"
        );

        let list_response = app(pool)
            .oneshot(
                Request::builder()
                    .uri("/api/nodes?node_type=person")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let list_body = axum::body::to_bytes(list_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let listed: JsonValue = serde_json::from_slice(&list_body).expect("valid json body");
        let row = listed
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == person_id.to_string())
            .expect("the person must be present");
        let row_last_interaction_at = row["last_interaction_at"]
            .as_str()
            .expect("last_interaction_at must be present");
        let row_parsed =
            chrono::DateTime::parse_from_rfc3339(row_last_interaction_at).expect("valid RFC3339");
        assert!(
            (row_parsed.timestamp() - newest_edge_at.timestamp()).abs() < 2,
            "the batched list route must prefer the newer edge date across both paths"
        );
    }

    /// ADR-0071: Person detail's Past section returns deduplicated interaction
    /// sources newest-first, drawing on both the participated_in edge path
    /// and the legacy speaker path.
    #[tokio::test]
    async fn person_detail_returns_recent_interactions_newest_first_across_both_paths() {
        let pool = test_pool().await;
        let marker = uuid::Uuid::new_v4();
        let person_name = format!("Recent Interactions Person {marker}");
        let person_id = graph::create_node(&pool, "person", &person_name, json!({}))
            .await
            .expect("create person");

        let edge_source = uuid::Uuid::new_v4();
        let edge_at = chrono::Utc::now() - chrono::Duration::days(2);
        sqlx::query("INSERT INTO nodes (id, node_type, canonical_text, attributes, occurred_at) VALUES ($1, 'note', 'Edge Interaction Source', '{}'::jsonb, $2)")
            .bind(edge_source)
            .bind(edge_at)
            .execute(&pool)
            .await
            .expect("create edge-linked source");
        graph::create_edge(&pool, person_id, edge_source, "participated_in", Some(1.0))
            .await
            .expect("link edge source");

        let legacy_source = uuid::Uuid::new_v4();
        let legacy_at = chrono::Utc::now() - chrono::Duration::days(5);
        sqlx::query("INSERT INTO nodes (id, node_type, canonical_text, attributes, occurred_at) VALUES ($1, 'meeting', 'Legacy Interaction Source', '{}'::jsonb, $2)")
            .bind(legacy_source)
            .bind(legacy_at)
            .execute(&pool)
            .await
            .expect("create legacy-matched source");
        sqlx::query("INSERT INTO source_fragments (source_id, text, speaker, hash) VALUES ($1, 'hello', $2, 'recent-interactions-legacy-hash')")
            .bind(legacy_source)
            .bind(&person_name)
            .execute(&pool)
            .await
            .expect("create a fragment spoken by this person");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/nodes/{person_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");

        let interactions = detail["recent_interactions"]
            .as_array()
            .expect("recent_interactions is an array");
        assert_eq!(
            interactions.len(),
            2,
            "both evidence paths must contribute an interaction"
        );
        assert_eq!(detail["recent_interactions_total"], 2);
        assert_eq!(
            interactions[0]["source_id"],
            edge_source.to_string(),
            "the newer edge-path source must come first"
        );
        assert_eq!(interactions[0]["evidence_mode"], "participated_in");
        assert_eq!(interactions[1]["source_id"], legacy_source.to_string());
        assert_eq!(interactions[1]["evidence_mode"], "legacy_speaker");
    }

    /// ADR-0071: when one source is reachable by both a participated_in edge
    /// and a legacy speaker match, it must appear exactly once, with
    /// participated_in as the reported evidence mode.
    #[tokio::test]
    async fn recent_interactions_deduplicate_a_source_with_edge_precedence() {
        let pool = test_pool().await;
        let marker = uuid::Uuid::new_v4();
        let person_name = format!("Dedup Interactions Person {marker}");
        let person_id = graph::create_node(&pool, "person", &person_name, json!({}))
            .await
            .expect("create person");

        let source_id = uuid::Uuid::new_v4();
        let occurred_at = chrono::Utc::now() - chrono::Duration::days(1);
        sqlx::query("INSERT INTO nodes (id, node_type, canonical_text, attributes, occurred_at) VALUES ($1, 'meeting', 'Dual Evidence Source', '{}'::jsonb, $2)")
            .bind(source_id)
            .bind(occurred_at)
            .execute(&pool)
            .await
            .expect("create a source reachable by both paths");
        graph::create_edge(&pool, person_id, source_id, "participated_in", Some(1.0))
            .await
            .expect("link source by edge");
        sqlx::query("INSERT INTO source_fragments (source_id, text, speaker, hash) VALUES ($1, 'hello', $2, 'dedup-interactions-hash')")
            .bind(source_id)
            .bind(&person_name)
            .execute(&pool)
            .await
            .expect("create a fragment spoken by this person for the same source");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/nodes/{person_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");

        let interactions = detail["recent_interactions"]
            .as_array()
            .expect("recent_interactions is an array");
        assert_eq!(
            interactions.len(),
            1,
            "one source reachable by both paths must appear exactly once"
        );
        assert_eq!(detail["recent_interactions_total"], 1);
        assert_eq!(interactions[0]["source_id"], source_id.to_string());
        assert_eq!(
            interactions[0]["evidence_mode"], "participated_in",
            "edge evidence must take precedence over the legacy match"
        );
    }

    /// ADR-0071: more than 10 distinct interaction sources are capped at 10,
    /// newest-first, with an honest total reflecting every deduplicated source.
    #[tokio::test]
    async fn recent_interactions_are_capped_at_ten_with_an_honest_total() {
        let pool = test_pool().await;
        let marker = uuid::Uuid::new_v4();
        let person_id = graph::create_node(
            &pool,
            "person",
            &format!("Capped Interactions Person {marker}"),
            json!({}),
        )
        .await
        .expect("create person");

        for offset in 0..12i64 {
            let source_id = uuid::Uuid::new_v4();
            let occurred_at = chrono::Utc::now() - chrono::Duration::days(offset);
            sqlx::query("INSERT INTO nodes (id, node_type, canonical_text, attributes, occurred_at) VALUES ($1, 'note', 'Capped Source', '{}'::jsonb, $2)")
                .bind(source_id)
                .bind(occurred_at)
                .execute(&pool)
                .await
                .expect("create a dated source");
            graph::create_edge(&pool, person_id, source_id, "participated_in", Some(1.0))
                .await
                .expect("link source by edge");
        }

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/nodes/{person_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");

        let interactions = detail["recent_interactions"]
            .as_array()
            .expect("recent_interactions is an array");
        assert_eq!(
            interactions.len(),
            10,
            "the response must cap at 10 interactions"
        );
        assert_eq!(
            detail["recent_interactions_total"], 12,
            "the total must honestly reflect every deduplicated source"
        );
    }

    /// ADR-0071: non-person nodes always get an empty collection and a zero
    /// total, preserving one stable response shape across node types.
    #[tokio::test]
    async fn recent_interactions_are_empty_for_non_person_nodes() {
        let pool = test_pool().await;
        let risk_id = graph::create_node(
            &pool,
            "risk",
            "Recent Interactions Non-Person Test",
            json!({}),
        )
        .await
        .expect("create risk node");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/nodes/{risk_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");

        assert_eq!(detail["recent_interactions"].as_array().unwrap().len(), 0);
        assert_eq!(detail["recent_interactions_total"], 0);
    }

    #[tokio::test]
    async fn node_detail_omits_relationship_grouping_for_non_person_nodes() {
        let pool = test_pool().await;
        let risk_id = graph::create_node(&pool, "risk", "Non-Person Relationship Test", json!({}))
            .await
            .expect("create risk node");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/nodes/{risk_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert!(
            detail["relationship"].is_null(),
            "a non-person node must not get a relationship grouping"
        );
    }

    /// ADR-0083: open_commitments carries the same risk_signals computation
    /// as the existing relationship grouping, and excludes closed obligations.
    #[tokio::test]
    async fn person_brief_returns_open_commitments_with_risk_signals() {
        let pool = test_pool().await;
        let person_id = graph::create_node(&pool, "person", "Brief Open Commitments Person", json!({}))
            .await
            .expect("create person");

        let open_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            open_id,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open"}),
        )
        .await
        .expect("append an open obligation");
        graph::create_edge(&pool, person_id, open_id, "owns", None)
            .await
            .expect("link person to the open obligation");

        let closed_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            closed_id,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "closed"}),
        )
        .await
        .expect("append a closed obligation");
        graph::create_edge(&pool, person_id, closed_id, "owns", None)
            .await
            .expect("link person to the closed obligation");
        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/people/{person_id}/brief"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let brief: JsonValue = serde_json::from_slice(&body).expect("valid json body");

        let commitments = brief["open_commitments"].as_array().unwrap();
        assert_eq!(commitments.len(), 1, "the closed obligation must be excluded");
        assert_eq!(commitments[0]["obligation_id"], open_id.to_string());
        assert!(
            commitments[0]["risk_signals"].is_array(),
            "each open commitment must carry risk_signals"
        );
    }

    /// ADR-0083: recent_asks draws from candidates whose meeting the person
    /// participated in, excluding rejected and already-promoted candidates.
    #[tokio::test]
    async fn recent_asks_excludes_rejected_and_promoted_candidates() {
        let pool = test_pool().await;
        let person_id = graph::create_node(&pool, "person", "Brief Recent Asks Person", json!({}))
            .await
            .expect("create person");
        let meeting_id = uuid::Uuid::new_v4();
        sqlx::query("INSERT INTO nodes (id, node_type, canonical_text, attributes, occurred_at) VALUES ($1, 'meeting', 'Brief Test Meeting', '{}'::jsonb, now())")
            .bind(meeting_id)
            .execute(&pool)
            .await
            .expect("create meeting node");
        graph::create_edge(&pool, person_id, meeting_id, "participated_in", Some(1.0))
            .await
            .expect("link person to the meeting");

        let pending_id = uuid::Uuid::new_v4();
        let pending_fragment = graph::create_source_fragment(&pool, meeting_id, "pending ask", "brief-pending-hash")
            .await
            .expect("create pending fragment");
        crate::extraction::extract_candidate(&pool, pending_id, "risk", "a pending ask", pending_fragment, Some(0.7), None)
            .await
            .expect("extract pending candidate");

        let rejected_id = uuid::Uuid::new_v4();
        let rejected_fragment = graph::create_source_fragment(&pool, meeting_id, "rejected ask", "brief-rejected-hash")
            .await
            .expect("create rejected fragment");
        crate::extraction::extract_candidate(&pool, rejected_id, "risk", "a rejected ask", rejected_fragment, Some(0.7), None)
            .await
            .expect("extract rejected candidate");
        crate::extraction::transition_candidate(&pool, rejected_id, "rejected", json!({}))
            .await
            .expect("reject candidate");

        let promoted_id = uuid::Uuid::new_v4();
        let promoted_fragment = graph::create_source_fragment(&pool, meeting_id, "promoted ask", "brief-promoted-hash")
            .await
            .expect("create promoted fragment");
        crate::extraction::extract_candidate(&pool, promoted_id, "commitment", "a promoted ask", promoted_fragment, Some(0.8), None)
            .await
            .expect("extract promoted candidate");
        crate::extraction::transition_candidate(&pool, promoted_id, "accepted", json!({}))
            .await
            .expect("accept candidate");
        crate::extraction::transition_candidate(&pool, promoted_id, "promoted", json!({"obligation_id": uuid::Uuid::new_v4()}))
            .await
            .expect("promote candidate");
        crate::extraction::rebuild_candidate_projection(&pool)
            .await
            .expect("rebuild candidate projection");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/people/{person_id}/brief"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let brief: JsonValue = serde_json::from_slice(&body).expect("valid json body");

        let asks = brief["recent_asks"].as_array().unwrap();
        assert_eq!(asks.len(), 1, "only the still-pending candidate must be included");
        assert_eq!(asks[0]["candidate_id"], pending_id.to_string());
        assert_eq!(brief["recent_asks_total"], 1);
    }

    /// ADR-0083: recent_asks is capped at 10 with an honest total, newest
    /// source meeting first -- matching ADR-0071's precedent exactly.
    #[tokio::test]
    async fn recent_asks_are_capped_with_an_honest_total() {
        let pool = test_pool().await;
        let person_id = graph::create_node(&pool, "person", "Brief Capped Asks Person", json!({}))
            .await
            .expect("create person");

        for offset in 0..12i64 {
            let meeting_id = uuid::Uuid::new_v4();
            let occurred_at = chrono::Utc::now() - chrono::Duration::days(offset);
            sqlx::query("INSERT INTO nodes (id, node_type, canonical_text, attributes, occurred_at) VALUES ($1, 'meeting', 'Capped Ask Meeting', '{}'::jsonb, $2)")
                .bind(meeting_id)
                .bind(occurred_at)
                .execute(&pool)
                .await
                .expect("create dated meeting");
            graph::create_edge(&pool, person_id, meeting_id, "participated_in", Some(1.0))
                .await
                .expect("link person to meeting");
            let candidate_id = uuid::Uuid::new_v4();
            let fragment_id = graph::create_source_fragment(&pool, meeting_id, "an ask", &format!("brief-capped-hash-{offset}"))
                .await
                .expect("create fragment");
            crate::extraction::extract_candidate(&pool, candidate_id, "risk", &format!("ask {offset}"), fragment_id, Some(0.7), None)
                .await
                .expect("extract candidate");
        }
        crate::extraction::rebuild_candidate_projection(&pool)
            .await
            .expect("rebuild candidate projection");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/people/{person_id}/brief"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let brief: JsonValue = serde_json::from_slice(&body).expect("valid json body");

        let asks = brief["recent_asks"].as_array().unwrap();
        assert_eq!(asks.len(), 10, "recent_asks must be capped at 10");
        assert_eq!(brief["recent_asks_total"], 12, "the total must honestly reflect every match");
        assert_eq!(asks[0]["statement"], "ask 0", "the most recent meeting's ask must come first");
    }

    #[tokio::test]
    async fn person_brief_rejects_a_non_person_node() {
        let pool = test_pool().await;
        let risk_id = graph::create_node(&pool, "risk", "Brief Non-Person Test", json!({}))
            .await
            .expect("create risk node");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/people/{risk_id}/brief"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn person_brief_returns_honest_empty_lists_with_no_data() {
        let pool = test_pool().await;
        let person_id = graph::create_node(&pool, "person", "Brief Empty Person", json!({}))
            .await
            .expect("create person");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/people/{person_id}/brief"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let brief: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert_eq!(brief["open_commitments"].as_array().unwrap().len(), 0);
        assert_eq!(brief["recent_asks"].as_array().unwrap().len(), 0);
        assert_eq!(brief["recent_asks_total"], 0);
    }

    /// ADR-0088: the Career/Connect export -- the exact opposite filter of
    /// `person_brief`/`get_node_detail`'s relationship grouping, both of
    /// which explicitly exclude closed Obligations.
    #[tokio::test]
    async fn person_career_history_returns_closed_obligations_with_evidence() {
        let pool = test_pool().await;
        let person_id = graph::create_node(&pool, "person", "Career History Person", json!({}))
            .await
            .expect("create person");
        let meeting_id = graph::create_node(&pool, "meeting", "Career History Meeting", json!({}))
            .await
            .expect("create meeting");
        let fragment_id = graph::create_source_fragment(
            &pool,
            meeting_id,
            "Shipped the onboarding checklist for the new hire.",
            "career-history-hash",
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
        .expect("append an obligation to be closed");
        crate::obligation::append_event(
            &pool,
            obligation_id,
            crate::obligation::ObligationEventType::Closed,
            json!({}),
        )
        .await
        .expect("close it");
        graph::create_edge(&pool, person_id, obligation_id, "owns", None)
            .await
            .expect("link person to the closed obligation");
        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/people/{person_id}/career-export"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let export: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let completed = export["completed"].as_array().expect("completed array");
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0]["obligation_id"], obligation_id.to_string());
        assert!(completed[0]["reason"]
            .as_str()
            .expect("reason is a string")
            .contains("Shipped the onboarding checklist"));
    }

    #[tokio::test]
    async fn career_history_excludes_an_open_obligation() {
        let pool = test_pool().await;
        let person_id = graph::create_node(&pool, "person", "Career History Open Test Person", json!({}))
            .await
            .expect("create person");

        let obligation_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            obligation_id,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open"}),
        )
        .await
        .expect("append an open obligation");
        graph::create_edge(&pool, person_id, obligation_id, "owns", None)
            .await
            .expect("link person to the open obligation");
        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/people/{person_id}/career-export"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let export: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert_eq!(
            export["completed"].as_array().expect("completed array").len(),
            0,
            "an open obligation must never appear in the career export"
        );
    }

    #[tokio::test]
    async fn person_career_history_rejects_a_non_person_node() {
        let pool = test_pool().await;
        let risk_id = graph::create_node(&pool, "risk", "Career Export Non-Person Test", json!({}))
            .await
            .expect("create risk node");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/people/{risk_id}/career-export"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn edge_create_route_round_trips() {
        let pool = test_pool().await;
        let from_id = graph::create_node(&pool, "person", "Edge Route Test From", json!({}))
            .await
            .expect("create from-node");
        let to_id = graph::create_node(&pool, "person", "Edge Route Test To", json!({}))
            .await
            .expect("create to-node");

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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert_eq!(created["edge_type"], "collaborates_with");
    }

    /// ADR-0032: supersede omitted (or false) leaves valid_from/valid_to
    /// NULL, unchanged from every pre-ADR-0032 caller's behavior.
    #[tokio::test]
    async fn edge_create_route_without_supersede_leaves_valid_from_and_valid_to_null() {
        let pool = test_pool().await;
        let from_id = graph::create_node(&pool, "person", "Supersede Default Test From", json!({}))
            .await
            .expect("create from-node");
        let to_id = graph::create_node(&pool, "person", "Supersede Default Test To", json!({}))
            .await
            .expect("create to-node");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/edges")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"from_id": from_id, "to_id": to_id, "edge_type": "lives_in"})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert_eq!(created["valid_from"], JsonValue::Null);
        assert_eq!(created["valid_to"], JsonValue::Null);
    }

    /// ADR-0032: supersede: true closes the prior current edge sharing the
    /// same (from_id, edge_type) but leaves a different edge_type on the
    /// same from_id untouched -- matching is (from_id, edge_type) only, not
    /// to_id, matching the LIVES_IN Barcelona -> Madrid example.
    #[tokio::test]
    async fn edge_create_route_with_supersede_closes_the_prior_current_edge_matching_from_id_and_edge_type(
    ) {
        let pool = test_pool().await;
        let user_id = graph::create_node(&pool, "person", "Supersede Test User", json!({}))
            .await
            .expect("create user node");
        let barcelona_id = graph::create_node(&pool, "city", "Barcelona", json!({}))
            .await
            .expect("create barcelona node");
        let madrid_id = graph::create_node(&pool, "city", "Madrid", json!({}))
            .await
            .expect("create madrid node");
        let risk_id = graph::create_node(&pool, "risk", "Unrelated Risk", json!({}))
            .await
            .expect("create risk node");

        let barcelona_edge_id = graph::create_edge(&pool, user_id, barcelona_id, "lives_in", None)
            .await
            .expect("create initial lives_in edge");
        let flagged_edge_id = graph::create_edge(&pool, user_id, risk_id, "flagged", None)
            .await
            .expect("create an unrelated edge_type on the same from_id");

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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        assert!(
            created["valid_from"].is_string(),
            "new current edge must carry a valid_from"
        );
        assert_eq!(
            created["valid_to"],
            JsonValue::Null,
            "the new edge is current"
        );

        let superseded = graph::get_edge(&pool, barcelona_edge_id)
            .await
            .expect("fetch the superseded edge");
        assert!(
            superseded.valid_to.is_some(),
            "the prior lives_in edge must be closed out"
        );

        let untouched = graph::get_edge(&pool, flagged_edge_id)
            .await
            .expect("fetch the unrelated edge_type");
        assert_eq!(
            untouched.valid_to, None,
            "a different edge_type on the same from_id must not be superseded"
        );
    }
}
