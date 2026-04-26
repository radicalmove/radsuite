use radsuite_core::ProjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncConflict {
    pub project_id: ProjectId,
    pub entity_type: String,
    pub entity_id: String,
    pub local_payload: serde_json::Value,
    pub remote_payload: serde_json::Value,
}
