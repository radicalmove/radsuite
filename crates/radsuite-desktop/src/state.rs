use std::{fs, path::PathBuf};

use radsuite_engines::EngineRegistry;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use thiserror::Error;

use crate::AppPaths;

#[derive(Debug, Clone)]
pub struct DesktopState {
    pub app_name: String,
    pub paths: AppPaths,
    pub database_ready: bool,
    pub sync_configured: bool,
    pub engine_registry: EngineRegistry,
    pub database_pool: SqlitePool,
}

#[derive(Debug, Error)]
pub enum DesktopStateError {
    #[error("could not resolve application directories for {0}")]
    MissingAppDirectories(String),
    #[error("failed to create RADsuite data directory at {path}")]
    CreateDataDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to open RADsuite local database")]
    Database(#[from] sqlx::Error),
    #[error("failed to migrate RADsuite local database")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

impl DesktopState {
    pub async fn for_app(app_name: &str) -> Result<Self, DesktopStateError> {
        let paths = AppPaths::for_app(app_name)
            .ok_or_else(|| DesktopStateError::MissingAppDirectories(app_name.to_string()))?;

        fs::create_dir_all(&paths.data_dir).map_err(|source| DesktopStateError::CreateDataDir {
            path: paths.data_dir.clone(),
            source,
        })?;

        let database_path = paths.data_dir.join("radsuite.sqlite3");
        let connect_options = SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true);
        let database_pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(connect_options)
            .await?;

        radsuite_db::migrate(&database_pool).await?;

        Ok(Self::new(
            app_name.to_string(),
            paths,
            true,
            false,
            database_pool,
        ))
    }

    pub fn for_tests() -> Self {
        let database_pool = SqlitePool::connect_lazy("sqlite::memory:")
            .expect("create lazy in-memory SQLite pool for tests");
        Self::for_tests_with_pool(database_pool)
    }

    pub fn for_tests_with_pool(database_pool: SqlitePool) -> Self {
        Self::new(
            "RADsuite".to_string(),
            AppPaths::for_app("RADsuite").expect("resolve RADsuite app paths"),
            true,
            false,
            database_pool,
        )
    }

    fn new(
        app_name: String,
        paths: AppPaths,
        database_ready: bool,
        sync_configured: bool,
        database_pool: SqlitePool,
    ) -> Self {
        Self {
            app_name,
            paths,
            database_ready,
            sync_configured,
            engine_registry: EngineRegistry::default(),
            database_pool,
        }
    }
}
