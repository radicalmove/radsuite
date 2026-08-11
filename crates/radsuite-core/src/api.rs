use serde::{Deserialize, Serialize};

use crate::{Project, ProjectId, ProjectRole, UserId};

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
pub struct ApiUserSummary {
    pub id: UserId,
    pub email: String,
    pub display_name: String,
    pub is_admin: bool,
    pub is_active: bool,
    pub owned_project_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub code: Option<String>,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddProjectMemberRequest {
    pub email: String,
    pub role: ProjectRole,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiProjectSummary {
    pub id: ProjectId,
    pub code: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub structure_mode: String,
    pub archived_at: Option<chrono::DateTime<chrono::Utc>>,
    pub role: ProjectRole,
}

impl ApiProjectSummary {
    pub fn from_project(project: &Project, role: ProjectRole) -> Self {
        Self {
            id: project.id,
            code: project.code.clone(),
            title: project.title.clone(),
            description: project.description.clone(),
            structure_mode: project.structure_mode.clone(),
            archived_at: project.archived_at,
            role,
        }
    }
}
