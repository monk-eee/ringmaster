use axum::{
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use sqlx::PgPool;

mod audit_events;
mod candidates;
mod ingestion;
mod nodes;
mod obligations;
mod search;

use audit_events::list_audit_events;
use candidates::{
    accept_candidate, batch_promote_candidates, batch_transition_candidates, correct_candidate,
    extract_source_fragment, list_candidates, promote_candidate, reject_candidate,
};
use ingestion::{get_meeting_candidates, get_meeting_detail, ingest_meeting, ingest_source_route};
use nodes::{create_edge_route, create_node_route, get_node_detail, list_nodes_route, update_node_route};
// ADR-0083: re-exported (not just `use`d) so the ringmaster-ingest binary's
// MCP tool can call the exact same function the HTTP route uses.
pub use nodes::person_brief;
// ADR-0088: a person's completed-obligation history, for a Career/Connect export.
use nodes::person_career_history;
use obligations::{
    daily_brief, focus_blocks, get_obligation_detail, list_obligations, time_horizon,
};
use search::search;

/// Shared `?limit=`/`?offset=` query params for the three list views
/// (ADR-0059): Obligations, Candidates, and (via `NodeQuery`) People. `None`
/// for either fetches every matching row, preserving each route's exact
/// prior behavior for any caller that omits them.
#[derive(Debug, Deserialize)]
struct ListQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

/// Page size ceiling shared by every list-view `limit` param (ADR-0059),
/// matching the value ADR-0049 already established for audit events.
const MAX_LIST_LIMIT: i64 = 200;

/// Clamps a list-view page size to `[1, MAX_LIST_LIMIT]` rather than
/// rejecting it (ADR-0059, matching ADR-0049's audit-limit precedent);
/// `None` stays `None` -- Postgres treats `LIMIT NULL`/`OFFSET NULL` as
/// unbounded, so every existing caller that omits both sees zero change.
fn clamp_list_params(params: &ListQuery) -> (Option<i64>, Option<i64>) {
    (
        params.limit.map(|value| value.clamp(1, MAX_LIST_LIMIT)),
        params.offset.map(|value| value.max(0)),
    )
}

/// Builds the HTTP API: `/health` and the read-only `/api/obligations`
/// (ADR-0012), plus `/api/candidates` and the extraction trigger (ADR-0013),
/// plus the semantic search route (ADR-0019), plus the Daily Brief route
/// (ADR-0022), plus the accept/reject candidate routes (ADR-0024), plus the
/// node/edge write API and traversal routes (ADR-0025), plus the promote
/// route (ADR-0027), plus the Time Horizon route (ADR-0029), plus the
/// Suggested Focus Blocks route (ADR-0031), plus the atomic meeting-ingestion
/// route (ADR-0034), plus the meeting detail read (ADR-0036), plus the
/// meeting-scoped candidate listing route (ADR-0037), plus the dated,
/// any-source-type ingestion route (ADR-0040).
pub fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/obligations", get(list_obligations))
        .route("/api/obligations/:id", get(get_obligation_detail))
        .route("/api/daily-brief", get(daily_brief))
        .route("/api/time-horizon", get(time_horizon))
        .route("/api/focus-blocks", get(focus_blocks))
        .route("/api/meetings/ingest", post(ingest_meeting))
        .route("/api/meetings/:id", get(get_meeting_detail))
        .route("/api/meetings/:id/candidates", get(get_meeting_candidates))
        .route("/api/sources/ingest", post(ingest_source_route))
        .route("/api/candidates", get(list_candidates))
        .route("/api/candidates/batch", post(batch_transition_candidates))
        .route("/api/candidates/:id/accept", post(accept_candidate))
        .route("/api/candidates/:id/reject", post(reject_candidate))
        .route("/api/candidates/:id/correct", post(correct_candidate))
        .route("/api/candidates/:id/promote", post(promote_candidate))
        .route(
            "/api/candidates/batch-promote",
            post(batch_promote_candidates),
        )
        .route(
            "/api/source-fragments/:id/extract",
            post(extract_source_fragment),
        )
        .route("/api/search", get(search))
        .route("/api/audit-events", get(list_audit_events))
        .route("/api/nodes", get(list_nodes_route).post(create_node_route))
        .route(
            "/api/nodes/:id",
            get(get_node_detail).patch(update_node_route),
        )
        .route("/api/people/:id/brief", get(person_brief))
        .route("/api/people/:id/career-export", get(person_career_history))
        .route("/api/edges", post(create_edge_route))
        .with_state(pool)
}

async fn health() -> &'static str {
    "OK"
}

#[cfg(test)]
use sqlx::postgres::PgPoolOptions;

#[cfg(test)]
pub(crate) async fn test_pool() -> PgPool {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to run api tests");
    crate::guard_test_database(&database_url);
    PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("connect to test database")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_route_returns_ok() {
        let pool = test_pool().await;
        let response = app(pool)
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }
}
