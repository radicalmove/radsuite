use radsuite_engines::EngineRegistry;

use crate::AppPaths;

#[derive(Debug, Clone)]
pub struct DesktopState {
    pub app_name: String,
    pub paths: AppPaths,
    pub database_ready: bool,
    pub sync_configured: bool,
    pub engine_registry: EngineRegistry,
}

impl DesktopState {
    pub fn for_tests() -> Self {
        Self {
            app_name: "RADsuite".to_string(),
            paths: AppPaths::for_app("RADsuite").expect("resolve RADsuite app paths"),
            database_ready: true,
            sync_configured: false,
            engine_registry: EngineRegistry::default(),
        }
    }
}
