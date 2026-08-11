use std::str::FromStr;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use chrono::Utc;
use radsuite_core::{LoginRequest, LoginResponse, RegisterRequest};
use rand_core::OsRng;
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

use crate::{AppState, state::AuthUser};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthenticatedUser {
    pub id: radsuite_core::UserId,
    pub email: String,
    pub is_admin: bool,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
}

async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let email = normalize_email(&req.email)?;
    if req.display_name.trim().is_empty() {
        return Err(ApiError::bad_request("display name is required"));
    }
    if req.password.len() < 12 {
        return Err(ApiError::bad_request(
            "password must be at least 12 characters",
        ));
    }

    let password_hash = hash_password(&req.password)?;
    let user = AuthUser {
        id: radsuite_core::UserId::new(),
        email: email.clone(),
        display_name: req.display_name.trim().to_string(),
        password_hash,
        is_active: true,
        is_admin: false,
    };

    if state
        .auth
        .lock()
        .expect("auth store lock")
        .users_by_email
        .contains_key(&email)
    {
        return Err(ApiError::conflict("user already exists"));
    }

    if load_user(&state, &email).await?.is_some() {
        return Err(ApiError::conflict("user already exists"));
    }

    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        INSERT INTO users
            (id, email, display_name, password_hash, is_active, is_admin, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
        "#,
    )
    .bind(user.id.0.to_string())
    .bind(&user.email)
    .bind(&user.display_name)
    .bind(&user.password_hash)
    .bind(user.is_active)
    .bind(user.is_admin)
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(|_| ApiError::internal("could not save user"))?;

    let token = Uuid::new_v4().to_string();
    let mut auth = state.auth.lock().expect("auth store lock");
    auth.users_by_email.insert(email.clone(), user);
    // Alpha-only in-memory sessions. Persistent sessions or refresh tokens must
    // replace this before external release.
    auth.sessions_by_token.insert(token.clone(), email);

    Ok((StatusCode::CREATED, Json(LoginResponse { token })))
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let email = normalize_email(&req.email)?;
    let cached_user = state
        .auth
        .lock()
        .expect("auth store lock")
        .users_by_email
        .get(&email)
        .cloned();
    let user = match cached_user {
        Some(user) => user,
        None => load_user(&state, &email)
            .await?
            .ok_or_else(ApiError::unauthorized)?,
    };

    if !verify_password(&req.password, &user.password_hash)? {
        return Err(ApiError::unauthorized());
    }
    if !user.is_active {
        return Err(ApiError::unauthorized());
    }

    let token = Uuid::new_v4().to_string();
    let mut auth = state.auth.lock().expect("auth store lock");
    auth.users_by_email.insert(email.clone(), user);
    auth.sessions_by_token.insert(token.clone(), email);
    Ok(Json(LoginResponse { token }))
}

pub(crate) async fn load_user(state: &AppState, email: &str) -> Result<Option<AuthUser>, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT id, email, display_name, password_hash, is_active, is_admin
        FROM users
        WHERE email = ?1
        "#,
    )
    .bind(email)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| ApiError::internal("could not load user"))?;

    row.map(auth_user_from_row).transpose()
}

pub(crate) async fn load_users(state: &AppState) -> Result<Vec<AuthUser>, ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT id, email, display_name, password_hash, is_active, is_admin
        FROM users
        ORDER BY display_name COLLATE NOCASE, email COLLATE NOCASE
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| ApiError::internal("could not load users"))?;

    rows.into_iter().map(auth_user_from_row).collect()
}

fn auth_user_from_row(row: sqlx::sqlite::SqliteRow) -> Result<AuthUser, ApiError> {
    let id = row
        .try_get::<String, _>("id")
        .map_err(|_| ApiError::internal("invalid stored user"))?;
    let id = radsuite_core::UserId::from_str(&id)
        .map_err(|_| ApiError::internal("invalid stored user"))?;
    Ok(AuthUser {
        id,
        email: row
            .try_get("email")
            .map_err(|_| ApiError::internal("invalid stored user"))?,
        display_name: row
            .try_get("display_name")
            .map_err(|_| ApiError::internal("invalid stored user"))?,
        password_hash: row
            .try_get("password_hash")
            .map_err(|_| ApiError::internal("invalid stored user"))?,
        is_active: row
            .try_get("is_active")
            .map_err(|_| ApiError::internal("invalid stored user"))?,
        is_admin: row
            .try_get("is_admin")
            .map_err(|_| ApiError::internal("invalid stored user"))?,
    })
}

fn normalize_email(email: &str) -> Result<String, ApiError> {
    let normalized = email.trim().to_lowercase();
    if normalized.contains('@') {
        Ok(normalized)
    } else {
        Err(ApiError::bad_request("valid email is required"))
    }
}

fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| ApiError::internal("could not hash password"))
}

fn verify_password(password: &str, password_hash: &str) -> Result<bool, ApiError> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|_| ApiError::internal("invalid password hash"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[derive(Debug)]
pub(crate) struct ApiError {
    status: StatusCode,
    message: &'static str,
}

impl ApiError {
    pub(crate) fn bad_request(message: &'static str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message,
        }
    }

    pub(crate) fn conflict(message: &'static str) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message,
        }
    }

    pub(crate) fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: "invalid credentials",
        }
    }

    pub(crate) fn forbidden(message: &'static str) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message,
        }
    }

    pub(crate) fn not_found(message: &'static str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message,
        }
    }

    pub(crate) fn internal(message: &'static str) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorBody {
            error: &'static str,
        }

        (
            self.status,
            Json(ErrorBody {
                error: self.message,
            }),
        )
            .into_response()
    }
}

pub(crate) fn require_auth(
    headers: &HeaderMap,
    state: &AppState,
) -> Result<AuthenticatedUser, ApiError> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(ApiError::unauthorized)?;

    let auth = state.auth.lock().expect("auth store lock");
    let email = auth
        .sessions_by_token
        .get(token)
        .ok_or_else(ApiError::unauthorized)?;
    let user = auth
        .users_by_email
        .get(email)
        .ok_or_else(ApiError::unauthorized)?;

    Ok(AuthenticatedUser {
        id: user.id,
        email: user.email.clone(),
        is_admin: user.is_admin,
    })
}
