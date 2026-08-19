use ringmaster_backend::obligation::{self, ObligationStatus};
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
        Some("update-obligation") => run_update_obligation(args[1..].to_vec()).await,
        _ => run_ingest(args).await,
    };

    if let Err(message) = result {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

async fn connect_pool() -> Result<sqlx::PgPool, String> {
    let database_url =
        std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set".to_string())?;
    PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .map_err(|error| error.to_string())
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
    println!(
        "{}",
        serde_json::json!({ "unembedded_before": unembedded_before, "embedded": embedded })
    );
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
            "--source-type" => {
                source_type = Some(iter.next().ok_or("--source-type requires a value")?)
            }
            "--title" => title = Some(iter.next().ok_or("--title requires a value")?),
            "--occurred-at" => {
                occurred_at_raw = Some(iter.next().ok_or("--occurred-at requires a value")?)
            }
            "--participants" => {
                participants.push(iter.next().ok_or("--participants requires a value")?)
            }
            "--text-file" => text_file = Some(iter.next().ok_or("--text-file requires a value")?),
            other => return Err(format!("unknown flag: {other}")),
        }
    }

    let source_type = source_type.ok_or("missing --source-type")?;
    let title = title.ok_or("missing --title")?;
    let occurred_at_raw = occurred_at_raw.ok_or("missing --occurred-at")?;
    let occurred_at = chrono::DateTime::parse_from_rfc3339(&occurred_at_raw)
        .map(|value| value.with_timezone(&chrono::Utc))
        .map_err(|_| {
            format!("--occurred-at must be a valid RFC3339 datetime, got: {occurred_at_raw}")
        })?;

    let text = match text_file {
        Some(path) => std::fs::read_to_string(&path)
            .map_err(|error| format!("could not read --text-file {path}: {error}"))?,
        None => {
            let mut buffer = String::new();
            std::io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|error| error.to_string())?;
            buffer
        }
    };
    if text.trim().is_empty() {
        return Err(
            "text must not be blank (pass --text-file <path> or pipe via stdin)".to_string(),
        );
    }

    let pool = connect_pool().await?;
    let metadata = SourceMetadata {
        source_type,
        title,
        occurred_at,
        participants,
    };
    let ingested = ingest_source(&pool, &metadata, &text)
        .await
        .map_err(|error| error.to_string())?;

    println!(
        "{}",
        serde_json::json!({ "node_id": ingested.node_id, "fragment_ids": ingested.fragment_ids })
    );
    Ok(())
}

/// ADR-0093: the CLI edit surface, calling the exact same
/// `obligation::update_status` function the HTTP `PATCH /api/obligations/:id`
/// route and the `update_obligation` MCP tool call -- no separate logic to
/// drift. To clear a due date rather than leave it unchanged, pass an empty
/// string (e.g. `--hard-due ""`), matching the function's own contract.
async fn run_update_obligation(args: Vec<String>) -> Result<(), String> {
    let mut id_raw: Option<String> = None;
    let mut status_raw: Option<String> = None;
    let mut hard_due_at: Option<String> = None;
    let mut soft_due_at: Option<String> = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "--id" => id_raw = Some(iter.next().ok_or("--id requires a value")?),
            "--status" => status_raw = Some(iter.next().ok_or("--status requires a value")?),
            "--hard-due" => hard_due_at = Some(iter.next().ok_or("--hard-due requires a value")?),
            "--soft-due" => soft_due_at = Some(iter.next().ok_or("--soft-due requires a value")?),
            other => return Err(format!("unknown flag: {other}")),
        }
    }

    let id_raw = id_raw.ok_or("missing --id")?;
    let id = uuid::Uuid::parse_str(&id_raw).map_err(|_| format!("--id is not a valid uuid: {id_raw}"))?;
    let new_status = match status_raw {
        Some(raw) => Some(
            ObligationStatus::parse(&raw)
                .ok_or(format!("--status must be one of open/at_risk/closed, got {raw:?}"))?,
        ),
        None => None,
    };

    let pool = connect_pool().await?;
    let updated = obligation::update_status(
        &pool,
        id,
        new_status,
        hard_due_at,
        soft_due_at,
        "local-operator",
        "cli",
    )
    .await
    .map_err(|error| error.to_string())?;

    println!(
        "{}",
        serde_json::json!({
            "obligation_id": updated.obligation_id,
            "status": updated.status,
            "hard_due_at": updated.hard_due_at.map(|value| value.to_rfc3339()),
            "soft_due_at": updated.soft_due_at.map(|value| value.to_rfc3339()),
        })
    );
    Ok(())
}
