use ringmaster_backend::embedding_adapter::EmbeddingConfig;
use ringmaster_backend::graph;
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

/// ADR-0042: the same optional filters `GET /api/nodes` accepts, so this
/// tool and that route share one underlying `list_nodes` call.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RecallSourcesParams {
    /// Only return nodes of this free-text type, e.g. "meeting", "email", "note".
    #[serde(default)]
    pub node_type: Option<String>,
    /// Only return nodes that occurred at or after this RFC3339 datetime.
    #[serde(default)]
    pub occurred_from: Option<String>,
    /// Only return nodes that occurred at or before this RFC3339 datetime.
    #[serde(default)]
    pub occurred_to: Option<String>,
}

/// ADR-0064: a natural-language query over embedded source fragments, reusing
/// the same `graph::search_source_fragments` the HTTP `GET /api/search` route
/// calls (ADR-0019).
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchParams {
    /// Natural-language query to rank source fragments by semantic similarity.
    pub query: String,
    /// Maximum number of hits to return (default 5).
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Clone)]
struct RingmasterIngestServer {
    pool: PgPool,
}

/// Parses an optional RFC3339 argument for an MCP tool: absent is `Ok(None)`,
/// present-but-unparseable is `Err` with a human-readable message (ADR-0042).
fn parse_optional_rfc3339(label: &str, raw: Option<&str>) -> Result<Option<chrono::DateTime<chrono::Utc>>, String> {
    match raw.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(None),
        Some(value) => chrono::DateTime::parse_from_rfc3339(value)
            .map(|parsed| Some(parsed.with_timezone(&chrono::Utc)))
            .map_err(|_| format!("{label} must be a valid RFC3339 datetime, got: {value}")),
    }
}

#[tool_router(server_handler)]
impl RingmasterIngestServer {
    /// Ingestion (ADR-0040) and recall (ADR-0042): no resources, prompts,
    /// or sampling -- getting dated sources in and back out, matching what
    /// was actually asked for.
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

    #[tool(
        description = "Recall previously-ingested dated sources (nodes), optionally filtered by node_type and/or an occurred_at range. Requires no embedding model -- a plain date-range/type read, not similarity search."
    )]
    async fn recall_sources(&self, Parameters(params): Parameters<RecallSourcesParams>) -> Result<CallToolResult, rmcp::ErrorData> {
        let occurred_from = match parse_optional_rfc3339("occurred_from", params.occurred_from.as_deref()) {
            Ok(value) => value,
            Err(message) => return Ok(CallToolResult::error(vec![ContentBlock::text(message)])),
        };
        let occurred_to = match parse_optional_rfc3339("occurred_to", params.occurred_to.as_deref()) {
            Ok(value) => value,
            Err(message) => return Ok(CallToolResult::error(vec![ContentBlock::text(message)])),
        };

        match graph::list_nodes(&self.pool, params.node_type.as_deref(), occurred_from, occurred_to, false, None, None).await {
            Ok(nodes) => Ok(CallToolResult::success(vec![ContentBlock::text(serde_json::json!(nodes).to_string())])),
            Err(error) => Ok(CallToolResult::error(vec![ContentBlock::text(error.to_string())])),
        }
    }

    #[tool(
        description = "Search previously-ingested evidence by meaning: ranks source fragments by semantic similarity to a natural-language query. Requires an embedding model to be configured; returns a clear error if one is not."
    )]
    async fn search(&self, Parameters(params): Parameters<SearchParams>) -> Result<CallToolResult, rmcp::ErrorData> {
        let Some(config) = EmbeddingConfig::from_env() else {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "RINGMASTER_EMBEDDING_URL is not set; semantic search is disabled".to_string(),
            )]));
        };
        let limit = params.limit.filter(|value| *value > 0).unwrap_or(5);
        match graph::search_source_fragments(&self.pool, &config, &params.query, limit).await {
            Ok(results) => Ok(CallToolResult::success(vec![ContentBlock::text(serde_json::json!(results).to_string())])),
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
