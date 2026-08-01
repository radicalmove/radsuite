use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use radsuite_engines::EngineRegistry;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use thiserror::Error;

use crate::AppPaths;

use crate::radcast::{RadcastAudioOutput, RadcastProcessingPhase, RadcastProcessingProgress};
use crate::radt_ts::{
    RadtTsChildHandle, RadtTsJobStatus, RadtTsLifecycleRegistry, RadtTsLifecycleState,
};
use crate::radt_ts_tools::{RadtTsMediaChildHandle, RadtTsMediaJobStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadcastJobState {
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RadcastJobStatus {
    pub id: String,
    pub state: RadcastJobState,
    pub phase: RadcastProcessingPhase,
    pub percent: u8,
    pub elapsed_seconds: f64,
    pub output: Option<RadcastAudioOutput>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DesktopState {
    pub app_name: String,
    pub paths: AppPaths,
    pub database_ready: bool,
    pub sync_configured: bool,
    pub engine_registry: EngineRegistry,
    pub database_pool: SqlitePool,
    pub radcast_jobs: Arc<Mutex<HashMap<String, RadcastJobStatus>>>,
    pub radcast_cancel_requests: Arc<Mutex<HashSet<String>>>,
    pub radt_ts_jobs: Arc<Mutex<HashMap<String, RadtTsJobStatus>>>,
    pub radt_ts_children: Arc<Mutex<HashMap<String, RadtTsChildHandle>>>,
    pub radt_ts_cancel_requests: Arc<Mutex<HashSet<String>>>,
    pub radt_ts_active_projects: Arc<Mutex<HashSet<String>>>,
    pub radt_ts_lifecycle: RadtTsLifecycleRegistry,
    pub radt_ts_media_jobs: Arc<Mutex<HashMap<String, RadtTsMediaJobStatus>>>,
    pub radt_ts_media_children: Arc<Mutex<HashMap<String, RadtTsMediaChildHandle>>>,
    pub radt_ts_media_cancel_requests: Arc<Mutex<HashSet<String>>>,
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
            radcast_jobs: Arc::new(Mutex::new(HashMap::new())),
            radcast_cancel_requests: Arc::new(Mutex::new(HashSet::new())),
            radt_ts_jobs: Arc::new(Mutex::new(HashMap::new())),
            radt_ts_children: Arc::new(Mutex::new(HashMap::new())),
            radt_ts_cancel_requests: Arc::new(Mutex::new(HashSet::new())),
            radt_ts_active_projects: Arc::new(Mutex::new(HashSet::new())),
            radt_ts_lifecycle: Arc::new(Mutex::new(RadtTsLifecycleState::default())),
            radt_ts_media_jobs: Arc::new(Mutex::new(HashMap::new())),
            radt_ts_media_children: Arc::new(Mutex::new(HashMap::new())),
            radt_ts_media_cancel_requests: Arc::new(Mutex::new(HashSet::new())),
        }
    }
}

impl RadcastJobStatus {
    pub fn running(id: String) -> Self {
        Self {
            id,
            state: RadcastJobState::Running,
            phase: RadcastProcessingPhase::Preparing,
            percent: 0,
            elapsed_seconds: 0.0,
            output: None,
            error: None,
        }
    }

    pub fn update_progress(&mut self, progress: RadcastProcessingProgress, elapsed_seconds: f64) {
        self.phase = progress.phase;
        self.percent = progress.percent;
        self.elapsed_seconds = elapsed_seconds;
    }
}
