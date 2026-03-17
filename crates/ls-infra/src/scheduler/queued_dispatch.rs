//! Generic queued-job dispatcher.
//!
//! Picks ready jobs from the shared queue and dispatches them to `process_job()`.

use chrono::Utc;
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

pub const DEFAULT_DISPATCH_LIMIT: i64 = 64;

const DISPATCHABLE_JOBS_SQL: &str = r#"
SELECT id
FROM jobs
WHERE status IN ('queued', 'pending')
  AND dead_letter = false
  AND (next_retry_at IS NULL OR next_retry_at <= $1)
  AND (scheduled_for IS NULL OR scheduled_for <= $1)
ORDER BY COALESCE(scheduled_for, created_at) ASC, created_at ASC
LIMIT $2
"#;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QueuedDispatchResult {
    pub jobs_found: u64,
    pub jobs_dispatched: u64,
    pub dispatch_failures: u64,
}

pub async fn get_dispatchable_jobs(pool: &PgPool, limit: i64) -> anyhow::Result<Vec<Uuid>> {
    let now = Utc::now();
    let rows = sqlx::query_scalar::<_, Uuid>(DISPATCHABLE_JOBS_SQL)
        .bind(now)
        .bind(limit.clamp(1, 1000))
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn dispatch_queued_jobs<F, Fut>(
    pool: &PgPool,
    limit: i64,
    process_fn: F,
) -> anyhow::Result<QueuedDispatchResult>
where
    F: Fn(Uuid) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<()>>,
{
    let job_ids = get_dispatchable_jobs(pool, limit).await?;
    let jobs_found = job_ids.len() as u64;

    if jobs_found == 0 {
        return Ok(QueuedDispatchResult::default());
    }

    let mut jobs_dispatched = 0_u64;
    let mut dispatch_failures = 0_u64;
    for job_id in job_ids {
        match process_fn(job_id).await {
            Ok(()) => jobs_dispatched += 1,
            Err(err) => {
                dispatch_failures += 1;
                warn!(error = %err, %job_id, "queued job dispatch failed");
            }
        }
    }

    info!(
        jobs_found,
        jobs_dispatched, dispatch_failures, "queued job dispatch cycle completed"
    );
    Ok(QueuedDispatchResult {
        jobs_found,
        jobs_dispatched,
        dispatch_failures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatchable_jobs_sql_contains_ready_filters() {
        assert!(DISPATCHABLE_JOBS_SQL.contains("status IN ('queued', 'pending')"));
        assert!(DISPATCHABLE_JOBS_SQL.contains("dead_letter = false"));
        assert!(DISPATCHABLE_JOBS_SQL.contains("next_retry_at <= $1"));
        assert!(DISPATCHABLE_JOBS_SQL.contains("scheduled_for <= $1"));
    }

    #[test]
    fn queued_dispatch_result_defaults_to_zero() {
        assert_eq!(QueuedDispatchResult::default().jobs_found, 0);
        assert_eq!(QueuedDispatchResult::default().jobs_dispatched, 0);
        assert_eq!(QueuedDispatchResult::default().dispatch_failures, 0);
    }
}
