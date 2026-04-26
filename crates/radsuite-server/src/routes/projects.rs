use std::str::FromStr;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use radsuite_core::{
    AddProjectMemberRequest, ApiProjectSummary, CreateProjectRequest, Project, ProjectId,
    ProjectRole,
};

use crate::{
    AppState,
    routes::auth::{ApiError, require_auth},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/projects", post(create_project).get(list_projects))
        .route("/projects/{project_id}", get(get_project))
        .route("/projects/{project_id}/members", post(add_project_member))
        .route("/admin/projects", get(admin_projects))
}

async fn create_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<ApiProjectSummary>), ApiError> {
    let user = require_auth(&headers, &state)?;
    if req.title.trim().is_empty() {
        return Err(ApiError::bad_request("project title is required"));
    }

    let project = Project::new(req.code.unwrap_or_default(), req.title.trim(), user.id);
    let summary = ApiProjectSummary::from_project(&project, ProjectRole::Owner);
    let mut projects = state.projects.lock().expect("project store lock");
    projects.projects.insert(project.id, project.clone());
    projects
        .members
        .entry(project.id)
        .or_default()
        .insert(user.email, ProjectRole::Owner);

    Ok((StatusCode::CREATED, Json(summary)))
}

async fn list_projects(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ApiProjectSummary>>, ApiError> {
    let user = require_auth(&headers, &state)?;
    let projects = state.projects.lock().expect("project store lock");
    let mut summaries = projects
        .projects
        .values()
        .filter_map(|project| {
            let role = projects
                .members
                .get(&project.id)
                .and_then(|members| members.get(&user.email))
                .copied()?;
            Some(ApiProjectSummary::from_project(project, role))
        })
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| left.title.cmp(&right.title));
    Ok(Json(summaries))
}

async fn get_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<ApiProjectSummary>, ApiError> {
    let user = require_auth(&headers, &state)?;
    let project_id = parse_project_id(&project_id)?;
    let projects = state.projects.lock().expect("project store lock");
    let project = projects
        .projects
        .get(&project_id)
        .ok_or_else(|| ApiError::not_found("project not found"))?;
    let role = projects
        .members
        .get(&project_id)
        .and_then(|members| members.get(&user.email))
        .copied()
        .ok_or_else(|| ApiError::forbidden("project access denied"))?;

    Ok(Json(ApiProjectSummary::from_project(project, role)))
}

async fn add_project_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(req): Json<AddProjectMemberRequest>,
) -> Result<Json<ApiProjectSummary>, ApiError> {
    let user = require_auth(&headers, &state)?;
    let project_id = parse_project_id(&project_id)?;
    if req.role == ProjectRole::Owner {
        return Err(ApiError::bad_request("cannot grant owner role"));
    }
    let target_email = req.email.trim().to_lowercase();
    ensure_user_exists(&state, &target_email)?;

    let mut projects = state.projects.lock().expect("project store lock");
    let project = projects
        .projects
        .get(&project_id)
        .cloned()
        .ok_or_else(|| ApiError::not_found("project not found"))?;
    let current_role = projects
        .members
        .get(&project_id)
        .and_then(|members| members.get(&user.email))
        .copied()
        .ok_or_else(|| ApiError::forbidden("project access denied"))?;
    if current_role != ProjectRole::Owner {
        return Err(ApiError::forbidden("only owners can share projects"));
    }

    projects
        .members
        .entry(project_id)
        .or_default()
        .insert(target_email, req.role);

    Ok(Json(ApiProjectSummary::from_project(&project, req.role)))
}

async fn admin_projects(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ApiProjectSummary>>, ApiError> {
    let user = require_auth(&headers, &state)?;
    if !user.is_admin {
        return Err(ApiError::forbidden("admin access required"));
    }

    let projects = state.projects.lock().expect("project store lock");
    let summaries = projects
        .projects
        .values()
        .map(|project| ApiProjectSummary::from_project(project, ProjectRole::Viewer))
        .collect();
    Ok(Json(summaries))
}

fn parse_project_id(value: &str) -> Result<ProjectId, ApiError> {
    ProjectId::from_str(value).map_err(|_| ApiError::bad_request("invalid project id"))
}

fn ensure_user_exists(state: &AppState, email: &str) -> Result<(), ApiError> {
    let auth = state.auth.lock().expect("auth store lock");
    if auth.users_by_email.contains_key(email) {
        Ok(())
    } else {
        Err(ApiError::not_found("user not found"))
    }
}
