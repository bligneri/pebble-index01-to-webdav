mod caldav;
mod classify;
mod config;
mod ics;
mod webhook;

use std::sync::Arc;

use anyhow::Context;
use axum::{
    Router,
    routing::{get, post},
};
use config::{Config, Secrets};

struct State {
    cfg: Config,
    secrets: Secrets,
    seen: webhook::Seen,
    // None until CALDAV_APP_PASSWORD is set
    dav: Option<caldav::Dav>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "/config/config.yaml".into());
    let (cfg, secrets) = (Config::load(&path)?, Secrets::from_env()?);
    let dav = match &secrets.caldav_password {
        None => None,
        Some(pw) => {
            let user = secrets.caldav_user.as_ref().or(cfg.caldav.user.as_ref());
            let user =
                user.context("CALDAV_APP_PASSWORD is set but neither CALDAV_USER nor caldav.user")?;
            Some(caldav::Dav::new(&cfg, user, pw))
        }
    };
    let state = Arc::new(State {
        cfg,
        secrets,
        seen: Default::default(),
        dav,
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
