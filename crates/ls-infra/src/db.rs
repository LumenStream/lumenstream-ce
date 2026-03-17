use anyhow::Context;
use sqlx::{PgPool, postgres::PgPoolOptions};

use ls_config::AppConfig;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

pub async fn connect(cfg: &AppConfig) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(cfg.database.max_connections)
        .connect(&cfg.database.url)
        .await
        .with_context(|| "failed to connect to postgres")?;
    Ok(pool)
}

pub async fn migrate(pool: &PgPool) -> anyhow::Result<()> {
    MIGRATOR
        .run(pool)
        .await
        .with_context(|| "failed to run migrations")?;
    Ok(())
}
