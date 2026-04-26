use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineStatus {
    pub id: String,
    pub label: String,
    pub available: bool,
    pub detail: String,
}
