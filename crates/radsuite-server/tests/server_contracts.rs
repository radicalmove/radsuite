use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use radsuite_core::{ApiProjectSummary, ApiUserSummary, LoginResponse, ProjectId, ProjectRole};
use radsuite_server::{AppConfig, AppState, build_router};
use radsuite_sync::{AssetManifest, AssetSyncPolicy, LocalChange, SyncOperation};
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let state = AppState::for_tests().await;
    let app = build_router(state, AppConfig::test());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn auth_register_creates_user_for_internal_alpha() {
    let state = AppState::for_tests().await;
    let app = build_router(state, AppConfig::test());

    let response = app
        .oneshot(json_request(
            Method::POST,
            "/auth/register",
            json!({
                "email": "owner@example.com",
                "display_name": "Owner",
                "password": "correct horse battery staple"
            }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn auth_login_returns_session_token_for_correct_credentials() {
    let state = AppState::for_tests().await;
    let app = build_router(state, AppConfig::test());

    let app = register_owner(app).await;
    let response = app
        .oneshot(json_request(
            Method::POST,
            "/auth/login",
            json!({
                "email": "owner@example.com",
                "password": "correct horse battery staple"
            }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let login: LoginResponse = json_response(response).await;
    assert!(!login.token.is_empty());
}

#[tokio::test]
async fn auth_login_rejects_bad_credentials() {
    let state = AppState::for_tests().await;
    let app = build_router(state, AppConfig::test());

    let app = register_owner(app).await;
    let response = app
        .oneshot(json_request(
            Method::POST,
            "/auth/login",
            json!({
                "email": "owner@example.com",
                "password": "wrong password"
            }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_can_list_users_and_regular_users_cannot() {
    let state = AppState::for_tests().await;
    let app = build_router(state.clone(), AppConfig::test());

    let (app, admin_token) = register_user(app, "admin@example.com").await;
    let (app, _) = register_user(app, "member@example.com").await;
    promote_to_admin(&state, "admin@example.com").await;

    let response = app
        .clone()
        .oneshot(bearer_request(Method::GET, "/admin/users", &admin_token))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let users: Vec<ApiUserSummary> = json_response(response).await;
    assert_eq!(users.len(), 2);
    assert!(
        users
            .iter()
            .any(|user| user.email == "admin@example.com" && user.is_admin)
    );
    assert!(
        users
            .iter()
            .any(|user| user.email == "member@example.com" && !user.is_admin)
    );

    let member_token = login_user(app.clone(), "member@example.com").await;
    let response = app
        .oneshot(bearer_request(Method::GET, "/admin/users", &member_token))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_delete_user_revokes_sessions_and_removes_membership() {
    let state = AppState::for_tests().await;
    let app = build_router(state.clone(), AppConfig::test());

    let (app, admin_token) = register_user(app, "admin@example.com").await;
    let (app, member_token) = register_user(app, "member@example.com").await;
    promote_to_admin(&state, "admin@example.com").await;
    let member_id = user_id(&state, "member@example.com");
    let project = create_project(app.clone(), &admin_token).await;
    let share_response = app
        .clone()
        .oneshot(bearer_json_request(
            Method::POST,
            &format!("/projects/{}/members", project.id.0),
            &admin_token,
            json!({
                "email": "member@example.com",
                "role": "viewer"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(share_response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(bearer_request(
            Method::DELETE,
            &format!("/admin/users/{member_id}"),
            &admin_token,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let deleted: ApiUserSummary = json_response(response).await;
    assert_eq!(deleted.id, member_id);

    let response = app
        .clone()
        .oneshot(bearer_request(Method::GET, "/admin/users", &admin_token))
        .await
        .unwrap();
    let users: Vec<ApiUserSummary> = json_response(response).await;
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].email, "admin@example.com");

    state
        .auth
        .lock()
        .expect("auth store lock")
        .users_by_email
        .clear();
    let response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/auth/login",
            json!({
                "email": "member@example.com",
                "password": "correct horse battery staple"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let response = app
        .oneshot(bearer_request(Method::GET, "/projects", &member_token))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(
        !state
            .projects
            .lock()
            .expect("project store lock")
            .members
            .values()
            .any(|members| members.contains_key("member@example.com"))
    );
}

#[tokio::test]
async fn admin_cannot_delete_self_or_a_project_owner() {
    let state = AppState::for_tests().await;
    let app = build_router(state.clone(), AppConfig::test());

    let (app, admin_token) = register_user(app, "admin@example.com").await;
    let (app, owner_token) = register_user(app, "owner@example.com").await;
    promote_to_admin(&state, "admin@example.com").await;
    let admin_id = user_id(&state, "admin@example.com");
    let owner_id = user_id(&state, "owner@example.com");
    let _project = create_project(app.clone(), &owner_token).await;

    let response = app
        .clone()
        .oneshot(bearer_request(
            Method::DELETE,
            &format!("/admin/users/{admin_id}"),
            &admin_token,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = app
        .oneshot(bearer_request(
            Method::DELETE,
            &format!("/admin/users/{owner_id}"),
            &admin_token,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn admin_cannot_delete_user_with_a_persisted_project() {
    let state = AppState::for_tests().await;
    let app = build_router(state.clone(), AppConfig::test());

    let (app, admin_token) = register_user(app, "admin@example.com").await;
    let (app, _) = register_user(app, "owner@example.com").await;
    promote_to_admin(&state, "admin@example.com").await;
    let owner_id = user_id(&state, "owner@example.com");
    let project_id = ProjectId::new();
    sqlx::query(
        "INSERT INTO projects (id, owner_id, code, title, description, structure_mode, archived_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, NULL, ?5, NULL, ?6, ?6)",
    )
    .bind(project_id.0.to_string())
    .bind(owner_id.0.to_string())
    .bind("COMS435")
    .bind("Persisted project")
    .bind("modules")
    .bind("2026-01-01T00:00:00Z")
    .execute(&state.db)
    .await
    .expect("insert persisted project");

    let response = app
        .oneshot(bearer_request(
            Method::DELETE,
            &format!("/admin/users/{}", owner_id.0),
            &admin_token,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn registered_users_can_be_loaded_again_after_auth_cache_reset() {
    let state = AppState::for_tests().await;
    let app = build_router(state.clone(), AppConfig::test());
    let (app, _) = register_user(app, "persisted@example.com").await;

    {
        let mut auth = state.auth.lock().expect("auth store lock");
        auth.users_by_email.clear();
        auth.sessions_by_token.clear();
    }

    let response = app
        .oneshot(json_request(
            Method::POST,
            "/auth/login",
            json!({
                "email": "persisted@example.com",
                "password": "correct horse battery staple"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn project_authenticated_user_can_create_and_list_project() {
    let state = AppState::for_tests().await;
    let app = build_router(state, AppConfig::test());

    let (app, token) = register_user(app, "owner@example.com").await;
    let create_response = app
        .clone()
        .oneshot(bearer_json_request(
            Method::POST,
            "/projects",
            &token,
            json!({
                "code": "COMS435",
                "title": "Good data and how to use it"
            }),
        ))
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::CREATED);
    let created: ApiProjectSummary = json_response(create_response).await;
    assert_eq!(created.role, ProjectRole::Owner);

    let list_response = app
        .oneshot(bearer_request(Method::GET, "/projects", &token))
        .await
        .unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let projects: Vec<ApiProjectSummary> = json_response(list_response).await;
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id, created.id);
}

#[tokio::test]
async fn project_non_member_cannot_read_project() {
    let state = AppState::for_tests().await;
    let app = build_router(state, AppConfig::test());

    let (app, owner_token) = register_user(app, "owner@example.com").await;
    let (app, other_token) = register_user(app, "other@example.com").await;
    let created = create_project(app.clone(), &owner_token).await;
    let response = app
        .oneshot(bearer_request(
            Method::GET,
            &format!("/projects/{}", created.id.0),
            &other_token,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn project_owner_can_share_project_with_editor() {
    let state = AppState::for_tests().await;
    let app = build_router(state, AppConfig::test());

    let (app, owner_token) = register_user(app, "owner@example.com").await;
    let (app, editor_token) = register_user(app, "editor@example.com").await;
    let created = create_project(app.clone(), &owner_token).await;

    let share_response = app
        .clone()
        .oneshot(bearer_json_request(
            Method::POST,
            &format!("/projects/{}/members", created.id.0),
            &owner_token,
            json!({
                "email": "editor@example.com",
                "role": "editor"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(share_response.status(), StatusCode::OK);

    let list_response = app
        .oneshot(bearer_request(Method::GET, "/projects", &editor_token))
        .await
        .unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let projects: Vec<ApiProjectSummary> = json_response(list_response).await;
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id, created.id);
    assert_eq!(projects[0].role, ProjectRole::Editor);
}

#[tokio::test]
async fn asset_project_member_can_register_manifest() {
    let state = AppState::for_tests().await;
    let app = build_router(state, AppConfig::test());

    let (app, token) = register_user(app, "owner@example.com").await;
    let project = create_project(app.clone(), &token).await;
    let response = app
        .oneshot(bearer_json_request(
            Method::POST,
            &format!("/projects/{}/assets", project.id.0),
            &token,
            serde_json::to_value(sample_asset(project.id)).unwrap(),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body: serde_json::Value = json_response(response).await;
    assert_eq!(body["upload_required"], true);
}

#[tokio::test]
async fn asset_non_member_cannot_register_manifest() {
    let state = AppState::for_tests().await;
    let app = build_router(state, AppConfig::test());

    let (app, owner_token) = register_user(app, "owner@example.com").await;
    let (app, other_token) = register_user(app, "other@example.com").await;
    let project = create_project(app.clone(), &owner_token).await;
    let response = app
        .oneshot(bearer_json_request(
            Method::POST,
            &format!("/projects/{}/assets", project.id.0),
            &other_token,
            serde_json::to_value(sample_asset(project.id)).unwrap(),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn sync_project_member_can_push_and_pull_records() {
    let state = AppState::for_tests().await;
    let app = build_router(state, AppConfig::test());

    let (app, token) = register_user(app, "owner@example.com").await;
    let project = create_project(app.clone(), &token).await;
    let change = LocalChange {
        project_id: project.id,
        entity_type: "project".to_string(),
        entity_id: project.id.to_string(),
        operation: SyncOperation::Update,
        payload: json!({ "title": "Updated" }),
    };

    let push_response = app
        .clone()
        .oneshot(bearer_json_request(
            Method::POST,
            &format!("/projects/{}/sync/push", project.id.0),
            &token,
            json!({ "changes": [change] }),
        ))
        .await
        .unwrap();
    assert_eq!(push_response.status(), StatusCode::OK);

    let pull_response = app
        .oneshot(bearer_request(
            Method::GET,
            &format!("/projects/{}/sync/pull?after=0", project.id.0),
            &token,
        ))
        .await
        .unwrap();
    assert_eq!(pull_response.status(), StatusCode::OK);
    let body: serde_json::Value = json_response(pull_response).await;
    assert_eq!(body["records"].as_array().unwrap().len(), 1);
    assert_eq!(body["next_cursor"], 1);
}

fn json_request(method: Method, uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn bearer_request(method: Method, uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

fn bearer_json_request(
    method: Method,
    uri: &str,
    token: &str,
    body: serde_json::Value,
) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn json_response<T: serde::de::DeserializeOwned>(response: axum::response::Response) -> T {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    serde_json::from_slice(&bytes).expect("parse json")
}

async fn register_owner(app: axum::Router) -> axum::Router {
    let response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/auth/register",
            json!({
                "email": "owner@example.com",
                "display_name": "Owner",
                "password": "correct horse battery staple"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    app
}

async fn register_user(app: axum::Router, email: &str) -> (axum::Router, String) {
    let response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/auth/register",
            json!({
                "email": email,
                "display_name": email,
                "password": "correct horse battery staple"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let login: LoginResponse = json_response(response).await;
    (app, login.token)
}

async fn login_user(app: axum::Router, email: &str) -> String {
    let response = app
        .oneshot(json_request(
            Method::POST,
            "/auth/login",
            json!({
                "email": email,
                "password": "correct horse battery staple"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let login: LoginResponse = json_response(response).await;
    login.token
}

async fn promote_to_admin(state: &AppState, email: &str) {
    state
        .auth
        .lock()
        .expect("auth store lock")
        .users_by_email
        .get_mut(email)
        .expect("registered admin")
        .is_admin = true;
    sqlx::query("UPDATE users SET is_admin = 1 WHERE email = ?1")
        .bind(email)
        .execute(&state.db)
        .await
        .expect("persist admin");
}

fn user_id(state: &AppState, email: &str) -> radsuite_core::UserId {
    state
        .auth
        .lock()
        .expect("auth store lock")
        .users_by_email
        .get(email)
        .expect("registered user")
        .id
}

async fn create_project(app: axum::Router, token: &str) -> ApiProjectSummary {
    let response = app
        .oneshot(bearer_json_request(
            Method::POST,
            "/projects",
            token,
            json!({
                "code": "CRJU150",
                "title": "Legal Method"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    json_response(response).await
}

fn sample_asset(project_id: radsuite_core::ProjectId) -> AssetManifest {
    AssetManifest {
        project_id,
        sha256: "b".repeat(64),
        byte_size: 2048,
        mime_type: "application/pdf".to_string(),
        original_name: "reading.pdf".to_string(),
        sync_policy: AssetSyncPolicy::CollaborativeSource,
    }
}
