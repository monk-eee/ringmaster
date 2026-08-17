use ringmaster_backend::transcript::{ingest_source, SourceMetadata};
use sqlx::postgres::PgPoolOptions;
use std::io::Read;

mod mcp;

/// ADR-0040: a CLI binary for scripting a corpus through Ringmaster one
/// source at a time, and (via `mcp-serve`) an MCP tool for the same, both
/// calling the identical `ingest_source` function the HTTP API uses.
/// Connects directly to DATABASE_URL -- no running HTTP server required.
#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let result = match args.first().map(String::as_str) {
        Some("mcp-serve") => run_mcp_serve().await,
        Some("reindex-embeddings") => run_reindex_embeddings().await,
        _ => run_ingest(args).await,
    };

    if let Err(message) = result {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

async fn connect_pool() -> Result<sqlx::PgPool, String> {
    let database_url = std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set".to_string())?;
    PgPoolOptions::new().max_connections(2).connect(&database_url).await.map_err(|error| error.to_string())
}

async fn run_mcp_serve() -> Result<(), String> {
    let pool = connect_pool().await?;
    mcp::run(pool).await
}

/// ADR-0063: backfills embeddings for every source fragment that has none yet,
/// so the existing corpus becomes searchable without re-ingesting it.
async fn run_reindex_embeddings() -> Result<(), String> {
    let pool = connect_pool().await?;
    let Some(config) = ringmaster_backend::embedding_adapter::EmbeddingConfig::from_env() else {
        return Err("RINGMASTER_EMBEDDING_URL must be set to reindex embeddings".to_string());
    };
    let (unembedded_before, embedded) =
        ringmaster_backend::transcript::reindex_unembedded_fragments(&pool, &config)
            .await
            .map_err(|error| error.to_string())?;
    println!("{}", serde_json::json!({ "unembedded_before": unembedded_before, "embedded": embedded }));
    Ok(())
}

async fn run_ingest(args: Vec<String>) -> Result<(), String> {
    let mut source_type: Option<String> = None;
    let mut title: Option<String> = None;
    let mut occurred_at_raw: Option<String> = None;
    let mut participants: Vec<String> = Vec::new();
    let mut text_file: Option<String> = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "--source-type" => source_type = Some(iter.next().ok_or("--source-type requires a value")?),
            "--title" => title = Some(iter.next().ok_or("--title requires a value")?),
            "--occurred-at" => occurred_at_raw = Some(iter.next().ok_or("--occurred-at requires a value")?),
            "--participants" => participants.push(iter.next().ok_or("--participants requires a value")?),
            "--text-file" => text_file = Some(iter.next().ok_or("--text-file requires a value")?),
            other => return Err(format!("unknown flag: {other}")),
        }
    }

    let source_type = source_type.ok_or("missing --source-type")?;
    let title = title.ok_or("missing --title")?;
    let occurred_at_raw = occurred_at_raw.ok_or("missing --occurred-at")?;
    let occurred_at = chrono::DateTime::parse_from_rfc3339(&occurred_at_raw)
        .map(|value| value.with_timezone(&chrono::Utc))
        .map_err(|_| format!("--occurred-at must be a valid RFC3339 datetime, got: {occurred_at_raw}"))?;

    let text = match text_file {
        Some(path) => std::fs::read_to_string(&path).map_err(|error| format!("could not read --text-file {path}: {error}"))?,
        None => {
            let mut buffer = String::new();
            std::io::stdin().read_to_string(&mut buffer).map_err(|error| error.to_string())?;
            buffer
        }
    };
    if text.trim().is_empty() {
        return Err("text must not be blank (pass --text-file <path> or pipe via stdin)".to_string());
    }

    let pool = connect_pool().await?;
    let metadata = SourceMetadata { source_type, title, occurred_at, participants };
    let ingested = ingest_source(&pool, &metadata, &text).await.map_err(|error| error.to_string())?;

    println!("{}", serde_json::json!({ "node_id": ingested.node_id, "fragment_ids": ingested.fragment_ids }));
    Ok(())
}
