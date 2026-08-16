use ringmaster_backend::transcript::{ingest_source, SourceMetadata};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{schemars, tool, tool_router, transport::stdio, ServiceExt};
use sqlx::PgPool;

/// ADR-0040: the same parameters as the CLI/API surfaces, so all three call
/// the identical shared `ingest_source` function.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct IngestSourceParams {
    /// Free-text source kind, e.g. "meeting", "email", "teams_message", "note".
    pub source_type: String,
    /// Short human-readable title for this source.
    pub title: String,
    /// The real-world time this source occurred, as an RFC3339 datetime (e.g. "2026-08-14T09:00:00Z").
    pub occurred_at: String,
    /// Names of people involved, if known.
    #[serde(default)]
    pub participants: Vec<String>,
    /// The raw text to ingest.
    pub text: String,
}

#[derive(Clone)]
struct RingmasterIngestServer {
    pool: PgPool,
}

#[tool_router(server_handler)]
impl RingmasterIngestServer {
    /// The one tool this server exposes (ADR-0040): no resources, prompts,
    /// or sampling -- ingestion only, matching what was actually asked for.
    #[tool(
        description = "Ingest a dated source (meeting, email, note, Teams message, ...) into Ringmaster's graph as a node with ordered, hashed, immutable evidence fragments. Never triggers extraction or embedding."
    )]
    async fn ingest_source(&self, Parameters(params): Parameters<IngestSourceParams>) -> Result<CallToolResult, rmcp::ErrorData> {
        let occurred_at = match chrono::DateTime::parse_from_rfc3339(&params.occurred_at) {
            Ok(value) => value.with_timezone(&chrono::Utc),
            Err(_) => {
                return Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                    "occurred_at must be a valid RFC3339 datetime, got: {}",
                    params.occurred_at
                ))]))
            }
        };

        let metadata = SourceMetadata { source_type: params.source_type, title: params.title, occurred_at, participants: params.participants };

        match ingest_source(&self.pool, &metadata, &params.text).await {
            Ok(ingested) => Ok(CallToolResult::success(vec![ContentBlock::text(
                serde_json::json!({ "node_id": ingested.node_id, "fragment_ids": ingested.fragment_ids }).to_string(),
            )])),
            Err(error) => Ok(CallToolResult::error(vec![ContentBlock::text(error.to_string())])),
        }
    }
}

/// Starts the stdio MCP server (ADR-0040) and blocks until the client
/// disconnects, matching how MindLeak/Lodestar are already launched locally.
pub async fn run(pool: PgPool) -> Result<(), String> {
    let server = RingmasterIngestServer { pool };
    let service = server.serve(stdio()).await.map_err(|error| error.to_string())?;
    service.waiting().await.map_err(|error| error.to_string())?;
    Ok(())
}
