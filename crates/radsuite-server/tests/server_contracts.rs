use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use radsuite_core::LoginResponse;
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

fn json_request(method: Method, uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
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
