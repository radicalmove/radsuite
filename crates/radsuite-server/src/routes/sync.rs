use std::str::FromStr;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
};
use radsuite_core::ProjectId;
use radsuite_sync::LocalChange;
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    routes::auth::{ApiError, require_auth},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncPushRequest {
    pub changes: Vec<LocalChange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncPushResponse {
    pub accepted: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncPullResponse {
    pub records: Vec<LocalChange>,
    pub next_cursor: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PullQuery {
    #[serde(default)]
    pub after: usize,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/projects/{project_id}/sync/push", post(push_sync))
        .route("/projects/{project_id}/sync/pull", get(pull_sync))
}

async fn push_sync(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(req): Json<SyncPushRequest>,
) -> Result<Json<SyncPushResponse>, ApiError> {
    let user = require_auth(&headers, &state)?;
    let project_id = parse_project_id(&project_id)?;
    ensure_member(&state, project_id, &user.email)?;
    if req
        .changes
        .iter()
        .any(|change| change.project_id != project_id)
    {
        return Err(ApiError::bad_request("sync project id mismatch"));
    }

    let accepted = req.changes.len();
    state
        .sync
        .lock()
        .expect("sync store lock")
        .records
        .entry(project_id)
        .or_default()
        .extend(req.changes);
    Ok(Json(SyncPushResponse { accepted }))
}

async fn pull_sync(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Query(query): Query<PullQuery>,
) -> Result<Json<SyncPullResponse>, ApiError> {
    let user = require_auth(&headers, &state)?;
    let project_id = parse_project_id(&project_id)?;
    ensure_member(&state, project_id, &user.email)?;
    let sync = state.sync.lock().expect("sync store lock");
    let all_records = sync.records.get(&project_id).cloned().unwrap_or_default();
    let records = all_records
        .iter()
        .skip(query.after)
        .cloned()
        .collect::<Vec<_>>();
    Ok(Json(SyncPullResponse {
        records,
        next_cursor: all_records.len(),
    }))
}

fn parse_project_id(value: &str) -> Result<ProjectId, ApiError> {
    ProjectId::from_str(value).map_err(|_| ApiError::bad_request("invalid project id"))
}

fn ensure_member(state: &AppState, project_id: ProjectId, email: &str) -> Result<(), ApiError> {
    let projects = state.projects.lock().expect("project store lock");
    if !projects.projects.contains_key(&project_id) {
        return Err(ApiError::not_found("project not found"));
    }
    if projects
        .members
        .get(&project_id)
        .and_then(|members| members.get(email))
        .is_some()
    {
        Ok(())
    } else {
        Err(ApiError::forbidden("project access denied"))
    }
}
