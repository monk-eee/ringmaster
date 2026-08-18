use ringmaster_backend::api;
use ringmaster_backend::obligation::rebuild_projection;
use sqlx::postgres::PgPoolOptions;

// ADR-0005/ADR-0007: connects to Postgres, requires pgvector, and applies
// the append-only event-log migrations for the Obligation aggregate.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")?;
    ringmaster_backend::enforce_test_database_if_required(&database_url)?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let projected = rebuild_projection(&pool).await?;
    println!(
        "ringmaster-backend: connected to Postgres, migrations applied, projection rebuilt ({projected} obligation(s))"
    );
    // ADR-0078: build provenance, so a stale container is visible in
    // `podman compose logs` instead of requiring a manual `podman inspect`.
    println!(
        "ringmaster-backend: built from {} ({})",
        env!("RINGMASTER_GIT_SHA"),
        env!("RINGMASTER_GIT_COMMIT_TIME")
    );

    // ADR-0012: serves the read-only HTTP API instead of idling.
    let bind_addr = ringmaster_backend::backend_bind_addr();
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    println!("ringmaster-backend: listening on {bind_addr}");
    axum::serve(listener, api::app(pool)).await?;
    Ok(())
}
