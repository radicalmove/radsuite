use serde::{Deserialize, Serialize};

use crate::{Project, ProjectId, ProjectRole};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub display_name: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiProjectSummary {
    pub id: ProjectId,
    pub code: Option<String>,
    pub title: String,
    pub role: ProjectRole,
}

impl ApiProjectSummary {
    pub fn from_project(project: &Project, role: ProjectRole) -> Self {
        Self {
            id: project.id,
            code: project.code.clone(),
            title: project.title.clone(),
            role,
        }
    }
}
