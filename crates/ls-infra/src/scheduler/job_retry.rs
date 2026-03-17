//! Job retry processing for the scheduler.
//!
//! Finds jobs that are eligible for retry (status = 'queued' with next_retry_at <= now)
//! and re-dispatches them to the job processor.

use chrono::Utc;
use sqlx::PgPool;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::info;
use uuid::Uuid;

/// Metrics for job retry operations.
#[derive(Default)]
pub struct JobRetryMetrics {
    pub jobs_retried: AtomicU64,
    pub retry_failures: AtomicU64,
}

impl JobRetryMetrics {
    pub fn snapshot(&self) -> JobRetryMetricsSnapshot {
        JobRetryMetricsSnapshot {
            jobs_retried: self.jobs_retried.load(Ordering::Relaxed),
            retry_failures: self.retry_failures.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobRetryMetricsSnapshot {
    pub jobs_retried: u64,
    pub retry_failures: u64,
}

/// Result of a retry processing run.
#[derive(Debug, Clone, Copy, Default)]
pub struct RetryProcessingResult {
    pub jobs_found: u64,
    pub jobs_dispatched: u64,
}

/// Get IDs of jobs eligible for retry.
///
/// Returns job IDs where `status = 'queued'` and `next_retry_at <= now()`.
/// These are jobs that previously failed and are scheduled for retry.
pub async fn get_retry_eligible_jobs(pool: &PgPool, limit: i64) -> anyhow::Result<Vec<Uuid>> {
    let now = Utc::now();
    let rows = sqlx::query_scalar::<_, Uuid>(
        r#"
SELECT id FROM jobs
WHERE status = 'queued'
  AND next_retry_at IS NOT NULL
  AND next_retry_at <= $1
ORDER BY next_retry_at ASC
LIMIT $2
        "#,
    )
    .bind(now)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Process retry-eligible jobs.
///
/// This function:
/// 1. Finds jobs where `status = 'queued'` and `next_retry_at <= now()`
/// 2. Returns the job IDs for the caller to dispatch
///
/// The actual job processing is done by `AppInfra::process_job()` which handles:
/// - Incrementing attempts
/// - Updating next_retry_at on failure
/// - Moving to dead-letter after max_attempts exceeded
///
/// Returns the number of jobs found and dispatched.
pub async fn process_retry_jobs<F, Fut>(
    pool: &PgPool,
    metrics: &JobRetryMetrics,
    limit: i64,
    process_fn: F,
) -> anyhow::Result<RetryProcessingResult>
where
    F: Fn(Uuid) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<()>>,
{
    let job_ids = get_retry_eligible_jobs(pool, limit).await?;
    let jobs_found = job_ids.len() as u64;

    if jobs_found == 0 {
        return Ok(RetryProcessingResult::default());
    }

    let mut jobs_dispatched = 0u64;

    for job_id in job_ids {
        match process_fn(job_id).await {
            Ok(()) => {
                jobs_dispatched += 1;
                metrics.jobs_retried.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => {
                metrics.retry_failures.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(job_id = %job_id, error = %e, "failed to process retry job");
            }
        }
    }

    if jobs_dispatched > 0 {
        info!(jobs_found, jobs_dispatched, "processed retry-eligible jobs");
    }

    Ok(RetryProcessingResult {
        jobs_found,
        jobs_dispatched,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_retry_metrics_default() {
        let metrics = JobRetryMetrics::default();
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.jobs_retried, 0);
        assert_eq!(snapshot.retry_failures, 0);
    }

    #[test]
    fn job_retry_metrics_snapshot() {
        let metrics = JobRetryMetrics::default();
        metrics.jobs_retried.store(10, Ordering::Relaxed);
        metrics.retry_failures.store(2, Ordering::Relaxed);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.jobs_retried, 10);
        assert_eq!(snapshot.retry_failures, 2);
    }

    #[test]
    fn job_retry_metrics_atomic_increment() {
        let metrics = JobRetryMetrics::default();

        metrics.jobs_retried.fetch_add(5, Ordering::Relaxed);
        metrics.jobs_retried.fetch_add(3, Ordering::Relaxed);
        metrics.retry_failures.fetch_add(1, Ordering::Relaxed);

        assert_eq!(metrics.jobs_retried.load(Ordering::Relaxed), 8);
        assert_eq!(metrics.retry_failures.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn retry_processing_result_default() {
        let result = RetryProcessingResult::default();
        assert_eq!(result.jobs_found, 0);
        assert_eq!(result.jobs_dispatched, 0);
    }

    #[test]
    fn retry_processing_result_can_be_constructed() {
        let result = RetryProcessingResult {
            jobs_found: 5,
            jobs_dispatched: 4,
        };
        assert_eq!(result.jobs_found, 5);
        assert_eq!(result.jobs_dispatched, 4);
    }
}
