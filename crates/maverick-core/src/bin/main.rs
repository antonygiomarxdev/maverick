use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use maverick_core::{
    adapters::persistence::{CircularUplinkBuffer, SqliteUplinkRepository},
    api::create_app,
    config::RuntimeConfig,
    db::{select_database, BatchWriter},
    events::EventBus,
    host::{build_app_state, spawn_runtime_tasks},
    ports::UplinkRepository,
    storage_profile::StorageProfile,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = RuntimeConfig::from_env()?;

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| config.log_filter.clone().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (db, resolved_storage_profile) = select_database(&config).await?;

    tracing::info!(
        storage_profile = %resolved_storage_profile,
        max_local_storage_mb = config.storage_limits.max_local_storage_mb,
        retention_days = config.storage_limits.retention_days,
        "storage profile resolved"
    );

    let event_bus = EventBus::new(config.event_bus_capacity);
    let db_arc = Arc::new(db);

    let uplink_repo: Arc<dyn UplinkRepository + Send + Sync> = match resolved_storage_profile {
        StorageProfile::Extreme => Arc::new(CircularUplinkBuffer::new(
            config.storage_limits.circular_buffer_capacity,
            event_bus.clone(),
        )),
        _ => {
            let sqlite_repo = Arc::new(SqliteUplinkRepository::new(db_arc.clone()));
            let batch = BatchWriter::new(sqlite_repo, config.storage_limits.batch_commit_size);
            let interval = Duration::from_millis(config.storage_limits.batch_commit_interval_ms);
            batch.clone().spawn_drain_loop(interval);
            Arc::new(batch)
        }
    };

    let state = build_app_state(
        db_arc,
        config.clone(),
        env!("CARGO_PKG_VERSION"),
        event_bus,
        uplink_repo,
    );
    let udp_handle = spawn_runtime_tasks(&state, resolved_storage_profile);
    let app = create_app(state);

    let addr: SocketAddr = config.http_bind_addr.parse().map_err(|err| {
        anyhow::anyhow!("invalid http bind addr '{}': {err}", config.http_bind_addr)
    })?;
    tracing::info!("🚀 Server starting on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tokio::select! {
        result = axum::serve(listener, app) => {
            result?;
        }
        result = udp_handle => {
            result.map_err(|err| anyhow::anyhow!("udp ingester task join error: {err}"))??;
        }
    }

    Ok(())
}
