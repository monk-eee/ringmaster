use serde_json::Value as Json;
use sqlx::PgPool;
use uuid::Uuid;

/// Appends one immutable audit row (ADR-0008). The database rejects any
/// later mutation or deletion of the returned row. No application feature
/// calls this yet; wiring real call sites in is future, ADR-governed work.
pub async fn record(
    pool: &PgPool,
    actor: &str,
    action: &str,
    previous_state: Option<Json>,
    new_state: Option<Json>,
    source: &str,
    policy_outcome: &str,
) -> Result<Uuid, sqlx::Error> {
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
    .fetch_one(pool)
    .await?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sqlx::postgres::PgPoolOptions;

    async fn test_pool() -> PgPool {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to run audit tests");
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
}
