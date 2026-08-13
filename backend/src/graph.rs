use serde_json::Value as Json;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct Node {
    pub id: Uuid,
    pub node_type: String,
    pub canonical_text: String,
    pub attributes: Json,
    pub lifecycle_state: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct Edge {
    pub id: Uuid,
    pub from_id: Uuid,
    pub to_id: Uuid,
    pub edge_type: String,
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, FromRow)]
pub struct SourceFragment {
    pub id: Uuid,
    pub source_id: Uuid,
    pub text: String,
    pub hash: String,
}

/// Creates one node (ADR-0009). Deduplication/entity resolution against
/// existing nodes is future, extraction-pipeline work (docs/PRODUCT-SPEC.md
/// SS6.2) and is not implemented here.
pub async fn create_node(
    pool: &PgPool,
    node_type: &str,
    canonical_text: &str,
    attributes: Json,
) -> Result<Uuid, sqlx::Error> {
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO nodes (node_type, canonical_text, attributes) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(node_type)
    .bind(canonical_text)
    .bind(&attributes)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn get_node(pool: &PgPool, id: Uuid) -> Result<Node, sqlx::Error> {
    sqlx::query_as("SELECT id, node_type, canonical_text, attributes, lifecycle_state FROM nodes WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
}

/// Creates one edge between any two entity ids (ADR-0009). `from_id`/`to_id`
/// may each be a `nodes.id` or an Obligation's `obligation_id`; nothing
/// enforces that at the database level.
pub async fn create_edge(
    pool: &PgPool,
    from_id: Uuid,
    to_id: Uuid,
    edge_type: &str,
    confidence: Option<f32>,
) -> Result<Uuid, sqlx::Error> {
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO edges (from_id, to_id, edge_type, confidence) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(from_id)
    .bind(to_id)
    .bind(edge_type)
    .bind(confidence)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn get_edge(pool: &PgPool, id: Uuid) -> Result<Edge, sqlx::Error> {
    sqlx::query_as("SELECT id, from_id, to_id, edge_type, confidence FROM edges WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn create_source_fragment(
    pool: &PgPool,
    source_id: Uuid,
    text: &str,
    hash: &str,
) -> Result<Uuid, sqlx::Error> {
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO source_fragments (source_id, text, hash) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(source_id)
    .bind(text)
    .bind(hash)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn get_source_fragment(pool: &PgPool, id: Uuid) -> Result<SourceFragment, sqlx::Error> {
    sqlx::query_as("SELECT id, source_id, text, hash FROM source_fragments WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sqlx::postgres::PgPoolOptions;

    async fn test_pool() -> PgPool {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to run graph tests");
        PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await
            .expect("connect to test database")
    }

    #[tokio::test]
    async fn nodes_round_trip() {
        let pool = test_pool().await;
        let id = create_node(&pool, "person", "Roopa", json!({"role": "manager"}))
            .await
            .expect("create node");
        let node = get_node(&pool, id).await.expect("read node back");
        assert_eq!(node.node_type, "person");
        assert_eq!(node.canonical_text, "Roopa");
        assert_eq!(node.lifecycle_state, "active");
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

    #[tokio::test]
    async fn source_fragments_round_trip() {
        let pool = test_pool().await;
        let meeting_id = Uuid::new_v4();
        let id = create_source_fragment(&pool, meeting_id, "We have a two-week transition.", "hash123")
            .await
            .expect("create source fragment");
        let fragment = get_source_fragment(&pool, id).await.expect("read source fragment back");
        assert_eq!(fragment.source_id, meeting_id);
        assert_eq!(fragment.text, "We have a two-week transition.");
        assert_eq!(fragment.hash, "hash123");
    }
}
