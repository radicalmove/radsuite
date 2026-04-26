use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use radsuite_core::{ApiProjectSummary, LoginResponse, ProjectRole};
use radsuite_server::{AppConfig, AppState, build_router};
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
