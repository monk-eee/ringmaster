use serde_json::Value as Json;
use sqlx::{FromRow, PgPool};
use std::fmt;
use uuid::Uuid;

/// Obligation event vocabulary (ADR-0005/ADR-0007 scope: event types may
/// evolve without a new ADR as long as the event log stays authoritative).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObligationEventType {
    Created,
    StatusChanged,
    Closed,
}

impl ObligationEventType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::StatusChanged => "status_changed",
            Self::Closed => "closed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "created" => Some(Self::Created),
            "status_changed" => Some(Self::StatusChanged),
            "closed" => Some(Self::Closed),
            _ => None,
        }
    }
}

/// Obligation lifecycle status carried in a `created`/`status_changed` event payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObligationStatus {
    Open,
    AtRisk,
    Closed,
}

impl ObligationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::AtRisk => "at_risk",
            Self::Closed => "closed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "open" => Some(Self::Open),
            "at_risk" => Some(Self::AtRisk),
            "closed" => Some(Self::Closed),
            _ => None,
        }
    }
}

fn payload_status(payload: &Json) -> Option<ObligationStatus> {
    payload.get("status").and_then(|value| value.as_str()).and_then(ObligationStatus::parse)
}

/// Reads an optional due-date field from an event payload (ADR-0020).
/// `None` means the payload doesn't name this field at all -- callers must
/// preserve whatever was previously recorded. `Some(None)` means the field
/// is present but not a parseable timestamp -- callers clear it explicitly.
fn payload_timestamp(payload: &Json, key: &str) -> Option<Option<chrono::DateTime<chrono::Utc>>> {
    payload.get(key).map(|value| {
        value
            .as_str()
            .and_then(|text| chrono::DateTime::parse_from_rfc3339(text).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
    })
}

/// Reads an optional source_fragment_id field from an event payload
/// (ADR-0023), same carry-forward semantics as `payload_timestamp`.
fn payload_uuid(payload: &Json, key: &str) -> Option<Option<Uuid>> {
    payload.get(key).map(|value| value.as_str().and_then(|text| Uuid::parse_str(text).ok()))
}

/// Rejected before any row is written, so an invalid payload never reaches
/// the append-only event log.
#[derive(Debug)]
pub enum AppendEventError {
    InvalidPayload(String),
    Database(sqlx::Error),
}

impl fmt::Display for AppendEventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPayload(reason) => write!(f, "invalid obligation event payload: {reason}"),
            Self::Database(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for AppendEventError {}

impl From<sqlx::Error> for AppendEventError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct ObligationEvent {
    pub id: Uuid,
    pub obligation_id: Uuid,
    pub event_type: String,
    pub payload: Json,
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct ObligationProjection {
    pub obligation_id: Uuid,
    pub status: String,
    pub hard_due_at: Option<chrono::DateTime<chrono::Utc>>,
    pub soft_due_at: Option<chrono::DateTime<chrono::Utc>>,
    pub source_fragment_id: Option<Uuid>,
}

/// Appends one immutable event (ADR-0005/ADR-0007). The database rejects any
/// later mutation or deletion of the returned row. A `created` or
/// `status_changed` event without a recognized `status` payload is rejected
/// here, before it ever reaches the append-only log. Generic over the SQL
/// executor (ADR-0038) so a caller can append this event in the same
/// transaction as an audit row -- `&PgPool` still works unchanged.
pub async fn append_event<'e, E>(
    executor: E,
    obligation_id: Uuid,
    event_type: ObligationEventType,
    payload: Json,
) -> Result<Uuid, AppendEventError>
where
    E: sqlx::PgExecutor<'e>,
{
    let requires_status = matches!(
        event_type,
        ObligationEventType::Created | ObligationEventType::StatusChanged
    );
    if requires_status && payload_status(&payload).is_none() {
        return Err(AppendEventError::InvalidPayload(format!(
            "{} event requires a recognized \"status\" field",
            event_type.as_str()
        )));
    }

    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO obligation_events (obligation_id, event_type, payload) \
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(obligation_id)
    .bind(event_type.as_str())
    .bind(&payload)
    .fetch_one(executor)
    .await?;
    Ok(id)
}

/// Derives current obligation status from the full event log alone
/// (ADR-0005/ADR-0007). Always truncates and rewrites the projection; never
/// patches it in place, so a rebuild can never disagree with the event log
/// it was just built from.
pub async fn rebuild_projection(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let events: Vec<ObligationEvent> = sqlx::query_as(
        "SELECT id, obligation_id, event_type, payload FROM obligation_events \
         ORDER BY obligation_id, recorded_at, id",
    )
    .fetch_all(pool)
    .await?;

    struct ObligationState {
        status: ObligationStatus,
        hard_due_at: Option<chrono::DateTime<chrono::Utc>>,
        soft_due_at: Option<chrono::DateTime<chrono::Utc>>,
        source_fragment_id: Option<Uuid>,
    }

    let mut latest_status: std::collections::HashMap<Uuid, ObligationState> =
        std::collections::HashMap::new();
    for event in &events {
        match ObligationEventType::parse(&event.event_type) {
            Some(ObligationEventType::Created) | Some(ObligationEventType::StatusChanged) => {
                if let Some(status) = payload_status(&event.payload) {
                    let entry = latest_status.entry(event.obligation_id).or_insert(ObligationState {
                        status,
                        hard_due_at: None,
                        soft_due_at: None,
                        source_fragment_id: None,
                    });
                    entry.status = status;
                    if let Some(hard_due_at) = payload_timestamp(&event.payload, "hard_due_at") {
                        entry.hard_due_at = hard_due_at;
                    }
                    if let Some(soft_due_at) = payload_timestamp(&event.payload, "soft_due_at") {
                        entry.soft_due_at = soft_due_at;
                    }
                    if let Some(source_fragment_id) = payload_uuid(&event.payload, "source_fragment_id") {
                        entry.source_fragment_id = source_fragment_id;
                    }
                }
            }
            Some(ObligationEventType::Closed) => {
                if let Some(entry) = latest_status.get_mut(&event.obligation_id) {
                    entry.status = ObligationStatus::Closed;
                } else {
                    latest_status.insert(
                        event.obligation_id,
                        ObligationState {
                            status: ObligationStatus::Closed,
                            hard_due_at: None,
                            soft_due_at: None,
                            source_fragment_id: None,
                        },
                    );
                }
            }
            None => {}
        }
    }

    let mut tx = pool.begin().await?;
    sqlx::query("TRUNCATE obligation_projection").execute(&mut *tx).await?;
    let mut written = 0u64;
    for (obligation_id, state) in &latest_status {
        sqlx::query(
            "INSERT INTO obligation_projection (obligation_id, status, hard_due_at, soft_due_at, source_fragment_id) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(obligation_id)
        .bind(state.status.as_str())
        .bind(state.hard_due_at)
        .bind(state.soft_due_at)
        .bind(state.source_fragment_id)
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
    use serde_json::json;
    use sqlx::postgres::PgPoolOptions;

    async fn test_pool() -> PgPool {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to run obligation tests");
        crate::guard_test_database(&database_url);
        PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await
            .expect("connect to test database")
    }

    #[tokio::test]
    async fn projection_is_rebuilt_from_the_event_log_alone() {
        let pool = test_pool().await;
        let obligation_id = Uuid::new_v4();

        append_event(&pool, obligation_id, ObligationEventType::Created, json!({"status": "open"}))
            .await
            .expect("append created event");
        rebuild_projection(&pool).await.expect("rebuild after created");

        let after_created: ObligationProjection = sqlx::query_as(
            "SELECT obligation_id, status, hard_due_at, soft_due_at, source_fragment_id FROM obligation_projection WHERE obligation_id = $1",
        )
        .bind(obligation_id)
        .fetch_one(&pool)
        .await
        .expect("projection row after created");
        assert_eq!(after_created.status, "open");

        append_event(&pool, obligation_id, ObligationEventType::StatusChanged, json!({"status": "at_risk"}))
            .await
            .expect("append status_changed event");
        rebuild_projection(&pool).await.expect("rebuild after status change");

        let after_status_change: ObligationProjection = sqlx::query_as(
            "SELECT obligation_id, status, hard_due_at, soft_due_at, source_fragment_id FROM obligation_projection WHERE obligation_id = $1",
        )
        .bind(obligation_id)
        .fetch_one(&pool)
        .await
        .expect("projection row after status change");
        assert_eq!(
            after_status_change.status, "at_risk",
            "rebuild_projection must derive current state from the full event log, not an incremental mutation"
        );
    }

    #[tokio::test]
    async fn event_rows_cannot_be_mutated_or_deleted() {
        let pool = test_pool().await;
        let obligation_id = Uuid::new_v4();
        let event_id = append_event(&pool, obligation_id, ObligationEventType::Created, json!({"status": "open"}))
            .await
            .expect("append created event");

        let update_result = sqlx::query("UPDATE obligation_events SET event_type = 'tampered' WHERE id = $1")
            .bind(event_id)
            .execute(&pool)
            .await;
        assert!(update_result.is_err(), "UPDATE must be rejected by the append-only trigger");

        let delete_result = sqlx::query("DELETE FROM obligation_events WHERE id = $1")
            .bind(event_id)
            .execute(&pool)
            .await;
        assert!(delete_result.is_err(), "DELETE must be rejected by the append-only trigger");
    }

    #[tokio::test]
    async fn append_event_rejects_a_lifecycle_event_without_a_recognized_status() {
        let pool = test_pool().await;
        let obligation_id = Uuid::new_v4();

        let result = append_event(&pool, obligation_id, ObligationEventType::Created, json!({})).await;
        assert!(
            matches!(result, Err(AppendEventError::InvalidPayload(_))),
            "a created event with no status must be rejected before it reaches the log"
        );

        let rows: Vec<ObligationEvent> = sqlx::query_as(
            "SELECT id, obligation_id, event_type, payload FROM obligation_events WHERE obligation_id = $1",
        )
        .bind(obligation_id)
        .fetch_all(&pool)
        .await
        .expect("query obligation_events");
        assert!(rows.is_empty(), "a rejected payload must never be written to the append-only log");
    }

    #[tokio::test]
    async fn append_event_rejects_an_unrecognized_status_value() {
        let pool = test_pool().await;
        let obligation_id = Uuid::new_v4();

        let result = append_event(
            &pool,
            obligation_id,
            ObligationEventType::StatusChanged,
            json!({"status": "not_a_real_status"}),
        )
        .await;
        assert!(matches!(result, Err(AppendEventError::InvalidPayload(_))));
    }

    #[tokio::test]
    async fn closed_event_does_not_require_a_status_field() {
        let pool = test_pool().await;
        let obligation_id = Uuid::new_v4();

        append_event(&pool, obligation_id, ObligationEventType::Created, json!({"status": "open"}))
            .await
            .expect("append created event");
        append_event(&pool, obligation_id, ObligationEventType::Closed, json!({}))
            .await
            .expect("closed event needs no status field");
        rebuild_projection(&pool).await.expect("rebuild after closed");

        let projection: ObligationProjection = sqlx::query_as(
            "SELECT obligation_id, status, hard_due_at, soft_due_at, source_fragment_id FROM obligation_projection WHERE obligation_id = $1",
        )
        .bind(obligation_id)
        .fetch_one(&pool)
        .await
        .expect("projection row after closed");
        assert_eq!(projection.status, "closed");
    }

    /// ADR-0020: a status_changed event that doesn't name hard_due_at must
    /// not silently erase a previously-recorded one.
    #[tokio::test]
    async fn rebuild_preserves_a_due_date_across_an_event_that_does_not_name_it() {
        let pool = test_pool().await;
        let obligation_id = Uuid::new_v4();

        append_event(
            &pool,
            obligation_id,
            ObligationEventType::Created,
            json!({"status": "open", "hard_due_at": "2026-09-01T00:00:00Z"}),
        )
        .await
        .expect("append created event with a due date");
        append_event(&pool, obligation_id, ObligationEventType::StatusChanged, json!({"status": "at_risk"}))
            .await
            .expect("append status_changed event naming no due date");
        rebuild_projection(&pool).await.expect("rebuild projection");

        let projection: ObligationProjection = sqlx::query_as(
            "SELECT obligation_id, status, hard_due_at, soft_due_at, source_fragment_id FROM obligation_projection WHERE obligation_id = $1",
        )
        .bind(obligation_id)
        .fetch_one(&pool)
        .await
        .expect("projection row");
        assert_eq!(projection.status, "at_risk");
        assert!(
            projection.hard_due_at.is_some(),
            "a status_changed event that doesn't name hard_due_at must preserve the previously-recorded value"
        );
    }

    /// ADR-0023: mirrors the due-date carry-forward guarantee, applied to
    /// source_fragment_id.
    #[tokio::test]
    async fn rebuild_preserves_a_source_fragment_id_across_an_event_that_does_not_name_it() {
        let pool = test_pool().await;
        let obligation_id = Uuid::new_v4();
        let fragment_id = Uuid::new_v4();

        append_event(
            &pool,
            obligation_id,
            ObligationEventType::Created,
            json!({"status": "open", "source_fragment_id": fragment_id.to_string()}),
        )
        .await
        .expect("append created event with evidence");
        append_event(&pool, obligation_id, ObligationEventType::StatusChanged, json!({"status": "at_risk"}))
            .await
            .expect("append status_changed event naming no evidence");
        rebuild_projection(&pool).await.expect("rebuild projection");

        let projection: ObligationProjection = sqlx::query_as(
            "SELECT obligation_id, status, hard_due_at, soft_due_at, source_fragment_id FROM obligation_projection WHERE obligation_id = $1",
        )
        .bind(obligation_id)
        .fetch_one(&pool)
        .await
        .expect("projection row");
        assert_eq!(
            projection.source_fragment_id,
            Some(fragment_id),
            "a status_changed event that doesn't name source_fragment_id must preserve the previously-recorded value"
        );
    }
}
