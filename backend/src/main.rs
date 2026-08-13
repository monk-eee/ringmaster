use ringmaster_backend::api;
use ringmaster_backend::obligation::rebuild_projection;
use sqlx::postgres::PgPoolOptions;

// ADR-0005/ADR-0007: connects to Postgres, requires pgvector, and applies
// the append-only event-log migrations for the Obligation aggregate.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let projected = rebuild_projection(&pool).await?;
    println!(
        "ringmaster-backend: connected to Postgres, migrations applied, projection rebuilt ({projected} obligation(s))"
    );

    // ADR-0012: serves the read-only HTTP API instead of idling.
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    println!("ringmaster-backend: listening on :8080");
    axum::serve(listener, api::app(pool)).await?;
    Ok(())
}
