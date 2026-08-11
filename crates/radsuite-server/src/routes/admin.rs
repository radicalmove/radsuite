use std::str::FromStr;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{delete, get},
};
use radsuite_core::{ApiUserSummary, UserId};

use crate::{
    AppState,
    routes::auth::{ApiError, require_auth},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/users", get(list_users))
        .route("/admin/users/{user_id}", delete(delete_user))
}

async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ApiUserSummary>>, ApiError> {
    require_admin(&headers, &state)?;

    let users_from_db = super::auth::load_users(&state).await?;
    let projects = state.projects.lock().expect("project store lock");
    let mut users = users_from_db
        .iter()
        .map(|user| user_summary(user, &projects))
        .collect::<Vec<_>>();
    users.sort_by(|left, right| {
        left.display_name
            .cmp(&right.display_name)
            .then_with(|| left.email.cmp(&right.email))
    });

    Ok(Json(users))
}

async fn delete_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Result<Json<ApiUserSummary>, ApiError> {
    let current_user = require_admin(&headers, &state)?;
    let user_id =
        UserId::from_str(&user_id).map_err(|_| ApiError::bad_request("invalid user id"))?;
    if user_id == current_user.id {
        return Err(ApiError::forbidden("you cannot delete your own account"));
    }

    let target = super::auth::load_users(&state)
        .await?
        .into_iter()
        .find(|user| user.id == user_id)
        .ok_or_else(|| ApiError::not_found("user not found"))?;
    let target_email = target.email.clone();
    if target.is_admin {
        return Err(ApiError::forbidden(
            "another admin account must be demoted first",
        ));
    }

    let stored_owned_project_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM projects WHERE owner_id = ?1")
            .bind(target.id.0.to_string())
            .fetch_one(&state.db)
            .await
            .map_err(|_| ApiError::internal("could not check project ownership"))?;
    let in_memory_owned_project_count = {
        let projects = state.projects.lock().expect("project store lock");
        projects
            .projects
            .values()
            .filter(|project| project.owner_id == target.id)
            .count()
    };
    if stored_owned_project_count > 0 || in_memory_owned_project_count > 0 {
        return Err(ApiError::conflict(
            "user owns projects; archive or reassign them before deleting the account",
        ));
    }

    sqlx::query("DELETE FROM project_members WHERE user_id = ?1")
        .bind(target.id.0.to_string())
        .execute(&state.db)
        .await
        .map_err(|_| ApiError::internal("could not remove user memberships"))?;
    sqlx::query("DELETE FROM users WHERE id = ?1")
        .bind(target.id.0.to_string())
        .execute(&state.db)
        .await
        .map_err(|_| ApiError::internal("could not delete user"))?;

    let mut auth = state.auth.lock().expect("auth store lock");
    auth.users_by_email.remove(&target_email);
    auth.sessions_by_token
        .retain(|_, email| email != &target_email);
    drop(auth);

    let mut projects = state.projects.lock().expect("project store lock");
    for members in projects.members.values_mut() {
        members.remove(&target_email);
    }

    Ok(Json(ApiUserSummary {
        id: target.id,
        email: target.email,
        display_name: target.display_name,
        is_admin: target.is_admin,
        is_active: target.is_active,
        owned_project_count: 0,
    }))
}

fn require_admin(
    headers: &HeaderMap,
    state: &AppState,
) -> Result<crate::routes::auth::AuthenticatedUser, ApiError> {
    let user = require_auth(headers, state)?;
    if !user.is_admin {
        return Err(ApiError::forbidden("admin access required"));
    }
    Ok(user)
}

fn user_summary(
    user: &crate::state::AuthUser,
    projects: &crate::state::ProjectStore,
) -> ApiUserSummary {
    ApiUserSummary {
        id: user.id,
        email: user.email.clone(),
        display_name: user.display_name.clone(),
        is_admin: user.is_admin,
        is_active: user.is_active,
        owned_project_count: projects
            .projects
            .values()
            .filter(|project| project.owner_id == user.id)
            .count(),
    }
}
