use crate::embedding_adapter::{self, EmbeddingAdapterError, EmbeddingConfig};
use serde_json::Value as Json;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct Node {
    pub id: Uuid,
    pub node_type: String,
    pub canonical_text: String,
    pub attributes: Json,
    pub lifecycle_state: String,
}

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

/// Lists nodes, optionally filtered by `node_type` (ADR-0025). Read-only.
pub async fn list_nodes(pool: &PgPool, node_type: Option<&str>) -> Result<Vec<Node>, sqlx::Error> {
    match node_type {
        Some(node_type) => {
            sqlx::query_as(
                "SELECT id, node_type, canonical_text, attributes, lifecycle_state FROM nodes \
                 WHERE node_type = $1 ORDER BY updated_at DESC",
            )
            .bind(node_type)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as("SELECT id, node_type, canonical_text, attributes, lifecycle_state FROM nodes ORDER BY updated_at DESC")
                .fetch_all(pool)
                .await
        }
    }
}

/// Enriches an existing node (ADR-0025). Any of `canonical_text`,
/// `lifecycle_state`, `attributes` may be omitted (`None`) to leave that
/// field untouched. `attributes`, when given, is shallow-merged into the
/// existing JSONB object via Postgres `||` -- enriching one attribute never
/// clobbers others already recorded.
pub async fn update_node(
    pool: &PgPool,
    id: Uuid,
    canonical_text: Option<&str>,
    lifecycle_state: Option<&str>,
    attributes: Option<&Json>,
) -> Result<Node, sqlx::Error> {
    sqlx::query_as(
        "UPDATE nodes SET \
            canonical_text = COALESCE($2, canonical_text), \
            lifecycle_state = COALESCE($3, lifecycle_state), \
            attributes = attributes || COALESCE($4, '{}'::jsonb), \
            updated_at = now() \
         WHERE id = $1 \
         RETURNING id, node_type, canonical_text, attributes, lifecycle_state",
    )
    .bind(id)
    .bind(canonical_text)
    .bind(lifecycle_state)
    .bind(attributes)
    .fetch_one(pool)
    .await
}

/// Creates one edge between any two entity ids (ADR-0009). `from_id`/`to_id`
/// may each be a `nodes.id` or an Obligation's `obligation_id`; nothing
/// enforces that at the database level. `valid_from`/`valid_to` are always
/// NULL; use `create_edge_with_options` for temporal validity (ADR-0032).
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
    sqlx::query("UPDATE edges SET valid_to = $1 WHERE from_id = $2 AND edge_type = $3 AND valid_to IS NULL")
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

/// Embeds and stores one named source fragment (ADR-0018). Reads the
/// fragment's own immutable text, calls the configured embedding adapter,
/// and inserts one `embeddings` row. Never automatic on ingestion --
/// called explicitly, the same non-blocking posture ADR-0013 chose for
/// extraction.
#[derive(Debug)]
pub enum EmbeddingError {
    Adapter(EmbeddingAdapterError),
    Database(sqlx::Error),
}

impl std::fmt::Display for EmbeddingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Adapter(error) => write!(f, "{error}"),
            Self::Database(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for EmbeddingError {}

pub async fn embed_source_fragment(
    pool: &PgPool,
    config: &EmbeddingConfig,
    source_fragment_id: Uuid,
) -> Result<Uuid, EmbeddingError> {
    let fragment = get_source_fragment(pool, source_fragment_id).await.map_err(EmbeddingError::Database)?;
    let vector = embedding_adapter::embed(config, &fragment.text).await.map_err(EmbeddingError::Adapter)?;
    let literal = format!("[{}]", vector.iter().map(|value| value.to_string()).collect::<Vec<_>>().join(","));

    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO embeddings (entity_id, entity_type, model_id, embedding, source_hash) \
         VALUES ($1, 'source_fragment', $2, $3::vector, $4) RETURNING id",
    )
    .bind(source_fragment_id)
    .bind(&config.model)
    .bind(&literal)
    .bind(&fragment.hash)
    .fetch_one(pool)
    .await
    .map_err(EmbeddingError::Database)?;
    Ok(id)
}

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct SearchResult {
    pub source_fragment_id: Uuid,
    pub text: String,
    pub speaker: Option<String>,
    pub similarity: f64,
}

/// Semantic search over embedded source fragments (ADR-0019): embeds the
/// query with the same adapter ADR-0018 uses to embed fragments, then ranks
/// stored `entity_type = 'source_fragment'` embeddings by pgvector cosine
/// distance. Read-only. No keyword fusion, metadata filters, or graph
/// expansion -- each deferred, per this ADR's scope.
pub async fn search_source_fragments(
    pool: &PgPool,
    config: &EmbeddingConfig,
    query: &str,
    limit: i64,
) -> Result<Vec<SearchResult>, EmbeddingError> {
    let vector = embedding_adapter::embed(config, query).await.map_err(EmbeddingError::Adapter)?;
    let literal = format!("[{}]", vector.iter().map(|value| value.to_string()).collect::<Vec<_>>().join(","));

    sqlx::query_as(
        "SELECT e.entity_id AS source_fragment_id, sf.text, sf.speaker, \
                1 - (e.embedding <=> $1::vector) AS similarity \
         FROM embeddings e \
         JOIN source_fragments sf ON sf.id = e.entity_id \
         WHERE e.entity_type = 'source_fragment' \
         ORDER BY e.embedding <=> $1::vector \
         LIMIT $2",
    )
    .bind(&literal)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(EmbeddingError::Database)
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

    /// ADR-0025: enriching one attribute must not erase another already
    /// recorded, and omitted fields must stay untouched.
    #[tokio::test]
    async fn update_node_merges_attributes_without_clobbering_existing_ones() {
        let pool = test_pool().await;
        let id = create_node(&pool, "person", "Roopa", json!({"role": "manager"}))
            .await
            .expect("create node");

        let node = update_node(&pool, id, None, None, Some(&json!({"team": "platform"})))
            .await
            .expect("enrich node with a new attribute");

        assert_eq!(node.canonical_text, "Roopa", "unspecified canonical_text must be unchanged");
        assert_eq!(node.attributes["role"], "manager", "a previously-recorded attribute must survive enrichment");
        assert_eq!(node.attributes["team"], "platform", "the newly-enriched attribute must be present");
    }

    #[tokio::test]
    async fn list_nodes_filters_by_node_type() {
        let pool = test_pool().await;
        let person_id = create_node(&pool, "person", "Filter Test Person", json!({})).await.expect("create person");
        create_node(&pool, "risk", "Filter Test Risk", json!({})).await.expect("create risk");

        let people = list_nodes(&pool, Some("person")).await.expect("list people");
        assert!(people.iter().any(|node| node.id == person_id));
        assert!(people.iter().all(|node| node.node_type == "person"), "filter must exclude other node types");
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

    /// Exercises a real, live round-trip when RINGMASTER_EMBEDDING_URL is
    /// configured (ADR-0018); otherwise reports and passes trivially,
    /// same posture as extraction::tests's own live-model test.
    #[tokio::test]
    async fn embed_source_fragment_round_trips_against_a_live_endpoint_when_configured() {
        let Some(config) = EmbeddingConfig::from_env() else {
            eprintln!("skipped: RINGMASTER_EMBEDDING_URL is not set, no live embedding model configured");
            return;
        };
        let pool = test_pool().await;
        let meeting_id = Uuid::new_v4();
        let fragment_id = create_source_fragment(&pool, meeting_id, "We have a two-week transition.", "embed-test-hash")
            .await
            .expect("create source fragment");

        let result = embed_source_fragment(&pool, &config, fragment_id).await;
        assert!(result.is_ok(), "live embedding call failed: {:?}", result.err());

        let (dimension,): (i32,) = sqlx::query_as("SELECT vector_dims(embedding) FROM embeddings WHERE entity_id = $1")
            .bind(fragment_id)
            .fetch_one(&pool)
            .await
            .expect("read stored embedding back");
        assert_eq!(dimension, 768);
    }

    /// Exercises a real, live round-trip when RINGMASTER_EMBEDDING_URL is
    /// configured (ADR-0019); otherwise reports and passes trivially, same
    /// posture as this module's own embed-source-fragment live test.
    #[tokio::test]
    async fn search_source_fragments_round_trips_against_a_live_endpoint_when_configured() {
        let Some(config) = EmbeddingConfig::from_env() else {
            eprintln!("skipped: RINGMASTER_EMBEDDING_URL is not set, no live embedding model configured");
            return;
        };
        let pool = test_pool().await;
        let meeting_id = Uuid::new_v4();
        let fragment_id = create_source_fragment(&pool, meeting_id, "We have a two-week transition plan.", "search-test-hash")
            .await
            .expect("create source fragment");
        embed_source_fragment(&pool, &config, fragment_id).await.expect("embed source fragment");

        let results = search_source_fragments(&pool, &config, "transition plan", 5)
            .await
            .expect("live search call failed");

        assert!(!results.is_empty(), "search must return at least one ranked result");
        assert!(
            results.iter().any(|result| result.source_fragment_id == fragment_id),
            "the just-embedded fragment must appear among the ranked results"
        );
    }
}
