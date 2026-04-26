use radsuite_core::ProjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncOperation {
    Create,
    Update,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalChange {
    pub project_id: ProjectId,
    pub entity_type: String,
    pub entity_id: String,
    pub operation: SyncOperation,
    pub payload: serde_json::Value,
}
