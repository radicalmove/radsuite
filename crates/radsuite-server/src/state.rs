use radsuite_db::migrate;
use sqlx::SqlitePool;

use crate::AppConfig;

#[derive(Debug, Clone)]
pub struct AppState {
    pub db: SqlitePool,
}

impl AppState {
    pub async fn from_config(config: &AppConfig) -> anyhow::Result<Self> {
        let db = SqlitePool::connect(&config.database_url).await?;
        migrate(&db).await?;
        Ok(Self { db })
    }

    pub async fn for_tests() -> Self {
        let db = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("connect test sqlite");
        migrate(&db).await.expect("migrate test sqlite");
        Self { db }
    }
}
