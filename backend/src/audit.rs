use serde_json::Value as Json;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

const DEFAULT_RECENT_LIMIT: i64 = 50;
const MAX_RECENT_LIMIT: i64 = 200;

#[derive(Debug, Clone, FromRow)]
pub struct AuditEvent {
    pub id: Uuid,
    pub actor: String,
    pub action: String,
    pub previous_state: Option<Json>,
    pub new_state: Option<Json>,
    pub source: String,
    pub policy_outcome: String,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
}

/// Appends one immutable audit row (ADR-0008). The database rejects any
/// later mutation or deletion of the returned row. Generic over the SQL
/// executor (ADR-0038) so a caller can record an audit row in the same
/// transaction as the state change it documents -- `&PgPool` still works
/// unchanged for any caller outside a transaction.
pub async fn record<'e, E>(
    executor: E,
    actor: &str,
    action: &str,
    previous_state: Option<Json>,
    new_state: Option<Json>,
    source: &str,
    policy_outcome: &str,
) -> Result<Uuid, sqlx::Error>
where
    E: sqlx::PgExecutor<'e>,
{
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO audit_events (actor, action, previous_state, new_state, source, policy_outcome) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(actor)
    .bind(action)
    .bind(&previous_state)
    .bind(&new_state)
    .bind(source)
    .bind(policy_outcome)
    .fetch_one(executor)
    .await?;
    Ok(id)
}

/// Reads the most recent audit rows, newest first (ADR-0049). `limit` is
/// clamped to `[1, 200]` (default 50 for `None`) rather than rejected --
/// this is a read-only diagnostic feed, not a validated write.
pub async fn recent(pool: &PgPool, limit: Option<i64>) -> Result<Vec<AuditEvent>, sqlx::Error> {
    let limit = limit.unwrap_or(DEFAULT_RECENT_LIMIT).clamp(1, MAX_RECENT_LIMIT);
    sqlx::query_as(
        "SELECT id, actor, action, previous_state, new_state, source, policy_outcome, recorded_at \
         FROM audit_events ORDER BY recorded_at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sqlx::postgres::PgPoolOptions;

    async fn test_pool() -> PgPool {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to run audit tests");
        crate::guard_test_database(&database_url);
        PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await
            .expect("connect to test database")
    }

    #[tokio::test]
    async fn audit_rows_cannot_be_mutated_or_deleted() {
        let pool = test_pool().await;
        let event_id = record(
            &pool,
            "test-actor",
            "test-action",
            None,
            Some(json!({"status": "recorded"})),
            "test",
            "advise",
        )
        .await
        .expect("append audit row");

        let update_result = sqlx::query("UPDATE audit_events SET action = 'tampered' WHERE id = $1")
            .bind(event_id)
            .execute(&pool)
            .await;
        assert!(update_result.is_err(), "UPDATE must be rejected by the append-only trigger");

        let delete_result = sqlx::query("DELETE FROM audit_events WHERE id = $1")
            .bind(event_id)
            .execute(&pool)
            .await;
        assert!(delete_result.is_err(), "DELETE must be rejected by the append-only trigger");
    }

    #[tokio::test]
    async fn recent_orders_newest_first_and_respects_limit() {
        let pool = test_pool().await;
        let rows = recent(&pool, Some(5)).await.expect("read recent");
        assert!(rows.len() <= 5, "must never return more than the requested limit");
        for pair in rows.windows(2) {
            assert!(pair[0].recorded_at >= pair[1].recorded_at, "rows must be ordered newest first");
        }
    }

    #[tokio::test]
    async fn recent_clamps_a_limit_above_the_maximum() {
        let pool = test_pool().await;
        let rows = recent(&pool, Some(10_000)).await.expect("read recent");
        assert!(rows.len() <= 200, "limit must be clamped to the maximum of 200");
    }

    #[tokio::test]
    async fn recent_clamps_a_limit_below_the_minimum() {
        let pool = test_pool().await;
        let rows = recent(&pool, Some(0)).await.expect("read recent");
        assert!(rows.len() <= 1, "a limit of 0 must be clamped up to 1");
    }

    #[tokio::test]
    async fn recent_defaults_to_fifty_when_no_limit_given() {
        let pool = test_pool().await;
        let rows = recent(&pool, None).await.expect("read recent");
        assert!(rows.len() <= 50, "omitting limit must default to at most 50");
    }

    #[tokio::test]
    async fn a_newly_recorded_row_is_findable_via_recent() {
        let pool = test_pool().await;
        let marker = format!("recent-marker-{}", Uuid::new_v4());
        record(&pool, "test-actor", "test-action", None, Some(json!({"marker": marker})), "test", "advise")
            .await
            .expect("append row");

        let rows = recent(&pool, Some(200)).await.expect("read recent");
        assert!(
            rows.iter()
                .any(|row| row.new_state.as_ref().and_then(|value| value.get("marker")).and_then(|value| value.as_str()) == Some(marker.as_str())),
            "a just-recorded row must appear in the 200-row window (it is the newest possible row)"
        );
    }
}
