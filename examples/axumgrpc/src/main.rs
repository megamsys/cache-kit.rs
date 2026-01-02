use axumgrpc::AppState;
use cache_kit::backend::InMemoryBackend;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    let _ = {
        use tracing_subscriber::fmt;
        fmt().try_init()
    };

    // Database setup
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost:5432/cache_kit".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    tracing::info!("Migrations completed");

    // Cache backend
    let cache_backend = Arc::new(InMemoryBackend::new());

    let state = AppState::new(pool, cache_backend);

    // Start gRPC server (gRPC-only, no REST)
    tracing::info!("Starting gRPC-only server...");
    axumgrpc::start_grpc_server(state).await?;

    Ok(())
}
