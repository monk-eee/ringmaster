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

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListEntitiesParams {
    /// Filter by exact free-text node type, e.g. "person", "team", or "project".
    #[serde(default)]
    pub node_type: Option<String>,
    /// Filter by exact, case-sensitive canonical text.
    #[serde(default)]
    pub canonical_text: Option<String>,
    /// Only return entities that occurred at or after this RFC3339 datetime.
    #[serde(default)]
    pub occurred_from: Option<String>,
    /// Only return entities that occurred at or before this RFC3339 datetime.
    #[serde(default)]
    pub occurred_to: Option<String>,
    /// Maximum entities to return, from 1 to 500. Omit for all matches.
    #[serde(default)]
    pub limit: Option<i64>,
    /// Number of matching entities to skip. Defaults to 0.
    #[serde(default)]
    pub offset: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetEntityParams {
    /// Entity UUID.
    pub id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateEntityParams {
    /// Free-text node type, e.g. "person", "team", or "project".
    pub node_type: String,
    /// Human-readable identity text for the entity.
    pub canonical_text: String,
    /// Structured entity attributes such as email, title, manager, or team.
    #[serde(default)]
    pub attributes: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UpdateEntityParams {
    /// Entity UUID.
    pub id: String,
    /// Replacement human-readable identity text.
    #[serde(default)]
    pub canonical_text: Option<String>,
    /// Replacement lifecycle state.
    #[serde(default)]
    pub lifecycle_state: Option<String>,
    /// Attributes to shallow-merge into the existing object.
    #[serde(default)]
    pub attributes: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EntityUpsertParams {
    /// Exact free-text node type used as part of the identity key.
    pub node_type: String,
    /// Exact, case-sensitive canonical text used as part of the identity key.
    pub canonical_text: String,
    /// Optional lifecycle state to set on create or update.
    #[serde(default)]
    pub lifecycle_state: Option<String>,
    /// Attributes to set on create or shallow-merge on update.
    #[serde(default)]
    pub attributes: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UpsertEntitiesParams {
    /// Between 1 and 100 entities to create or enrich atomically.
    pub entities: Vec<EntityUpsertParams>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListRelationshipsParams {
    /// UUID of the entity whose touching relationships should be returned.
    pub entity_id: String,
    /// Optional exact edge-type filter, e.g. "owns", "member_of", or "manages".
    #[serde(default)]
    pub edge_type: Option<String>,
    /// When true, omit relationships with a non-null valid_to.
    #[serde(default)]
    pub current_only: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateRelationshipParams {
    /// UUID at the relationship's source end. May identify a node or Obligation.
    pub from_id: String,
    /// UUID at the relationship's target end. May identify a node or Obligation.
    pub to_id: String,
    /// Free-text relationship type, e.g. "owns", "member_of", or "manages".
    pub edge_type: String,
    /// Optional confidence from 0.0 to 1.0.
    #[serde(default)]
    pub confidence: Option<f32>,
    /// Optional RFC3339 time from which this relationship is valid.
    #[serde(default)]
    pub valid_from: Option<String>,
    /// Close prior current relationships with the same from_id and edge_type.
    #[serde(default)]
    pub supersede: bool,
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

fn parse_uuid(label: &str, raw: &str) -> Result<uuid::Uuid, String> {
    uuid::Uuid::parse_str(raw.trim()).map_err(|_| format!("{label} must be a valid UUID, got: {raw}"))
}

fn required_text<'a>(label: &str, raw: &'a str) -> Result<&'a str, String> {
    let value = raw.trim();
    if value.is_empty() {
        Err(format!("{label} must not be blank"))
    } else {
        Ok(value)
    }
}

fn optional_text(label: &str, raw: Option<String>) -> Result<Option<String>, String> {
    raw.map(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            Err(format!("{label} must not be blank when supplied"))
        } else {
            Ok(trimmed.to_string())
        }
    })
    .transpose()
}

fn validate_attributes(attributes: Option<&serde_json::Value>) -> Result<(), String> {
    if attributes.is_some_and(|value| !value.is_object()) {
        Err("attributes must be a JSON object when supplied".to_string())
    } else {
        Ok(())
    }
}

fn tool_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message.into())])
}

fn json_success(value: impl serde::Serialize) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(serde_json::json!(value).to_string())])
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

    #[tool(
        description = "List Ringmaster graph entities with optional exact type/name and occurred-at filters. Use this to discover entity UUIDs before id-based updates or relationship writes."
    )]
    async fn list_entities(&self, Parameters(params): Parameters<ListEntitiesParams>) -> Result<CallToolResult, rmcp::ErrorData> {
        let node_type = match optional_text("node_type", params.node_type) {
            Ok(value) => value,
            Err(message) => return Ok(tool_error(message)),
        };
        let canonical_text = match optional_text("canonical_text", params.canonical_text) {
            Ok(value) => value,
            Err(message) => return Ok(tool_error(message)),
        };
        let occurred_from = match parse_optional_rfc3339("occurred_from", params.occurred_from.as_deref()) {
            Ok(value) => value,
            Err(message) => return Ok(tool_error(message)),
        };
        let occurred_to = match parse_optional_rfc3339("occurred_to", params.occurred_to.as_deref()) {
            Ok(value) => value,
            Err(message) => return Ok(tool_error(message)),
        };
        if params.limit.is_some_and(|value| !(1..=500).contains(&value)) {
            return Ok(tool_error("limit must be between 1 and 500 when supplied"));
        }
        if params.offset.is_some_and(|value| value < 0) {
            return Ok(tool_error("offset must be zero or greater when supplied"));
        }

        match graph::list_nodes_filtered(
            &self.pool,
            node_type.as_deref(),
            canonical_text.as_deref(),
            occurred_from,
            occurred_to,
            false,
            params.limit,
            params.offset,
        )
        .await
        {
            Ok(nodes) => Ok(json_success(nodes)),
            Err(error) => Ok(tool_error(error.to_string())),
        }
    }

    #[tool(description = "Get one Ringmaster graph entity by UUID together with every relationship touching it.")]
    async fn get_entity(&self, Parameters(params): Parameters<GetEntityParams>) -> Result<CallToolResult, rmcp::ErrorData> {
        let id = match parse_uuid("id", &params.id) {
            Ok(value) => value,
            Err(message) => return Ok(tool_error(message)),
        };
        let node = match graph::get_node(&self.pool, id).await {
            Ok(node) => node,
            Err(sqlx::Error::RowNotFound) => return Ok(tool_error(format!("entity not found: {id}"))),
            Err(error) => return Ok(tool_error(error.to_string())),
        };
        match graph::list_edges_for_node(&self.pool, id, None, false).await {
            Ok(relationships) => Ok(json_success(serde_json::json!({ "entity": node, "relationships": relationships }))),
            Err(error) => Ok(tool_error(error.to_string())),
        }
    }

    #[tool(description = "Create one generic Ringmaster graph entity. Attributes must be a JSON object.")]
    async fn create_entity(&self, Parameters(params): Parameters<CreateEntityParams>) -> Result<CallToolResult, rmcp::ErrorData> {
        let node_type = match required_text("node_type", &params.node_type) {
            Ok(value) => value,
            Err(message) => return Ok(tool_error(message)),
        };
        let canonical_text = match required_text("canonical_text", &params.canonical_text) {
            Ok(value) => value,
            Err(message) => return Ok(tool_error(message)),
        };
        if let Err(message) = validate_attributes(params.attributes.as_ref()) {
            return Ok(tool_error(message));
        }
        let attributes = params.attributes.unwrap_or_else(|| serde_json::json!({}));
        let id = match graph::create_node(&self.pool, node_type, canonical_text, attributes).await {
            Ok(id) => id,
            Err(error) => return Ok(tool_error(error.to_string())),
        };
        match graph::get_node(&self.pool, id).await {
            Ok(node) => Ok(json_success(node)),
            Err(error) => Ok(tool_error(error.to_string())),
        }
    }

    #[tool(
        description = "Update one Ringmaster graph entity by UUID. Supplied attributes shallow-merge and never clobber other existing keys."
    )]
    async fn update_entity(&self, Parameters(params): Parameters<UpdateEntityParams>) -> Result<CallToolResult, rmcp::ErrorData> {
        let id = match parse_uuid("id", &params.id) {
            Ok(value) => value,
            Err(message) => return Ok(tool_error(message)),
        };
        let canonical_text = match optional_text("canonical_text", params.canonical_text) {
            Ok(value) => value,
            Err(message) => return Ok(tool_error(message)),
        };
        let lifecycle_state = match optional_text("lifecycle_state", params.lifecycle_state) {
            Ok(value) => value,
            Err(message) => return Ok(tool_error(message)),
        };
        if let Err(message) = validate_attributes(params.attributes.as_ref()) {
            return Ok(tool_error(message));
        }
        if canonical_text.is_none() && lifecycle_state.is_none() && params.attributes.is_none() {
            return Ok(tool_error("supply canonical_text, lifecycle_state, attributes, or a combination to update"));
        }

        match graph::update_node(
            &self.pool,
            id,
            canonical_text.as_deref(),
            lifecycle_state.as_deref(),
            params.attributes.as_ref(),
        )
        .await
        {
            Ok(node) => Ok(json_success(node)),
            Err(sqlx::Error::RowNotFound) => Ok(tool_error(format!("entity not found: {id}"))),
            Err(error) => Ok(tool_error(error.to_string())),
        }
    }

    #[tool(
        description = "Atomically create or enrich 1-100 graph entities. Exact case-sensitive (node_type, canonical_text) identity only; ambiguous duplicates fail the whole batch. Attributes shallow-merge."
    )]
    async fn upsert_entities(&self, Parameters(params): Parameters<UpsertEntitiesParams>) -> Result<CallToolResult, rmcp::ErrorData> {
        let entities = params
            .entities
            .into_iter()
            .map(|entity| graph::EntityUpsert {
                node_type: entity.node_type,
                canonical_text: entity.canonical_text,
                lifecycle_state: entity.lifecycle_state,
                attributes: entity.attributes,
            })
            .collect();
        match graph::upsert_nodes(&self.pool, entities).await {
            Ok(results) => Ok(json_success(results)),
            Err(error) => Ok(tool_error(error.to_string())),
        }
    }

    #[tool(
        description = "List every relationship touching one Ringmaster entity, optionally filtered by edge type or to currently-valid relationships."
    )]
    async fn list_relationships(
        &self,
        Parameters(params): Parameters<ListRelationshipsParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let id = match parse_uuid("entity_id", &params.entity_id) {
            Ok(value) => value,
            Err(message) => return Ok(tool_error(message)),
        };
        let edge_type = match optional_text("edge_type", params.edge_type) {
            Ok(value) => value,
            Err(message) => return Ok(tool_error(message)),
        };
        match graph::get_node(&self.pool, id).await {
            Ok(_) => {}
            Err(sqlx::Error::RowNotFound) => return Ok(tool_error(format!("entity not found: {id}"))),
            Err(error) => return Ok(tool_error(error.to_string())),
        }
        match graph::list_edges_for_node(&self.pool, id, edge_type.as_deref(), params.current_only).await {
            Ok(relationships) => Ok(json_success(relationships)),
            Err(error) => Ok(tool_error(error.to_string())),
        }
    }

    #[tool(
        description = "Create a Ringmaster graph relationship. With supersede=true, closes prior current relationships sharing from_id and edge_type before inserting this one."
    )]
    async fn create_relationship(
        &self,
        Parameters(params): Parameters<CreateRelationshipParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let from_id = match parse_uuid("from_id", &params.from_id) {
            Ok(value) => value,
            Err(message) => return Ok(tool_error(message)),
        };
        let to_id = match parse_uuid("to_id", &params.to_id) {
            Ok(value) => value,
            Err(message) => return Ok(tool_error(message)),
        };
        let edge_type = match required_text("edge_type", &params.edge_type) {
            Ok(value) => value,
            Err(message) => return Ok(tool_error(message)),
        };
        if params.confidence.is_some_and(|value| !(0.0..=1.0).contains(&value)) {
            return Ok(tool_error("confidence must be between 0.0 and 1.0 when supplied"));
        }
        let valid_from = match parse_optional_rfc3339("valid_from", params.valid_from.as_deref()) {
            Ok(value) => value,
            Err(message) => return Ok(tool_error(message)),
        };
        let id = match graph::create_edge_with_options(
            &self.pool,
            from_id,
            to_id,
            edge_type,
            params.confidence,
            valid_from,
            params.supersede,
        )
        .await
        {
            Ok(id) => id,
            Err(error) => return Ok(tool_error(error.to_string())),
        };
        match graph::get_edge(&self.pool, id).await {
            Ok(relationship) => Ok(json_success(relationship)),
            Err(error) => Ok(tool_error(error.to_string())),
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
