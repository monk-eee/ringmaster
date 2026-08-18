use crate::embedding_adapter::{self, EmbeddingAdapterError, EmbeddingConfig};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct SourceFragment {
    pub id: Uuid,
    pub source_id: Uuid,
    pub text: String,
    pub hash: String,
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

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct MeetingFragment {
    pub id: Uuid,
    pub text: String,
    pub speaker: Option<String>,
    pub sequence: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Reads one meeting's source fragments in transcript turn order
/// (ADR-0036). Orders by `sequence` when present -- every fragment
/// ingested via `ingest_transcript` since this ADR has one -- falling back
/// to `created_at`/`id` for any fragment created before this column
/// existed. `created_at` alone is unreliable within one ingestion
/// transaction because Postgres's `now()` is the transaction start time,
/// not a per-statement time, so fragments from the same ingestion call can
/// share an identical timestamp.
pub async fn list_source_fragments_by_meeting(
    pool: &PgPool,
    meeting_id: Uuid,
) -> Result<Vec<MeetingFragment>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, text, speaker, sequence, created_at FROM source_fragments \
         WHERE source_id = $1 ORDER BY sequence ASC NULLS LAST, created_at ASC, id ASC",
    )
    .bind(meeting_id)
    .fetch_all(pool)
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
    let fragment = get_source_fragment(pool, source_fragment_id)
        .await
        .map_err(EmbeddingError::Database)?;
    let vector = embedding_adapter::embed(config, &fragment.text)
        .await
        .map_err(EmbeddingError::Adapter)?;
    let literal = format!(
        "[{}]",
        vector
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );

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
    let vector = embedding_adapter::embed(config, query)
        .await
        .map_err(EmbeddingError::Adapter)?;
    let literal = format!(
        "[{}]",
        vector
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );

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
    async fn source_fragments_round_trip() {
        let pool = test_pool().await;
        let meeting_id = Uuid::new_v4();
        let id = create_source_fragment(
            &pool,
            meeting_id,
            "We have a two-week transition.",
            "hash123",
        )
        .await
        .expect("create source fragment");
        let fragment = get_source_fragment(&pool, id)
            .await
            .expect("read source fragment back");
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
            eprintln!(
                "skipped: RINGMASTER_EMBEDDING_URL is not set, no live embedding model configured"
            );
            return;
        };
        let pool = test_pool().await;
        let meeting_id = Uuid::new_v4();
        let fragment_id = create_source_fragment(
            &pool,
            meeting_id,
            "We have a two-week transition.",
            "embed-test-hash",
        )
        .await
        .expect("create source fragment");

        let result = embed_source_fragment(&pool, &config, fragment_id).await;
        assert!(
            result.is_ok(),
            "live embedding call failed: {:?}",
            result.err()
        );

        let (dimension,): (i32,) =
            sqlx::query_as("SELECT vector_dims(embedding) FROM embeddings WHERE entity_id = $1")
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
            eprintln!(
                "skipped: RINGMASTER_EMBEDDING_URL is not set, no live embedding model configured"
            );
            return;
        };
        let pool = test_pool().await;
        let meeting_id = Uuid::new_v4();
        let fragment_id = create_source_fragment(
            &pool,
            meeting_id,
            "We have a two-week transition plan.",
            "search-test-hash",
        )
        .await
        .expect("create source fragment");
        embed_source_fragment(&pool, &config, fragment_id)
            .await
            .expect("embed source fragment");

        let results = search_source_fragments(&pool, &config, "transition plan", 5)
            .await
            .expect("live search call failed");

        assert!(
            !results.is_empty(),
            "search must return at least one ranked result"
        );
        assert!(
            results
                .iter()
                .any(|result| result.source_fragment_id == fragment_id),
            "the just-embedded fragment must appear among the ranked results"
        );
    }
}
