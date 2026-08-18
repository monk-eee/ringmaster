#[cfg(test)]
use crate::extraction;
use crate::graph;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value as JsonValue};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub(super) struct IngestMeetingRequest {
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
pub(super) async fn ingest_meeting(
    State(pool): State<PgPool>,
    Json(body): Json<IngestMeetingRequest>,
) -> Result<Response, (axum::http::StatusCode, String)> {
    if body.title.trim().is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "title must not be blank".to_string(),
        ));
    }
    if body.transcript.trim().is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "transcript must not be blank".to_string(),
        ));
    }
    let occurred_at = body
        .occurred_at
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or((
            axum::http::StatusCode::BAD_REQUEST,
            "occurred_at must not be blank".to_string(),
        ))?;
    let occurred_at = chrono::DateTime::parse_from_rfc3339(occurred_at)
        .map(|value| value.with_timezone(&chrono::Utc))
        .map_err(|_| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                "occurred_at must be a valid RFC3339 datetime".to_string(),
            )
        })?;

    let metadata = crate::transcript::MeetingMetadata {
        title: body.title,
        occurred_at: Some(occurred_at),
        organiser: body.organiser,
        participants: body.participants,
    };

    let ingested = crate::transcript::ingest_transcript(&pool, &metadata, &body.transcript)
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
            "meeting_id": ingested.meeting_id,
            "fragment_ids": ingested.fragment_ids,
        })),
    )
        .into_response())
}

#[derive(Debug, Deserialize)]
pub(super) struct IngestSourceRequest {
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
pub(super) async fn ingest_source_route(
    State(pool): State<PgPool>,
    Json(body): Json<IngestSourceRequest>,
) -> Result<Response, (axum::http::StatusCode, String)> {
    if body.source_type.trim().is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "source_type must not be blank".to_string(),
        ));
    }
    if body.title.trim().is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "title must not be blank".to_string(),
        ));
    }
    if body.text.trim().is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "text must not be blank".to_string(),
        ));
    }
    let occurred_at = body
        .occurred_at
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or((
            axum::http::StatusCode::BAD_REQUEST,
            "occurred_at must not be blank".to_string(),
        ))?;
    let occurred_at = chrono::DateTime::parse_from_rfc3339(occurred_at)
        .map(|value| value.with_timezone(&chrono::Utc))
        .map_err(|_| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                "occurred_at must be a valid RFC3339 datetime".to_string(),
            )
        })?;

    let metadata = crate::transcript::SourceMetadata {
        source_type: body.source_type,
        title: body.title,
        occurred_at,
        participants: body.participants,
    };

    let ingested = crate::transcript::ingest_source(&pool, &metadata, &body.text)
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
            "node_id": ingested.node_id,
            "fragment_ids": ingested.fragment_ids,
        })),
    )
        .into_response())
}

/// Reads one meeting and its transcript fragments in turn order (ADR-0036).
/// 404s for an unknown id or a node that isn't a meeting -- this route's
/// contract is specifically a meeting, not any node type. Read-only.
pub(super) async fn get_meeting_detail(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let node = graph::get_node(&pool, id)
        .await
        .map_err(|error| match error {
            sqlx::Error::RowNotFound => (
                axum::http::StatusCode::NOT_FOUND,
                "meeting not found".to_string(),
            ),
            other => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                other.to_string(),
            ),
        })?;
    if node.node_type != "meeting" {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "meeting not found".to_string(),
        ));
    }

    let fragments = graph::list_source_fragments_by_meeting(&pool, id)
        .await
        .map_err(|error| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                error.to_string(),
            )
        })?;

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
pub(super) async fn get_meeting_candidates(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<JsonValue>, (axum::http::StatusCode, String)> {
    let node = graph::get_node(&pool, id)
        .await
        .map_err(|error| match error {
            sqlx::Error::RowNotFound => (
                axum::http::StatusCode::NOT_FOUND,
                "meeting not found".to_string(),
            ),
            other => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                other.to_string(),
            ),
        })?;
    if node.node_type != "meeting" {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "meeting not found".to_string(),
        ));
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
    #[allow(clippy::type_complexity)]
    let mut fragment_data: std::collections::HashMap<
        Uuid,
        (Option<i32>, Option<String>, String, Vec<JsonValue>),
    > = std::collections::HashMap::new();
    let mut by_validation_state: std::collections::BTreeMap<String, i64> =
        std::collections::BTreeMap::new();

    for row in &rows {
        let entry = fragment_data.entry(row.fragment_id).or_insert_with(|| {
            fragment_order.push(row.fragment_id);
            (
                row.sequence,
                row.speaker.clone(),
                row.fragment_text.clone(),
                Vec::new(),
            )
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
    let extracted_fragment_count = fragment_order
        .iter()
        .filter(|fragment_id| !fragment_data[fragment_id].3.is_empty())
        .count() as i64;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{app, test_pool};
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

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

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let meeting_id = parsed["meeting_id"]
            .as_str()
            .expect("meeting_id present")
            .to_string();
        let fragment_ids = parsed["fragment_ids"]
            .as_array()
            .expect("fragment_ids is an array");
        assert_eq!(fragment_ids.len(), 2, "one fragment per transcript turn");

        let node = graph::get_node(&pool, uuid::Uuid::parse_str(&meeting_id).unwrap())
            .await
            .expect("meeting node exists");
        assert_eq!(node.node_type, "meeting");
        assert_eq!(node.canonical_text, "Weekly 1:1");

        let first_fragment_id = uuid::Uuid::parse_str(fragment_ids[0].as_str().unwrap()).unwrap();
        let first_fragment = graph::get_source_fragment(&pool, first_fragment_id)
            .await
            .expect("first fragment exists");
        assert_eq!(first_fragment.text, "Please bring me a transition plan.");

        let second_fragment_id = uuid::Uuid::parse_str(fragment_ids[1].as_str().unwrap()).unwrap();
        let second_fragment = graph::get_source_fragment(&pool, second_fragment_id)
            .await
            .expect("second fragment exists");
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

        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM source_fragments WHERE text = $1")
                .bind(&marker)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            count, 0,
            "a blank title must perform zero writes, including zero fragments"
        );
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

        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM nodes WHERE node_type = 'meeting' AND canonical_text = $1",
        )
        .bind(&marker)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            count, 0,
            "a blank transcript must perform zero writes, including zero Meeting nodes"
        );
    }

    /// ADR-0040: occurred_at is the one new hard requirement; missing or
    /// blank must reject with 400 and perform zero writes, matching the
    /// title/transcript blank-check posture exactly.
    #[tokio::test]
    async fn ingest_meeting_route_rejects_a_missing_occurred_at_with_no_writes() {
        let pool = test_pool().await;
        let marker = format!("missing-occurred-at-marker-{}", uuid::Uuid::new_v4());
        let request_body =
            json!({"title": "Missing Occurred At Test", "transcript": format!("Roopa: {marker}")});
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

        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM source_fragments WHERE text = $1")
                .bind(&marker)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            count, 0,
            "a missing occurred_at must perform zero writes, including zero fragments"
        );
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

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: JsonValue = serde_json::from_slice(&body).expect("valid json body");
        let node_id = parsed["node_id"]
            .as_str()
            .expect("node_id present")
            .to_string();
        let fragment_ids = parsed["fragment_ids"]
            .as_array()
            .expect("fragment_ids is an array");
        assert_eq!(fragment_ids.len(), 2, "one fragment per paragraph");

        let node = graph::get_node(&pool, uuid::Uuid::parse_str(&node_id).unwrap())
            .await
            .expect("node exists");
        assert_eq!(node.node_type, "email");
        assert_eq!(node.canonical_text, "Re: transition plan");

        let (stored_occurred_at,): (Option<chrono::DateTime<chrono::Utc>>,) =
            sqlx::query_as("SELECT occurred_at FROM nodes WHERE id = $1")
                .bind(uuid::Uuid::parse_str(&node_id).unwrap())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            stored_occurred_at,
            Some(
                chrono::DateTime::parse_from_rfc3339("2026-08-01T09:00:00Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc)
            )
        );
    }

    #[tokio::test]
    async fn ingest_source_route_rejects_a_missing_occurred_at_with_no_writes() {
        let pool = test_pool().await;
        let marker = format!("source-missing-occurred-at-{}", uuid::Uuid::new_v4());
        let request_body =
            json!({"source_type": "note", "title": "Missing Occurred At Test", "text": marker});
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

        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM source_fragments WHERE text = $1")
                .bind(&marker)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            count, 0,
            "a missing occurred_at must perform zero writes, including zero fragments"
        );
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
        assert_eq!(
            count, 0,
            "ingestion must never create a candidate on its own"
        );
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
        assert_eq!(
            count, 0,
            "ingestion must never create a candidate on its own"
        );
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
        let ingest_body = axum::body::to_bytes(ingest_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let ingested: JsonValue = serde_json::from_slice(&ingest_body).expect("valid json body");
        let meeting_id = ingested["meeting_id"]
            .as_str()
            .expect("meeting_id is a string");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/meetings/{meeting_id}"))
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

        assert_eq!(parsed["canonical_text"], "Ordered Fragments Test");
        let fragments = parsed["fragments"]
            .as_array()
            .expect("fragments is an array");
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
            .oneshot(
                Request::builder()
                    .uri(format!("/api/meetings/{unknown_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unknown_response.status(), axum::http::StatusCode::NOT_FOUND);

        let person_id = graph::create_node(&pool, "person", "Not A Meeting", json!({}))
            .await
            .expect("create person node");
        let wrong_type_response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/meetings/{person_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            wrong_type_response.status(),
            axum::http::StatusCode::NOT_FOUND
        );
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
            &crate::transcript::MeetingMetadata {
                title: "Candidates Progress Test".to_string(),
                occurred_at: None,
                organiser: None,
                participants: vec![],
            },
            "Roopa: please send me a transition plan.\nLyndon: sure, by Friday.",
        )
        .await
        .expect("ingest transcript");
        assert_eq!(ingested.fragment_ids.len(), 2);

        let candidate_id = uuid::Uuid::new_v4();
        extraction::extract_candidate(
            &pool,
            candidate_id,
            "request",
            "send a transition plan",
            ingested.fragment_ids[0],
            Some(0.8),
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
                    .uri(format!("/api/meetings/{}/candidates", ingested.meeting_id))
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

        let fragments = parsed["fragments"]
            .as_array()
            .expect("fragments is an array");
        assert_eq!(fragments.len(), 2);
        let extracted_fragment = fragments[0]["candidates"]
            .as_array()
            .expect("candidates is an array");
        assert_eq!(extracted_fragment.len(), 1);
        assert_eq!(
            extracted_fragment[0]["candidate_id"],
            candidate_id.to_string()
        );
        assert_eq!(extracted_fragment[0]["validation_state"], "candidate");
        let pending_fragment = fragments[1]["candidates"]
            .as_array()
            .expect("candidates is an array");
        assert_eq!(
            pending_fragment.len(),
            0,
            "a fragment with no candidate yet must still appear, with an empty array"
        );

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
            .oneshot(
                Request::builder()
                    .uri(format!("/api/meetings/{unknown_id}/candidates"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unknown_response.status(), axum::http::StatusCode::NOT_FOUND);

        let person_id = graph::create_node(&pool, "person", "Not A Meeting Either", json!({}))
            .await
            .expect("create person node");
        let wrong_type_response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/meetings/{person_id}/candidates"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            wrong_type_response.status(),
            axum::http::StatusCode::NOT_FOUND
        );
    }

    /// ADR-0037: this route only reads; calling it must never itself create
    /// a candidate for a never-extracted fragment.
    #[tokio::test]
    async fn meeting_candidates_route_never_triggers_extraction() {
        let pool = test_pool().await;
        let ingested = crate::transcript::ingest_transcript(
            &pool,
            &crate::transcript::MeetingMetadata {
                title: "No Extraction Test".to_string(),
                occurred_at: None,
                organiser: None,
                participants: vec![],
            },
            "Roopa: an unextracted turn.",
        )
        .await
        .expect("ingest transcript");

        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri(format!("/api/meetings/{}/candidates", ingested.meeting_id))
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

        assert_eq!(parsed["progress"]["extracted_fragment_count"], 0);
        assert_eq!(parsed["progress"]["pending_fragment_count"], 1);
        assert_eq!(
            parsed["fragments"][0]["candidates"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }
}
