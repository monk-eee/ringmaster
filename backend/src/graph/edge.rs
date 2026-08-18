use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct Edge {
    pub id: Uuid,
    pub from_id: Uuid,
    pub to_id: Uuid,
    pub edge_type: String,
    pub confidence: Option<f32>,
    pub valid_from: Option<chrono::DateTime<chrono::Utc>>,
    pub valid_to: Option<chrono::DateTime<chrono::Utc>>,
}

/// Creates one edge between any two entity ids (ADR-0009). `from_id`/`to_id`
/// may each be a `nodes.id` or an Obligation's `obligation_id`; nothing
/// enforces that at the database level. `valid_from`/`valid_to` are always
/// NULL; use `create_edge_with_options` for temporal validity (ADR-0032).
/// Generic over the SQL executor (ADR-0038's pattern) so a caller can
/// create an edge in the same transaction as another write -- `&PgPool`
/// still works unchanged.
pub async fn create_edge<'e, E>(
    executor: E,
    from_id: Uuid,
    to_id: Uuid,
    edge_type: &str,
    confidence: Option<f32>,
) -> Result<Uuid, sqlx::Error>
where
    E: sqlx::PgExecutor<'e>,
{
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO edges (from_id, to_id, edge_type, confidence) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(from_id)
    .bind(to_id)
    .bind(edge_type)
    .bind(confidence)
    .fetch_one(executor)
    .await?;
    Ok(id)
}

/// Creates one edge, optionally superseding a prior current edge of the same
/// `(from_id, edge_type)` (ADR-0032). When `supersede` is false, behavior is
/// byte-for-byte identical to `create_edge`: `valid_from`/`valid_to` stay
/// NULL. When true, in one transaction: every existing edge sharing this
/// `from_id`/`edge_type` with a NULL `valid_to` (still current) has its
/// `valid_to` set to this new edge's `valid_from` (defaulting to `now()`),
/// then the new edge is inserted current (NULL `valid_to`). Matching is
/// deliberately on `(from_id, edge_type)` only, not `to_id` -- "this node
/// has one current fact of this type," not "this exact link."
pub async fn create_edge_with_options(
    pool: &PgPool,
    from_id: Uuid,
    to_id: Uuid,
    edge_type: &str,
    confidence: Option<f32>,
    valid_from: Option<chrono::DateTime<chrono::Utc>>,
    supersede: bool,
) -> Result<Uuid, sqlx::Error> {
    if !supersede {
        return create_edge(pool, from_id, to_id, edge_type, confidence).await;
    }
    let valid_from = valid_from.unwrap_or_else(chrono::Utc::now);
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE edges SET valid_to = $1 WHERE from_id = $2 AND edge_type = $3 AND valid_to IS NULL",
    )
    .bind(valid_from)
    .bind(from_id)
    .bind(edge_type)
    .execute(&mut *tx)
    .await?;
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO edges (from_id, to_id, edge_type, confidence, valid_from) VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(from_id)
    .bind(to_id)
    .bind(edge_type)
    .bind(confidence)
    .bind(valid_from)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(id)
}

pub async fn get_edge(pool: &PgPool, id: Uuid) -> Result<Edge, sqlx::Error> {
    sqlx::query_as("SELECT id, from_id, to_id, edge_type, confidence, valid_from, valid_to FROM edges WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
}

/// Lists every relationship touching one graph entity (ADR-0066), optionally
/// narrowed to one edge type or to relationships whose `valid_to` is NULL.
pub async fn list_edges_for_node(
    pool: &PgPool,
    id: Uuid,
    edge_type: Option<&str>,
    current_only: bool,
) -> Result<Vec<Edge>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, from_id, to_id, edge_type, confidence, valid_from, valid_to \
                 FROM edges \
                 WHERE (from_id = $1 OR to_id = $1) \
                     AND ($2::text IS NULL OR edge_type = $2) \
                     AND ($3::boolean IS NOT TRUE OR valid_to IS NULL) \
                 ORDER BY created_at DESC",
    )
    .bind(id)
    .bind(edge_type)
    .bind(current_only)
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::create_node;
    use serde_json::json;
    use sqlx::postgres::PgPoolOptions;

    async fn test_pool() -> PgPool {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to run graph tests");
        crate::guard_test_database(&database_url);
        PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await
            .expect("connect to test database")
    }

    #[tokio::test]
    async fn edges_round_trip_and_can_reference_a_node_or_an_obligation_id() {
        let pool = test_pool().await;
        let person_id = create_node(&pool, "person", "John", json!({}))
            .await
            .expect("create person node");
        let obligation_id = Uuid::new_v4(); // stands in for a real Obligation id; no FK enforces this (ADR-0009).

        let edge_id = create_edge(&pool, person_id, obligation_id, "made", Some(0.9))
            .await
            .expect("create edge from a node to an obligation id");
        let edge = get_edge(&pool, edge_id).await.expect("read edge back");
        assert_eq!(edge.from_id, person_id);
        assert_eq!(edge.to_id, obligation_id);
        assert_eq!(edge.edge_type, "made");
    }
}
