use axum::{body::Body, http::Request};
use radsuite_server::{AppConfig, AppState, build_router};
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
