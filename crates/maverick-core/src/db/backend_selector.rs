use crate::config::RuntimeConfig;
use crate::db::SqliteDb;
use crate::error::Result;
use crate::storage_profile::StorageProfile;

pub async fn select_database(config: &RuntimeConfig) -> Result<(SqliteDb, StorageProfile)> {
    let profile = config.resolve_storage_profile();

    let db = if config.database_path == ":memory:" || profile == StorageProfile::Extreme {
        SqliteDb::in_memory_with_profile(profile).await?
    } else {
        SqliteDb::new_with_profile(&config.database_path, profile).await?
    };

    Ok((db, profile))
}
