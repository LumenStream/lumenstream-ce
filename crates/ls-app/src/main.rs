use std::sync::Arc;

use actix_web::{App, HttpServer};
use anyhow::Context;
use ls_api::{ApiContext, build_api_router};
use ls_config::AppConfig;
use ls_infra::{AppInfra, scheduler};
use ls_logging::init_logging;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::load_default().context("failed to load config")?;

    // Initialize logging early, before any tracing calls
    let log_handle = Arc::new(init_logging(&config.log));

    let bind_addr = config.bind_addr();

    let infra = AppInfra::init(config.clone())
        .await
        .context("failed to initialize infrastructure")?;
    let state = ApiContext::new(Arc::new(infra)).with_log_handle(log_handle);

    // Spawn background scheduler
    let _scheduler_handle = scheduler::spawn_scheduler(state.infra.clone());

    tracing::info!(address = %bind_addr, "LumenStream Media Server started");
    HttpServer::new(move || App::new().service(build_api_router(state.clone())))
        .bind(&bind_addr)
        .with_context(|| format!("failed to bind address: {bind_addr}"))?
        .run()
        .await
        .context("server error")?;

    // Scheduler will be shut down when _scheduler_handle is dropped
    Ok(())
}
