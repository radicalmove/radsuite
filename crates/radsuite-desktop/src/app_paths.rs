use std::path::PathBuf;

use directories::ProjectDirs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl AppPaths {
    pub fn for_app(app_name: &str) -> Option<Self> {
        let dirs = ProjectDirs::from("nz", "RADsuite", app_name)?;
        Some(Self {
            data_dir: dirs.data_dir().to_path_buf(),
            config_dir: dirs.config_dir().to_path_buf(),
            cache_dir: dirs.cache_dir().to_path_buf(),
        })
    }
}
