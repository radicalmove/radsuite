use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use radsuite_db::migrate;
use sqlx::SqlitePool;

use crate::AppConfig;

#[derive(Debug, Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub auth: Arc<Mutex<AuthStore>>,
}

#[derive(Debug, Default)]
pub struct AuthStore {
    pub users_by_email: HashMap<String, AuthUser>,
    pub sessions_by_token: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub email: String,
    pub display_name: String,
    pub password_hash: String,
}

impl AppState {
    pub async fn from_config(config: &AppConfig) -> anyhow::Result<Self> {
        let db = SqlitePool::connect(&config.database_url).await?;
        migrate(&db).await?;
        Ok(Self {
            db,
            auth: Arc::new(Mutex::new(AuthStore::default())),
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
        }
    }
}
