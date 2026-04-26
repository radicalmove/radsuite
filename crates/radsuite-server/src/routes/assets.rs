use std::str::FromStr;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use radsuite_core::ProjectId;
use radsuite_sync::AssetManifest;
use serde::Serialize;

use crate::{
    AppState,
    routes::auth::{ApiError, require_auth},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AssetRegistrationResponse {
    pub upload_required: bool,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/projects/{project_id}/assets", post(register_asset))
        .route("/projects/{project_id}/assets", get(list_assets))
}

async fn register_asset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(manifest): Json<AssetManifest>,
) -> Result<(StatusCode, Json<AssetRegistrationResponse>), ApiError> {
    let user = require_auth(&headers, &state)?;
    let project_id = parse_project_id(&project_id)?;
    ensure_member(&state, project_id, &user.email)?;
    if manifest.project_id != project_id {
        return Err(ApiError::bad_request("asset project id mismatch"));
    }

    state
        .assets
        .lock()
        .expect("asset store lock")
        .manifests
        .entry(project_id)
        .or_default()
        .push(manifest);

    Ok((
        StatusCode::CREATED,
        Json(AssetRegistrationResponse {
            upload_required: true,
        }),
    ))
}

async fn list_assets(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<AssetManifest>>, ApiError> {
    let user = require_auth(&headers, &state)?;
    let project_id = parse_project_id(&project_id)?;
    ensure_member(&state, project_id, &user.email)?;
    let assets = state.assets.lock().expect("asset store lock");
    Ok(Json(
        assets
            .manifests
            .get(&project_id)
            .cloned()
            .unwrap_or_default(),
    ))
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
