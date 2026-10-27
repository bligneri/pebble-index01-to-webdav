mod classify;
mod config;
#[allow(dead_code)] // wired into the webhook handler (plan step 6)
mod ics;
mod webhook;

use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use config::{Config, Secrets};

struct State {
    cfg: Config,
    secrets: Secrets,
    seen: webhook::Seen,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "/config/config.yaml".into());
    let state = Arc::new(State {
        cfg: Config::load(&path)?,
        secrets: Secrets::from_env()?,
        seen: Default::default(),
    });

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/hook", post(webhook::hook))
        .with_state(state);

    let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
    tracing::info!(%addr, "listening");
    axum::serve(tokio::net::TcpListener::bind(&addr).await?, app).await?;
    Ok(())
}
