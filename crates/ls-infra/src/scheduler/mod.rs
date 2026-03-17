//! Background scheduler for periodic task-center jobs.

pub mod billing;
pub mod cleanup;
pub mod job_retry;
pub mod library_scan;
pub mod queued_dispatch;

use std::{str::FromStr, sync::Arc};

use chrono::{DateTime, Duration, Utc};
use cron::Schedule;
use tokio::sync::watch;
use tracing::{info, warn};

use crate::AppInfra;

/// Handle to control the scheduler lifecycle.
pub struct SchedulerHandle {
    shutdown_tx: watch::Sender<bool>,
}

impl SchedulerHandle {
    /// Signal the scheduler to shut down gracefully.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }
}

/// Spawn the background task-center cron scheduler.
pub fn spawn_scheduler(infra: Arc<AppInfra>) -> SchedulerHandle {
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    if !infra.config_snapshot().scheduler.enabled {
        info!("scheduler disabled by configuration");
        return SchedulerHandle { shutdown_tx };
    }

    tokio::spawn(async move {
        run_scheduler_loop(infra, shutdown_rx).await;
    });

    info!("task-center scheduler started");
    SchedulerHandle { shutdown_tx }
}

async fn run_scheduler_loop(infra: Arc<AppInfra>, mut shutdown_rx: watch::Receiver<bool>) {
    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(30));

    // Skip first immediate tick to avoid startup stampede.
    ticker.tick().await;

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("scheduler received shutdown signal");
                    break;
                }
            }
            _ = ticker.tick() => {
                if let Err(err) = dispatch_due_tasks(infra.clone()).await {
                    warn!(error = %err, "failed to dispatch due scheduled tasks");
                }
                if let Err(err) = dispatch_ready_queued_jobs(infra.clone()).await {
                    warn!(error = %err, "failed to dispatch queued ready tasks");
                }
            }
        }
    }

    info!("task-center scheduler stopped");
}

async fn dispatch_due_tasks(infra: Arc<AppInfra>) -> anyhow::Result<()> {
    let tasks = infra.list_task_definitions().await?;
    if tasks.is_empty() {
        return Ok(());
    }

    let now = Utc::now();
    let window_start = now - Duration::seconds(59);

    for task in tasks {
        if !task.enabled {
            continue;
        }

        let schedule = match Schedule::from_str(task.cron_expr.trim()) {
            Ok(value) => value,
            Err(err) => {
                warn!(task_key = %task.task_key, cron_expr = %task.cron_expr, error = %err, "invalid cron expression for task");
                continue;
            }
        };

        let Some(scheduled_for) = find_due_time(&schedule, window_start, now) else {
            continue;
        };

        let Some(job) = infra
            .enqueue_scheduled_task_run(&task, scheduled_for)
            .await?
        else {
            continue;
        };

        let infra_for_run = infra.clone();
        let run_id = job.id;
        tokio::spawn(async move {
            if let Err(err) = infra_for_run.process_job(run_id).await {
                warn!(error = %err, run_id = %run_id, "scheduled task run crashed");
            }
        });

        info!(
            task_key = %task.task_key,
            run_id = %run_id,
            scheduled_for = %scheduled_for,
            "scheduled task run enqueued"
        );
    }

    Ok(())
}

async fn dispatch_ready_queued_jobs(infra: Arc<AppInfra>) -> anyhow::Result<()> {
    let pool = infra.pool.clone();
    queued_dispatch::dispatch_queued_jobs(
        &pool,
        queued_dispatch::DEFAULT_DISPATCH_LIMIT,
        move |job_id| {
            let infra = infra.clone();
            async move { infra.process_job(job_id).await }
        },
    )
    .await?;
    Ok(())
}

fn find_due_time(
    schedule: &Schedule,
    window_start: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    schedule
        .after(&window_start)
        .next()
        .map(|value| value.with_timezone(&Utc))
        .filter(|scheduled| *scheduled <= now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn due_time_found_within_window() {
        let schedule = Schedule::from_str("0 * * * * *").expect("valid cron");
        let now = Utc.with_ymd_and_hms(2026, 2, 16, 12, 34, 30).unwrap();
        let window_start = now - Duration::seconds(59);

        let due = find_due_time(&schedule, window_start, now);
        assert!(due.is_some());
    }

    #[test]
    fn due_time_not_found_outside_window() {
        let schedule = Schedule::from_str("0 0 * * * *").expect("valid cron");
        let now = Utc.with_ymd_and_hms(2026, 2, 16, 12, 34, 30).unwrap();
        let window_start = now - Duration::seconds(1);

        let due = find_due_time(&schedule, window_start, now);
        assert!(due.is_none());
    }
}
