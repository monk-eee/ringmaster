use super::{clamp_list_params, ListQuery};
#[cfg(test)]
use crate::graph;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde_json::{json, Value as JsonValue};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Reads the current `obligation_projection` rows (ADR-0005/ADR-0012). Never
/// writes; the projection remains the sole source this route reflects. Joins
/// read-only against the immutable `source_fragments` table for evidence
/// (ADR-0023), the same treatment ADR-0015 already gave `GET /api/candidates`.
/// `limit`/`offset` of `None` fetch every row unchanged (ADR-0059); a given
/// `limit` is clamped to `[1, MAX_LIST_LIMIT]` rather than rejected, matching
/// ADR-0049's audit-limit precedent.
pub(super) async fn list_obligations(
    State(pool): State<PgPool>,
    Query(params): Query<ListQuery>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let (limit, offset) = clamp_list_params(&params);
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
         ORDER BY op.updated_at DESC \
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&pool)
    .await
    .map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
    })?;

    let body = rows
        .into_iter()
        .map(
            |(
                obligation_id,
                status,
                updated_at,
                hard_due_at,
                soft_due_at,
                source_fragment_id,
                source_text,
            )| {
                json!({
                    "obligation_id": obligation_id,
                    "status": status,
                    "updated_at": updated_at.to_rfc3339(),
                    "hard_due_at": hard_due_at.map(|value| value.to_rfc3339()),
                    "soft_due_at": soft_due_at.map(|value| value.to_rfc3339()),
                    "source_fragment_id": source_fragment_id,
                    "source_text": source_text,
                })
            },
        )
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
pub(super) async fn get_obligation_detail(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
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
    let (
        status,
        updated_at,
        hard_due_at,
        soft_due_at,
        source_fragment_id,
        source_text,
        has_owner,
        has_edges,
    ) = row;

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

    let signals = risk_signals(
        hard_due_at,
        soft_due_at,
        updated_at,
        source_fragment_id,
        has_owner,
        has_edges,
    );
    Ok(Json(json!({
        "obligation_id": id,
        "status": &status,
        "updated_at": updated_at.to_rfc3339(),
        "hard_due_at": hard_due_at.map(|value| value.to_rfc3339()),
        "soft_due_at": soft_due_at.map(|value| value.to_rfc3339()),
        "source_fragment_id": source_fragment_id,
        "source_text": source_text,
        "health": obligation_health(&status, hard_due_at, &signals),
        "risk_signals": signals,
        "linked_nodes": linked_nodes,
    })))
}

/// A deterministic reason for one Daily Brief item (ADR-0022), with a second
/// evidence clause added by ADR-0023: cites the linked source fragment's
/// text when present, or states plainly that none is recorded. Never
/// fabricates evidence or groups obligations together.
pub(super) fn daily_brief_reason(
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

/// Risk Engine v1 (ADR-0041): the two of PRODUCT-SPEC.md ┬º7.1's nine
/// signals derivable today with zero schema change and zero fabricated
/// data. Each signal is independent and additive -- no combined severity
/// score is computed here, since weighting them together needs a model
/// this ADR does not decide. `has_owner` (ADR-0046) and `has_edges`
/// (ADR-0054, Congruence Engine v1) are computed by the caller from the
/// existing `edges` table, not by this function -- it stays pure and
/// directly unit-testable.
pub(super) fn risk_signals(
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
        signals.push(
            json!({ "signal": "isolated", "explanation": "Not linked to anyone or anything." }),
        );
    }

    signals
}

/// A derived, five-value Obligation health label (ADR-0061): a
/// deterministic lookup over already-computed fields, not a new signal or
/// a score. `risk_signals` is the same slice this function's caller
/// already computed via `risk_signals()` above -- never recomputed here.
fn obligation_health(
    status: &str,
    hard_due_at: Option<chrono::DateTime<chrono::Utc>>,
    risk_signals: &[JsonValue],
) -> &'static str {
    if status == "closed" {
        return "Completed";
    }
    if status == "at_risk" {
        return "At Risk";
    }
    if hard_due_at.is_some_and(|due| due < chrono::Utc::now()) {
        return "Broken";
    }
    let is_stale = risk_signals
        .iter()
        .any(|signal| signal["signal"] == "stale");
    if is_stale {
        return "Stalled";
    }
    "Healthy"
}

/// Ranks non-closed obligations by urgency and states a plain, deterministic
/// reason for each (ADR-0022): at-risk first, then soonest hard_due_at, then
/// soonest soft_due_at, then most-recently-updated. Read-only; a plain SQL
/// ORDER BY, not a scoring model. Joins read-only against `source_fragments`
/// for evidence (ADR-0023). Also attaches `risk_signals` (ADR-0041/ADR-0046).
pub(super) async fn daily_brief(
    State(pool): State<PgPool>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
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
        .map(
            |(
                obligation_id,
                status,
                updated_at,
                hard_due_at,
                soft_due_at,
                source_fragment_id,
                source_text,
                has_owner,
                has_edges,
            )| {
                let reason =
                    daily_brief_reason(&status, hard_due_at, soft_due_at, source_text.as_deref());
                let signals = risk_signals(
                    hard_due_at,
                    soft_due_at,
                    updated_at,
                    source_fragment_id,
                    has_owner,
                    has_edges,
                );
                json!({
                    "obligation_id": obligation_id,
                    "status": &status,
                    "updated_at": updated_at.to_rfc3339(),
                    "hard_due_at": hard_due_at.map(|value| value.to_rfc3339()),
                    "soft_due_at": soft_due_at.map(|value| value.to_rfc3339()),
                    "source_fragment_id": source_fragment_id,
                    "source_text": source_text,
                    "reason": reason,
                    "health": obligation_health(&status, hard_due_at, &signals),
                    "risk_signals": signals,
                })
            },
        )
        .collect::<Vec<_>>();

    Ok(Json(json!(body)))
}

const TIME_HORIZON_BUCKETS: [&str; 5] = [
    "overdue",
    "next_7_days",
    "next_30_days",
    "next_90_days",
    "beyond",
];

/// Buckets one Obligation by its effective due date (ADR-0029): hard_due_at
/// if present, else soft_due_at, else none. An at_risk Obligation with no
/// date at all lands in "overdue" (the one exception to pure date
/// bucketing); every other combination buckets purely by date.
fn time_horizon_bucket(
    status: &str,
    hard_due_at: Option<chrono::DateTime<chrono::Utc>>,
    soft_due_at: Option<chrono::DateTime<chrono::Utc>>,
) -> &'static str {
    let effective_due_at = hard_due_at.or(soft_due_at);
    let Some(due) = effective_due_at else {
        return if status == "at_risk" {
            "overdue"
        } else {
            "beyond"
        };
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
pub(super) async fn time_horizon(
    State(pool): State<PgPool>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
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

    let mut buckets: std::collections::HashMap<&'static str, Vec<JsonValue>> =
        std::collections::HashMap::new();
    for (
        obligation_id,
        status,
        updated_at,
        hard_due_at,
        soft_due_at,
        source_fragment_id,
        source_text,
        has_owner,
        has_edges,
    ) in rows
    {
        let bucket = time_horizon_bucket(&status, hard_due_at, soft_due_at);
        let reason = daily_brief_reason(&status, hard_due_at, soft_due_at, source_text.as_deref());
        let signals = risk_signals(
            hard_due_at,
            soft_due_at,
            updated_at,
            source_fragment_id,
            has_owner,
            has_edges,
        );
        buckets.entry(bucket).or_default().push(json!({
            "obligation_id": obligation_id,
            "status": &status,
            "updated_at": updated_at.to_rfc3339(),
            "hard_due_at": hard_due_at.map(|value| value.to_rfc3339()),
            "soft_due_at": soft_due_at.map(|value| value.to_rfc3339()),
            "source_fragment_id": source_fragment_id,
            "reason": reason,
            "health": obligation_health(&status, hard_due_at, &signals),
            "risk_signals": signals,
        }));
    }

    let body: JsonValue = TIME_HORIZON_BUCKETS
        .into_iter()
        .filter_map(|bucket| {
            buckets
                .remove(bucket)
                .map(|items| (bucket.to_string(), json!(items)))
        })
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
pub(super) async fn focus_blocks(
    State(pool): State<PgPool>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
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

    let mut blocks: std::collections::HashMap<(Uuid, &'static str), Block> =
        std::collections::HashMap::new();
    for row in rows {
        let bucket = time_horizon_bucket(&row.status, row.hard_due_at, row.soft_due_at);
        let reason = daily_brief_reason(
            &row.status,
            row.hard_due_at,
            row.soft_due_at,
            row.source_text.as_deref(),
        );
        let effective_due = row.hard_due_at.or(row.soft_due_at);
        let entry = blocks
            .entry((row.node_id, bucket))
            .or_insert_with(|| Block {
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

    let mut result: Vec<(
        bool,
        Option<chrono::DateTime<chrono::Utc>>,
        usize,
        JsonValue,
    )> = blocks
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

    Ok(Json(json!(result
        .into_iter()
        .map(|(_, _, _, value)| value)
        .collect::<Vec<_>>())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{app, test_pool};
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    /// ADR-0041: risk_signals is a pure function -- no database, no
    /// flakiness -- covering both signals independently and together.
    #[test]
    fn risk_signals_flags_date_compression_when_due_soon_with_no_evidence() {
        let due = chrono::Utc::now() + chrono::Duration::days(3);
        let signals = risk_signals(Some(due), None, chrono::Utc::now(), None, true, true);
        assert!(signals
            .iter()
            .any(|signal| signal["signal"] == "date_compression"));
    }

    #[test]
    fn risk_signals_does_not_flag_date_compression_when_evidence_is_linked() {
        let due = chrono::Utc::now() + chrono::Duration::days(3);
        let signals = risk_signals(
            Some(due),
            None,
            chrono::Utc::now(),
            Some(uuid::Uuid::new_v4()),
            true,
            true,
        );
        assert!(signals
            .iter()
            .all(|signal| signal["signal"] != "date_compression"));
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
        let signals = risk_signals(
            None,
            None,
            updated_at,
            Some(uuid::Uuid::new_v4()),
            true,
            true,
        );
        assert!(signals.iter().any(|signal| signal["signal"] == "stale"));
    }

    #[test]
    fn risk_signals_does_not_flag_stale_within_threshold() {
        let updated_at = chrono::Utc::now() - chrono::Duration::days(2);
        let signals = risk_signals(
            None,
            None,
            updated_at,
            Some(uuid::Uuid::new_v4()),
            true,
            true,
        );
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
        let signals = risk_signals(
            None,
            None,
            chrono::Utc::now(),
            Some(uuid::Uuid::new_v4()),
            false,
            true,
        );
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0]["signal"], "unowned");
    }

    #[test]
    fn risk_signals_does_not_flag_unowned_when_has_owner_is_true() {
        let signals = risk_signals(
            None,
            None,
            chrono::Utc::now(),
            Some(uuid::Uuid::new_v4()),
            true,
            true,
        );
        assert!(signals.is_empty());
    }

    /// ADR-0054 (Congruence Engine v1): isolated is independent of the
    /// other three signals -- no due date, recently updated, evidence
    /// linked, has an owner, just zero edges at all.
    #[test]
    fn risk_signals_flags_isolated_when_has_edges_is_false() {
        let signals = risk_signals(
            None,
            None,
            chrono::Utc::now(),
            Some(uuid::Uuid::new_v4()),
            true,
            false,
        );
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0]["signal"], "isolated");
    }

    #[test]
    fn risk_signals_does_not_flag_isolated_when_has_edges_is_true() {
        let signals = risk_signals(
            None,
            None,
            chrono::Utc::now(),
            Some(uuid::Uuid::new_v4()),
            true,
            true,
        );
        assert!(signals.is_empty());
    }

    /// ADR-0061: closed always reads Completed, regardless of dates or signals.
    #[test]
    fn obligation_health_returns_completed_for_closed_status() {
        let due = chrono::Utc::now() - chrono::Duration::days(30);
        assert_eq!(obligation_health("closed", Some(due), &[]), "Completed");
    }

    /// ADR-0061: at_risk always reads At Risk, taking priority over a
    /// stale signal that might otherwise read as Stalled.
    #[test]
    fn obligation_health_returns_at_risk_for_at_risk_status() {
        let signals = vec![json!({ "signal": "stale", "explanation": "..." })];
        assert_eq!(obligation_health("at_risk", None, &signals), "At Risk");
    }

    /// ADR-0061's own named distinction: an overdue, still-open Obligation
    /// with no stale signal is Broken, not Stalled -- the two must not be
    /// conflated even though both are "open and unhealthy".
    #[test]
    fn obligation_health_distinguishes_broken_from_stalled() {
        let overdue = chrono::Utc::now() - chrono::Duration::days(1);
        assert_eq!(obligation_health("open", Some(overdue), &[]), "Broken");

        let stale_signals = vec![json!({ "signal": "stale", "explanation": "..." })];
        assert_eq!(obligation_health("open", None, &stale_signals), "Stalled");
    }

    /// ADR-0061: open, not overdue, not stale reads Healthy -- the only
    /// remaining case of the five fixed values.
    #[test]
    fn obligation_health_returns_healthy_for_an_ordinary_open_obligation() {
        let future = chrono::Utc::now() + chrono::Duration::days(30);
        assert_eq!(obligation_health("open", Some(future), &[]), "Healthy");
        assert_eq!(obligation_health("open", None, &[]), "Healthy");
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
        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/obligations")
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
            "must include the just-appended obligation"
        );
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
        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/obligations")
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
            .find(|row| row["obligation_id"] == obligation_id.to_string())
            .expect("the just-created obligation must be present");
        assert_eq!(row["hard_due_at"], "2026-09-01T00:00:00+00:00");
        assert!(
            row["soft_due_at"].is_null(),
            "an unset soft_due_at must serialize as null, not be omitted"
        );
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
        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/obligations")
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
            .find(|row| row["obligation_id"] == obligation_id.to_string())
            .expect("the just-created obligation must be present");
        assert_eq!(row["source_fragment_id"], fragment_id.to_string());
        assert_eq!(
            row["source_text"],
            "We committed to a two-week transition plan."
        );
    }

    /// ADR-0059: `?limit=`/`?offset=` page GET /api/obligations; omitting
    /// both keeps returning every row.
    #[tokio::test]
    async fn obligations_route_applies_limit_and_offset() {
        let pool = test_pool().await;
        for _ in 0..2 {
            crate::obligation::append_event(
                &pool,
                uuid::Uuid::new_v4(),
                crate::obligation::ObligationEventType::Created,
                json!({"status": "open"}),
            )
            .await
            .expect("append created event");
        }
        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/obligations?limit=1")
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
        crate::obligation::append_event(
            &pool,
            without_evidence,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open"}),
        )
        .await
        .expect("append obligation without evidence");

        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/daily-brief")
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
        let rows = parsed.as_array().unwrap();
        let with_row = rows
            .iter()
            .find(|row| row["obligation_id"] == with_evidence.to_string())
            .expect("linked obligation present");
        let without_row = rows
            .iter()
            .find(|row| row["obligation_id"] == without_evidence.to_string())
            .expect("unlinked obligation present");
        assert_eq!(
            with_row["reason"],
            "No due date recorded. Last evidence: \"We committed to a two-week transition plan.\"."
        );
        assert_eq!(
            without_row["reason"],
            "No due date recorded. No evidence recorded."
        );
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
        crate::obligation::append_event(
            &pool,
            closed,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open"}),
        )
        .await
        .expect("append obligation to be closed");
        crate::obligation::append_event(
            &pool,
            closed,
            crate::obligation::ObligationEventType::Closed,
            json!({}),
        )
        .await
        .expect("close it");

        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/daily-brief")
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
        let rows = parsed.as_array().unwrap();

        assert!(
            rows.iter()
                .all(|row| row["obligation_id"] != closed.to_string()),
            "a closed obligation must never appear in the Daily Brief"
        );

        let at_risk_index = rows
            .iter()
            .position(|row| row["obligation_id"] == at_risk_no_date.to_string());
        let far_future_index = rows
            .iter()
            .position(|row| row["obligation_id"] == far_future_open.to_string());
        assert!(
            at_risk_index.is_some() && far_future_index.is_some(),
            "both open items must be present"
        );
        assert!(
            at_risk_index.unwrap() < far_future_index.unwrap(),
            "at_risk must outrank an open obligation with a due date, however distant"
        );
        assert_eq!(
            rows[at_risk_index.unwrap()]["reason"],
            "Marked at risk. No evidence recorded."
        );
        // ADR-0061: health is attached alongside risk_signals on this same route.
        assert_eq!(rows[at_risk_index.unwrap()]["health"], "At Risk");
        assert_eq!(rows[far_future_index.unwrap()]["health"], "Healthy");
    }

    /// ADR-0023: the reason cites the linked source fragment's text, or
    /// states plainly that none is recorded.
    #[tokio::test]
    async fn daily_brief_reason_cites_linked_evidence() {
        let pool = test_pool().await;
        let meeting_id = uuid::Uuid::new_v4();
        let fragment_id = graph::create_source_fragment(
            &pool,
            meeting_id,
            "Roopa: please send the transition plan.",
            "brief-evidence-hash",
        )
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

        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/daily-brief")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let rows = parsed.as_array().unwrap();

        let evidenced_row = rows
            .iter()
            .find(|row| row["obligation_id"] == with_evidence.to_string())
            .expect("present");
        assert_eq!(
            evidenced_row["reason"],
            "Marked at risk. Last evidence: \"Roopa: please send the transition plan.\"."
        );
        assert_eq!(evidenced_row["source_fragment_id"], fragment_id.to_string());

        let unevidenced_row = rows
            .iter()
            .find(|row| row["obligation_id"] == without_evidence.to_string())
            .expect("present");
        assert_eq!(
            unevidenced_row["reason"],
            "Marked at risk. No evidence recorded."
        );
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

        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/daily-brief")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let row = parsed
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["obligation_id"] == compressed.to_string())
            .expect("the just-created obligation must be present");
        let signals = row["risk_signals"]
            .as_array()
            .expect("risk_signals must be an array");
        assert!(signals
            .iter()
            .any(|signal| signal["signal"] == "date_compression"));
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
        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/daily-brief")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let row = parsed
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["obligation_id"] == obligation_id.to_string())
            .expect("the just-created obligation must be present");
        assert_eq!(
            row["source_text"],
            "We committed to a two-week transition plan."
        );
    }

    /// ADR-0046: an Obligation with an `owns` edge from a person is never
    /// flagged unowned; one without any such edge is.
    #[tokio::test]
    async fn daily_brief_flags_an_obligation_with_no_owns_edge_as_unowned() {
        let pool = test_pool().await;

        let owned = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            owned,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open"}),
        )
        .await
        .expect("append owned obligation");
        let unowned = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            unowned,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open"}),
        )
        .await
        .expect("append unowned obligation");
        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let person_id = graph::create_node(&pool, "person", "Owner Signal Test Person", json!({}))
            .await
            .expect("create person");
        graph::create_edge(&pool, person_id, owned, "owns", None)
            .await
            .expect("link person as owner");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri("/api/daily-brief")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let rows = parsed.as_array().unwrap();

        let owned_row = rows
            .iter()
            .find(|row| row["obligation_id"] == owned.to_string())
            .expect("present");
        let owned_signals = owned_row["risk_signals"].as_array().unwrap();
        assert!(
            owned_signals
                .iter()
                .all(|signal| signal["signal"] != "unowned"),
            "an owned obligation must not be flagged unowned"
        );

        let unowned_row = rows
            .iter()
            .find(|row| row["obligation_id"] == unowned.to_string())
            .expect("present");
        let unowned_signals = unowned_row["risk_signals"].as_array().unwrap();
        assert!(
            unowned_signals
                .iter()
                .any(|signal| signal["signal"] == "unowned"),
            "an obligation with no owns edge must be flagged unowned"
        );
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
        crate::obligation::append_event(
            &pool,
            at_risk_no_date_id,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "at_risk"}),
        )
        .await
        .expect("append at-risk obligation with no date");

        let closed_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            closed_id,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open"}),
        )
        .await
        .expect("append obligation to close");
        crate::obligation::append_event(
            &pool,
            closed_id,
            crate::obligation::ObligationEventType::Closed,
            json!({}),
        )
        .await
        .expect("close it");

        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri("/api/time-horizon")
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

        let overdue = parsed["overdue"]
            .as_array()
            .expect("overdue bucket present");
        assert!(
            overdue
                .iter()
                .any(|row| row["obligation_id"] == overdue_id.to_string()),
            "a past-due obligation must land in overdue"
        );
        assert!(
            overdue
                .iter()
                .any(|row| row["obligation_id"] == at_risk_no_date_id.to_string()),
            "an at_risk obligation with no date must land in overdue, not beyond"
        );

        let next_7 = parsed["next_7_days"]
            .as_array()
            .expect("next_7_days bucket present");
        assert!(next_7
            .iter()
            .any(|row| row["obligation_id"] == next_7_id.to_string()));

        for bucket in [
            "overdue",
            "next_7_days",
            "next_30_days",
            "next_90_days",
            "beyond",
        ] {
            if let Some(items) = parsed.get(bucket).and_then(|value| value.as_array()) {
                assert!(
                    items
                        .iter()
                        .all(|row| row["obligation_id"] != closed_id.to_string()),
                    "a closed obligation must never appear in any bucket"
                );
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
        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let person_id =
            graph::create_node(&pool, "person", "Obligation Detail Test Person", json!({}))
                .await
                .expect("create person node");
        graph::create_edge(&pool, person_id, obligation_id, "owns", None)
            .await
            .expect("link owner");
        let unresolved_id = uuid::Uuid::new_v4();
        graph::create_edge(&pool, obligation_id, unresolved_id, "blocks", None)
            .await
            .expect("link an edge to a non-node id");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/obligations/{obligation_id}"))
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

        assert_eq!(parsed["obligation_id"], obligation_id.to_string());
        let signals = parsed["risk_signals"]
            .as_array()
            .expect("risk_signals is an array");
        assert!(signals
            .iter()
            .any(|signal| signal["signal"] == "date_compression"));
        assert!(
            !signals.iter().any(|signal| signal["signal"] == "unowned"),
            "an owned obligation must not be flagged unowned"
        );

        let linked = parsed["linked_nodes"]
            .as_array()
            .expect("linked_nodes is an array");
        assert!(linked
            .iter()
            .any(|node| node["node_id"] == person_id.to_string() && node["edge_type"] == "owns"));
        let unresolved_link = linked
            .iter()
            .find(|node| node["edge_type"] == "blocks")
            .expect("the edge to a non-node id must still appear");
        assert!(
            unresolved_link["node_id"].is_null(),
            "an edge into a non-node id must report a null neighbor, not error"
        );
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

        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/time-horizon")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let row = parsed["overdue"]
            .as_array()
            .expect("overdue bucket must be present")
            .iter()
            .find(|row| row["obligation_id"] == overdue_no_evidence.to_string())
            .expect("the just-created obligation must be present in overdue");
        let signals = row["risk_signals"]
            .as_array()
            .expect("risk_signals must be an array");
        assert!(signals
            .iter()
            .any(|signal| signal["signal"] == "date_compression"));
        // ADR-0061: health is attached alongside risk_signals here too --
        // overdue, open, freshly-created (not stale) reads Broken.
        assert_eq!(row["health"], "Broken");
    }

    /// ADR-0046: mirrors the Daily Brief's own unowned proof, scoped to the
    /// Time Horizon's bucketed response shape.
    #[tokio::test]
    async fn time_horizon_flags_an_obligation_with_no_owns_edge_as_unowned() {
        let pool = test_pool().await;

        let owned = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            owned,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open"}),
        )
        .await
        .expect("append owned obligation");
        let unowned = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            unowned,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open"}),
        )
        .await
        .expect("append unowned obligation");
        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let person_id = graph::create_node(
            &pool,
            "person",
            "Time Horizon Owner Signal Test Person",
            json!({}),
        )
        .await
        .expect("create person");
        graph::create_edge(&pool, person_id, owned, "owns", None)
            .await
            .expect("link person as owner");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri("/api/time-horizon")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let beyond = parsed["beyond"]
            .as_array()
            .expect("beyond bucket must be present -- no date recorded");

        let owned_row = beyond
            .iter()
            .find(|row| row["obligation_id"] == owned.to_string())
            .expect("present");
        let owned_signals = owned_row["risk_signals"].as_array().unwrap();
        assert!(
            owned_signals
                .iter()
                .all(|signal| signal["signal"] != "unowned"),
            "an owned obligation must not be flagged unowned"
        );

        let unowned_row = beyond
            .iter()
            .find(|row| row["obligation_id"] == unowned.to_string())
            .expect("present");
        let unowned_signals = unowned_row["risk_signals"].as_array().unwrap();
        assert!(
            unowned_signals
                .iter()
                .any(|signal| signal["signal"] == "unowned"),
            "an obligation with no owns edge must be flagged unowned"
        );
    }
    /// (reusing daily_brief_reason verbatim) are present.
    #[tokio::test]
    async fn focus_blocks_route_groups_by_shared_node() {
        let pool = test_pool().await;
        let person_id = graph::create_node(&pool, "person", "Roopa", json!({}))
            .await
            .expect("create person node");
        let due_soon = (chrono::Utc::now() + chrono::Duration::days(3)).to_rfc3339();

        let obligation_a = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            obligation_a,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open", "hard_due_at": due_soon}),
        )
        .await
        .expect("append obligation a");
        let obligation_b = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            obligation_b,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "at_risk", "hard_due_at": due_soon}),
        )
        .await
        .expect("append obligation b");
        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        graph::create_edge(&pool, person_id, obligation_a, "owns", None)
            .await
            .expect("link obligation a");
        graph::create_edge(&pool, obligation_b, person_id, "owns", None)
            .await
            .expect("link obligation b");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri("/api/focus-blocks")
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
        let blocks = parsed.as_array().expect("response is a json array");
        let block = blocks
            .iter()
            .find(|block| block["node_id"] == person_id.to_string())
            .expect("a block for the shared person node and shared bucket must exist");
        assert_eq!(block["node_type"], "person");
        assert_eq!(block["canonical_text"], "Roopa");
        assert_eq!(block["time_horizon_bucket"], "next_7_days");
        let obligations = block["obligations"]
            .as_array()
            .expect("obligations is an array");
        assert_eq!(obligations.len(), 2);
        assert!(obligations
            .iter()
            .any(|o| o["obligation_id"] == obligation_a.to_string()));
        assert!(obligations
            .iter()
            .any(|o| o["obligation_id"] == obligation_b.to_string()
                && o["reason"] == "Marked at risk. No evidence recorded."));
    }

    /// ADR-0052: a shared node whose Obligations span two Time Horizon
    /// buckets forms one block per bucket, not one block spanning both.
    #[tokio::test]
    async fn focus_blocks_route_splits_by_time_horizon_bucket() {
        let pool = test_pool().await;
        let person_id = graph::create_node(&pool, "person", "Bucket Split Test Person", json!({}))
            .await
            .expect("create person node");
        let due_soon = (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339();
        let due_later = (chrono::Utc::now() + chrono::Duration::days(60)).to_rfc3339();

        let mut soon_ids = Vec::new();
        for _ in 0..2 {
            let id = uuid::Uuid::new_v4();
            crate::obligation::append_event(
                &pool,
                id,
                crate::obligation::ObligationEventType::Created,
                json!({"status": "open", "hard_due_at": due_soon}),
            )
            .await
            .expect("append due-soon obligation");
            graph::create_edge(&pool, person_id, id, "owns", None)
                .await
                .expect("link due-soon obligation");
            soon_ids.push(id);
        }
        let mut later_ids = Vec::new();
        for _ in 0..2 {
            let id = uuid::Uuid::new_v4();
            crate::obligation::append_event(
                &pool,
                id,
                crate::obligation::ObligationEventType::Created,
                json!({"status": "open", "hard_due_at": due_later}),
            )
            .await
            .expect("append due-later obligation");
            graph::create_edge(&pool, person_id, id, "owns", None)
                .await
                .expect("link due-later obligation");
            later_ids.push(id);
        }
        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri("/api/focus-blocks")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let blocks: Vec<&JsonValue> = parsed
            .as_array()
            .expect("response is a json array")
            .iter()
            .filter(|block| block["node_id"] == person_id.to_string())
            .collect();
        assert_eq!(
            blocks.len(),
            2,
            "the shared node's Obligations must form two blocks, one per bucket"
        );

        let soon_block = blocks
            .iter()
            .find(|block| block["time_horizon_bucket"] == "next_7_days")
            .expect("a next_7_days block must exist");
        let soon_obligations = soon_block["obligations"].as_array().unwrap();
        assert_eq!(soon_obligations.len(), 2);
        assert!(soon_ids.iter().all(|id| soon_obligations
            .iter()
            .any(|o| o["obligation_id"] == id.to_string())));

        let later_block = blocks
            .iter()
            .find(|block| block["time_horizon_bucket"] == "next_90_days")
            .expect("a next_90_days block must exist");
        let later_obligations = later_block["obligations"].as_array().unwrap();
        assert_eq!(later_obligations.len(), 2);
        assert!(later_ids.iter().all(|id| later_obligations
            .iter()
            .any(|o| o["obligation_id"] == id.to_string())));
    }

    #[tokio::test]
    async fn focus_blocks_route_forms_no_block_for_a_single_linked_obligation() {
        let pool = test_pool().await;
        let meeting_id = graph::create_node(&pool, "meeting", "Weekly sync", json!({}))
            .await
            .expect("create meeting node");

        let obligation_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            obligation_id,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open"}),
        )
        .await
        .expect("append obligation");
        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");
        graph::create_edge(&pool, meeting_id, obligation_id, "discussed", None)
            .await
            .expect("link obligation");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri("/api/focus-blocks")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let blocks = parsed.as_array().expect("response is a json array");
        assert!(
            blocks
                .iter()
                .all(|block| block["node_id"] != meeting_id.to_string()),
            "a node linked to only one non-closed obligation must form no block"
        );
    }

    #[tokio::test]
    async fn focus_blocks_route_excludes_a_closed_obligation() {
        let pool = test_pool().await;
        let person_id = graph::create_node(&pool, "person", "Closed Test Person", json!({}))
            .await
            .expect("create person node");

        let open_a = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            open_a,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open"}),
        )
        .await
        .expect("append open obligation a");
        let open_b = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            open_b,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open"}),
        )
        .await
        .expect("append open obligation b");
        let closed_id = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            closed_id,
            crate::obligation::ObligationEventType::Created,
            json!({"status": "open"}),
        )
        .await
        .expect("append obligation to close");
        crate::obligation::append_event(
            &pool,
            closed_id,
            crate::obligation::ObligationEventType::Closed,
            json!({}),
        )
        .await
        .expect("close it");
        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        graph::create_edge(&pool, person_id, open_a, "owns", None)
            .await
            .expect("link open obligation a");
        graph::create_edge(&pool, person_id, open_b, "owns", None)
            .await
            .expect("link open obligation b");
        graph::create_edge(&pool, person_id, closed_id, "owns", None)
            .await
            .expect("link closed obligation");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri("/api/focus-blocks")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let blocks = parsed.as_array().expect("response is a json array");
        let block = blocks
            .iter()
            .find(|block| block["node_id"] == person_id.to_string())
            .expect("a block must still form from the two open obligations");
        let obligations = block["obligations"]
            .as_array()
            .expect("obligations is an array");
        assert_eq!(
            obligations.len(),
            2,
            "the closed obligation must not be counted"
        );
        assert!(
            obligations
                .iter()
                .all(|o| o["obligation_id"] != closed_id.to_string()),
            "the closed obligation must never appear in a block"
        );
    }
}
