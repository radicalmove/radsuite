pub mod config;
pub mod routes;
pub mod state;

use axum::Router;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub use config::AppConfig;
pub use state::AppState;

pub fn build_router(state: AppState, _config: AppConfig) -> Router {
    Router::new()
        .merge(routes::health::router())
        .merge(routes::auth::router())
        .merge(routes::projects::router())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
