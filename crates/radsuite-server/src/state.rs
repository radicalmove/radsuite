use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use radsuite_core::{Project, ProjectId, ProjectRole, UserId};
use radsuite_db::migrate;
use radsuite_sync::{AssetManifest, LocalChange};
use sqlx::SqlitePool;

use crate::AppConfig;

#[derive(Debug, Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub auth: Arc<Mutex<AuthStore>>,
    pub projects: Arc<Mutex<ProjectStore>>,
    pub assets: Arc<Mutex<AssetStore>>,
    pub sync: Arc<Mutex<SyncStore>>,
}

#[derive(Debug, Default)]
pub struct AuthStore {
    pub users_by_email: HashMap<String, AuthUser>,
    pub sessions_by_token: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: UserId,
    pub email: String,
    pub display_name: String,
    pub password_hash: String,
    pub is_admin: bool,
}

#[derive(Debug, Default)]
pub struct ProjectStore {
    pub projects: HashMap<ProjectId, Project>,
    pub members: HashMap<ProjectId, HashMap<String, ProjectRole>>,
}

#[derive(Debug, Default)]
pub struct AssetStore {
    pub manifests: HashMap<ProjectId, Vec<AssetManifest>>,
}

#[derive(Debug, Default)]
pub struct SyncStore {
    pub records: HashMap<ProjectId, Vec<LocalChange>>,
}

impl AppState {
    pub async fn from_config(config: &AppConfig) -> anyhow::Result<Self> {
        let db = SqlitePool::connect(&config.database_url).await?;
        migrate(&db).await?;
        Ok(Self {
            db,
            auth: Arc::new(Mutex::new(AuthStore::default())),
            projects: Arc::new(Mutex::new(ProjectStore::default())),
            assets: Arc::new(Mutex::new(AssetStore::default())),
            sync: Arc::new(Mutex::new(SyncStore::default())),
        })
    }

    pub async fn for_tests() -> Self {
        let db = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("connect test sqlite");
        migrate(&db).await.expect("migrate test sqlite");
        Self {
            db,
            auth: Arc::new(Mutex::new(AuthStore::default())),
            projects: Arc::new(Mutex::new(ProjectStore::default())),
            assets: Arc::new(Mutex::new(AssetStore::default())),
            sync: Arc::new(Mutex::new(SyncStore::default())),
        }
    }
}
