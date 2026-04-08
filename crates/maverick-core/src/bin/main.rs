use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use maverick_core::{api::{create_app, AppState}, db::{SqliteDb, Database}};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "maverick_core=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = SqliteDb::in_memory()?;
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS health_check (id INTEGER PRIMARY KEY);",
    )
    .await?;

    let state = AppState::new(db, env!("CARGO_PKG_VERSION"));
    let app = create_app(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("🚀 Server starting on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}