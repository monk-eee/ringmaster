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
    pub occurred_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone)]
pub struct EntityUpsert {
    pub node_type: String,
    pub canonical_text: String,
    pub lifecycle_state: Option<String>,
    pub attributes: Option<Json>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EntityUpsertResult {
    pub action: String,
    pub node: Node,
}

#[derive(Debug)]
pub enum UpsertNodesError {
    InvalidInput(String),
    AmbiguousIdentity {
        node_type: String,
        canonical_text: String,
        matches: usize,
    },
    Database(sqlx::Error),
}

impl std::fmt::Display for UpsertNodesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(message) => formatter.write_str(message),
            Self::AmbiguousIdentity { node_type, canonical_text, matches } => write!(
                formatter,
                "exact identity ({node_type:?}, {canonical_text:?}) matched {matches} nodes; update the intended entity by id instead"
            ),
            Self::Database(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for UpsertNodesError {}

impl From<sqlx::Error> for UpsertNodesError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
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
    sqlx::query_as("SELECT id, node_type, canonical_text, attributes, lifecycle_state, occurred_at FROM nodes WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
}

/// Lists nodes, optionally filtered by `node_type` and/or an `occurred_at`
/// range (ADR-0025, ADR-0042), and/or (ADR-0051) restricted to those with
/// at least one linked `open`/`at_risk` Obligation when `needs_attention`
/// is `true`. Read-only. A NULL filter argument is a no-op in SQL (`$n IS
/// NULL OR ...`), so every combination of filters is one query, not eight
/// branches. Ordering switches to `occurred_at DESC NULLS LAST` when either
/// date bound is given -- sorting by write-time makes no sense once the
/// caller is asking "what happened in this window"; otherwise unchanged
/// (`updated_at DESC`). `limit`/`offset` of `None` fetch every matching row
/// unchanged (ADR-0059) -- Postgres treats `LIMIT NULL`/`OFFSET NULL` as
/// unbounded, so this is one query either way, not a conditionally-built one.
pub async fn list_nodes(
    pool: &PgPool,
    node_type: Option<&str>,
    occurred_from: Option<chrono::DateTime<chrono::Utc>>,
    occurred_to: Option<chrono::DateTime<chrono::Utc>>,
    needs_attention: bool,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Node>, sqlx::Error> {
    list_nodes_filtered(
        pool,
        node_type,
        None,
        occurred_from,
        occurred_to,
        needs_attention,
        false,
        limit,
        offset,
    )
    .await
}

/// The MCP graph surface (ADR-0066) also needs exact canonical-text lookup.
/// Existing callers keep using `list_nodes`; this fuller form is additive.
#[allow(clippy::too_many_arguments)]
pub async fn list_nodes_filtered(
    pool: &PgPool,
    node_type: Option<&str>,
    canonical_text: Option<&str>,
    occurred_from: Option<chrono::DateTime<chrono::Utc>>,
    occurred_to: Option<chrono::DateTime<chrono::Utc>>,
    needs_attention: bool,
    has_source_fragments: bool,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Node>, sqlx::Error> {
    let order_by = if occurred_from.is_some() || occurred_to.is_some() {
        "ORDER BY occurred_at DESC NULLS LAST"
    } else {
        "ORDER BY updated_at DESC"
    };
    let query = format!(
        "SELECT id, node_type, canonical_text, attributes, lifecycle_state, occurred_at FROM nodes n \
         WHERE ($1::text IS NULL OR node_type = $1) \
                     AND ($2::text IS NULL OR canonical_text = $2) \
                     AND ($3::timestamptz IS NULL OR occurred_at >= $3) \
                     AND ($4::timestamptz IS NULL OR occurred_at <= $4) \
                     AND ($5::boolean IS NOT TRUE OR EXISTS ( \
                SELECT 1 FROM edges e \
                JOIN obligation_projection op ON op.obligation_id = (CASE WHEN e.from_id = n.id THEN e.to_id ELSE e.from_id END) \
                WHERE (e.from_id = n.id OR e.to_id = n.id) AND op.status IN ('open', 'at_risk') \
           )) \
                     AND ($6::boolean IS NOT TRUE OR EXISTS ( \
                SELECT 1 FROM source_fragments sf WHERE sf.source_id = n.id \
           )) \
         {order_by} \
                 LIMIT $7 OFFSET $8"
    );
    sqlx::query_as(&query)
        .bind(node_type)
        .bind(canonical_text)
        .bind(occurred_from)
        .bind(occurred_to)
        .bind(needs_attention)
        .bind(has_source_fragments)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
}

/// Atomically creates or enriches a bounded set of exact graph identities
/// (ADR-0066). Matching is deliberately case-sensitive and limited to the
/// trimmed `(node_type, canonical_text)` pair. Advisory locks serialize
/// concurrent calls for the same identity without imposing a unique index on
/// historical data that may already contain duplicates.
pub async fn upsert_nodes(
    pool: &PgPool,
    entities: Vec<EntityUpsert>,
) -> Result<Vec<EntityUpsertResult>, UpsertNodesError> {
    if entities.is_empty() || entities.len() > 100 {
        return Err(UpsertNodesError::InvalidInput(
            "entities must contain between 1 and 100 items".to_string(),
        ));
    }

    let mut transaction = pool.begin().await?;
    let mut results = Vec::with_capacity(entities.len());

    for entity in entities {
        let node_type = entity.node_type.trim();
        let canonical_text = entity.canonical_text.trim();
        if node_type.is_empty() {
            return Err(UpsertNodesError::InvalidInput(
                "node_type must not be blank".to_string(),
            ));
        }
        if canonical_text.is_empty() {
            return Err(UpsertNodesError::InvalidInput(
                "canonical_text must not be blank".to_string(),
            ));
        }
        if entity
            .lifecycle_state
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(UpsertNodesError::InvalidInput(
                "lifecycle_state must not be blank when supplied".to_string(),
            ));
        }
        if entity
            .attributes
            .as_ref()
            .is_some_and(|value| !value.is_object())
        {
            return Err(UpsertNodesError::InvalidInput(
                "attributes must be a JSON object when supplied".to_string(),
            ));
        }

        sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1), hashtext($2))")
            .bind(node_type)
            .bind(canonical_text)
            .execute(&mut *transaction)
            .await?;

        let matches: Vec<Node> = sqlx::query_as(
            "SELECT id, node_type, canonical_text, attributes, lifecycle_state, occurred_at \
             FROM nodes WHERE node_type = $1 AND canonical_text = $2 FOR UPDATE",
        )
        .bind(node_type)
        .bind(canonical_text)
        .fetch_all(&mut *transaction)
        .await?;

        let (action, node) = match matches.as_slice() {
            [] => {
                let node: Node = sqlx::query_as(
                    "INSERT INTO nodes (node_type, canonical_text, attributes, lifecycle_state) \
                     VALUES ($1, $2, COALESCE($3, '{}'::jsonb), COALESCE($4, 'active')) \
                     RETURNING id, node_type, canonical_text, attributes, lifecycle_state, occurred_at",
                )
                .bind(node_type)
                .bind(canonical_text)
                .bind(entity.attributes.as_ref())
                .bind(entity.lifecycle_state.as_deref().map(str::trim))
                .fetch_one(&mut *transaction)
                .await?;
                ("created", node)
            }
            [_] => {
                let node: Node = sqlx::query_as(
                    "UPDATE nodes SET \
                        attributes = attributes || COALESCE($2, '{}'::jsonb), \
                        lifecycle_state = COALESCE($3, lifecycle_state), \
                        updated_at = now() \
                     WHERE id = $1 \
                     RETURNING id, node_type, canonical_text, attributes, lifecycle_state, occurred_at",
                )
                .bind(matches[0].id)
                .bind(entity.attributes.as_ref())
                .bind(entity.lifecycle_state.as_deref().map(str::trim))
                .fetch_one(&mut *transaction)
                .await?;
                ("updated", node)
            }
            duplicates => {
                return Err(UpsertNodesError::AmbiguousIdentity {
                    node_type: node_type.to_string(),
                    canonical_text: canonical_text.to_string(),
                    matches: duplicates.len(),
                })
            }
        };

        results.push(EntityUpsertResult {
            action: action.to_string(),
            node,
        });
    }

    transaction.commit().await?;
    Ok(results)
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
         RETURNING id, node_type, canonical_text, attributes, lifecycle_state, occurred_at",
    )
    .bind(id)
    .bind(canonical_text)
    .bind(lifecycle_state)
    .bind(attributes)
    .fetch_one(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::create_edge;
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

        assert_eq!(
            node.canonical_text, "Roopa",
            "unspecified canonical_text must be unchanged"
        );
        assert_eq!(
            node.attributes["role"], "manager",
            "a previously-recorded attribute must survive enrichment"
        );
        assert_eq!(
            node.attributes["team"], "platform",
            "the newly-enriched attribute must be present"
        );
    }

    #[tokio::test]
    async fn list_nodes_filters_by_node_type() {
        let pool = test_pool().await;
        let person_id = create_node(&pool, "person", "Filter Test Person", json!({}))
            .await
            .expect("create person");
        create_node(&pool, "risk", "Filter Test Risk", json!({}))
            .await
            .expect("create risk");

        let people = list_nodes(&pool, Some("person"), None, None, false, None, None)
            .await
            .expect("list people");
        assert!(people.iter().any(|node| node.id == person_id));
        assert!(
            people.iter().all(|node| node.node_type == "person"),
            "filter must exclude other node types"
        );
    }

    /// ADR-0059: `limit`/`offset` of `None` preserve every existing test's
    /// exact prior behavior (every matching row); a real `limit` truncates
    /// and `offset` skips, both against a stable `updated_at DESC` order.
    #[tokio::test]
    async fn list_nodes_applies_limit_and_offset() {
        let pool = test_pool().await;
        let first = create_node(&pool, "note", "Pagination Test First", json!({}))
            .await
            .expect("create first");
        let second = create_node(&pool, "note", "Pagination Test Second", json!({}))
            .await
            .expect("create second");

        let unbounded = list_nodes(&pool, None, None, None, false, None, None)
            .await
            .expect("list unbounded");
        let both_present = unbounded.iter().any(|node| node.id == first)
            && unbounded.iter().any(|node| node.id == second);
        assert!(
            both_present,
            "omitting limit/offset must return every matching row"
        );

        let page = list_nodes(&pool, None, None, None, false, Some(1), None)
            .await
            .expect("list first page");
        assert_eq!(page.len(), 1, "limit=1 must return exactly one row");
        assert_eq!(
            page[0].id, second,
            "updated_at DESC must put the most recently created row first"
        );

        let next_page = list_nodes(&pool, None, None, None, false, Some(1), Some(1))
            .await
            .expect("list second page");
        assert_eq!(
            next_page.len(),
            1,
            "limit=1 offset=1 must return exactly one row"
        );
        assert_eq!(
            next_page[0].id, first,
            "offset=1 must skip the first page's row"
        );
    }

    /// ADR-0051: `needs_attention` restricts to person nodes with at least
    /// one linked open/at_risk Obligation; a person with no such link is
    /// excluded, and a closed-only link does not count.
    #[tokio::test]
    async fn list_nodes_needs_attention_filters_to_linked_open_or_at_risk_obligations() {
        let pool = test_pool().await;
        let owed_person = create_node(&pool, "person", "Needs Attention Test Owed", json!({}))
            .await
            .expect("create owed person");
        let idle_person = create_node(&pool, "person", "Needs Attention Test Idle", json!({}))
            .await
            .expect("create idle person");
        let closed_only_person = create_node(
            &pool,
            "person",
            "Needs Attention Test Closed Only",
            json!({}),
        )
        .await
        .expect("create closed-only person");

        let open_obligation = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            open_obligation,
            crate::obligation::ObligationEventType::Created,
            serde_json::json!({"status": "open"}),
        )
        .await
        .expect("append open obligation");
        create_edge(&pool, owed_person, open_obligation, "owns", None)
            .await
            .expect("link owed person");

        let closed_obligation = uuid::Uuid::new_v4();
        crate::obligation::append_event(
            &pool,
            closed_obligation,
            crate::obligation::ObligationEventType::Created,
            serde_json::json!({"status": "open"}),
        )
        .await
        .expect("append obligation to close");
        crate::obligation::append_event(
            &pool,
            closed_obligation,
            crate::obligation::ObligationEventType::Closed,
            serde_json::json!({}),
        )
        .await
        .expect("close it");
        create_edge(&pool, closed_only_person, closed_obligation, "owns", None)
            .await
            .expect("link closed-only person");

        crate::obligation::rebuild_projection(&pool)
            .await
            .expect("rebuild projection");

        let filtered = list_nodes(&pool, Some("person"), None, None, true, None, None)
            .await
            .expect("list people needing attention");
        assert!(
            filtered.iter().any(|node| node.id == owed_person),
            "a person linked to an open Obligation must be included"
        );
        assert!(
            !filtered.iter().any(|node| node.id == idle_person),
            "a person with no linked Obligation must be excluded"
        );
        assert!(
            !filtered.iter().any(|node| node.id == closed_only_person),
            "a person linked only to a closed Obligation must be excluded"
        );
    }

    /// ADR-0096: the type-agnostic replacement for a fixed `node_type`
    /// allowlist -- proves it works for a non-"meeting" source type too.
    #[tokio::test]
    async fn list_nodes_filtered_has_source_fragments_matches_any_source_type() {
        let pool = test_pool().await;
        let with_fragment = create_node(&pool, "1on1", "Has A Fragment", json!({}))
            .await
            .expect("create source node");
        crate::graph::create_source_fragment(&pool, with_fragment, "some text", "hash-1")
            .await
            .expect("create source fragment");
        let without_fragment = create_node(&pool, "1on1", "Has No Fragment", json!({}))
            .await
            .expect("create fragment-less node");

        let filtered = list_nodes_filtered(&pool, None, None, None, None, false, true, None, None)
            .await
            .expect("list nodes with source fragments");
        assert!(
            filtered.iter().any(|node| node.id == with_fragment),
            "a node with at least one source fragment must be included, regardless of node_type"
        );
        assert!(
            !filtered.iter().any(|node| node.id == without_fragment),
            "a node with zero source fragments must be excluded"
        );
    }

    #[tokio::test]
    async fn list_nodes_filters_by_occurred_at_range_and_orders_by_it() {
        let pool = test_pool().await;
        let in_range = create_node(&pool, "note", "In Range Note", json!({}))
            .await
            .expect("create in-range node");
        let out_of_range = create_node(&pool, "note", "Out Of Range Note", json!({}))
            .await
            .expect("create out-of-range node");

        let from = chrono::Utc::now() - chrono::Duration::days(2);
        let to = chrono::Utc::now() - chrono::Duration::days(1);
        sqlx::query("UPDATE nodes SET occurred_at = $2 WHERE id = $1")
            .bind(in_range)
            .bind(from + chrono::Duration::hours(12))
            .execute(&pool)
            .await
            .expect("set in-range occurred_at");
        sqlx::query("UPDATE nodes SET occurred_at = $2 WHERE id = $1")
            .bind(out_of_range)
            .bind(chrono::Utc::now())
            .execute(&pool)
            .await
            .expect("set out-of-range occurred_at");

        let results = list_nodes(&pool, None, Some(from), Some(to), false, None, None)
            .await
            .expect("list nodes by range");
        assert!(
            results.iter().any(|node| node.id == in_range),
            "in-range node must be present"
        );
        assert!(
            !results.iter().any(|node| node.id == out_of_range),
            "out-of-range node must be excluded"
        );
    }
}
