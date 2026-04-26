use radsuite_core::ProjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetSyncPolicy {
    CollaborativeSource,
    ExplicitSharedOutput,
    LocalOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetManifest {
    pub project_id: ProjectId,
    pub sha256: String,
    pub byte_size: u64,
    pub mime_type: String,
    pub original_name: String,
    pub sync_policy: AssetSyncPolicy,
}
