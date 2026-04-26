use std::net::SocketAddr;

use radsuite_server::{AppConfig, AppState, build_router};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "radsuite_server=info,tower_http=info".into()),
        )
        .init();

    let config = AppConfig::from_env();
    let state = AppState::from_config(&config).await?;
    let app = build_router(state, config.clone());
    let addr: SocketAddr = config.bind_addr.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!(%addr, "starting RADsuite server");
    axum::serve(listener, app).await?;

    Ok(())
}
