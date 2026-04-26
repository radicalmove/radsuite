use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{ProjectId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub owner_id: UserId,
    pub code: Option<String>,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Project {
    pub fn new(code: impl Into<String>, title: impl Into<String>, owner_id: UserId) -> Self {
        let now = Utc::now();
        let code = code.into();
        Self {
            id: ProjectId::new(),
            owner_id,
            code: (!code.trim().is_empty()).then_some(code),
            title: title.into(),
            created_at: now,
            updated_at: now,
        }
    }
}
