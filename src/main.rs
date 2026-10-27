mod config;

use std::sync::Arc;

use axum::{Router, http::StatusCode, routing::{get, post}};
use config::{Config, Secrets};

#[allow(dead_code)] // used by the webhook handler (plan step 5)
struct State {
    cfg: Config,
    secrets: Secrets,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "/config/config.yaml".into());
    let state = Arc::new(State { cfg: Config::load(&path)?, secrets: Secrets::from_env()? });

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/hook", post(|| async { StatusCode::NOT_IMPLEMENTED }))
        .with_state(state);

    let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
    tracing::info!(%addr, "listening");
    axum::serve(tokio::net::TcpListener::bind(&addr).await?, app).await?;
    Ok(())
}
