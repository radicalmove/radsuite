use std::{env, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub bind_addr: String,
    pub database_url: String,
    pub asset_root: PathBuf,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            bind_addr: env::var("RADSUITE_SERVER_BIND")
                .unwrap_or_else(|_| "127.0.0.1:8088".to_string()),
            database_url: env::var("RADSUITE_DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://radsuite-server.sqlite".to_string()),
            asset_root: env::var("RADSUITE_ASSET_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("./data/assets")),
        }
    }

    pub fn test() -> Self {
        Self {
            bind_addr: "127.0.0.1:0".to_string(),
            database_url: "sqlite::memory:".to_string(),
            asset_root: PathBuf::from("./target/test-assets"),
        }
    }
}
