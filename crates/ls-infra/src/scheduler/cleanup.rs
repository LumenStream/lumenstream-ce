//! Database cleanup functions for expired/stale data.

use chrono::{Duration, Utc};
use sqlx::PgPool;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::info;
use uuid::Uuid;

/// Metrics for cleanup operations.
#[derive(Default)]
pub struct CleanupMetrics {
    pub expired_tokens_removed: AtomicU64,
    pub stale_sessions_marked: AtomicU64,
    pub tmdb_cache_removed: AtomicU64,
    pub old_jobs_removed: AtomicU64,
    pub old_audit_logs_removed: AtomicU64,
    pub old_system_events_removed: AtomicU64,
    pub old_user_stream_media_usage_removed: AtomicU64,
    pub orphaned_media_items_removed: AtomicU64,
}

impl CleanupMetrics {
    pub fn snapshot(&self) -> CleanupMetricsSnapshot {
        CleanupMetricsSnapshot {
            expired_tokens_removed: self.expired_tokens_removed.load(Ordering::Relaxed),
            stale_sessions_marked: self.stale_sessions_marked.load(Ordering::Relaxed),
            tmdb_cache_removed: self.tmdb_cache_removed.load(Ordering::Relaxed),
            old_jobs_removed: self.old_jobs_removed.load(Ordering::Relaxed),
            old_audit_logs_removed: self.old_audit_logs_removed.load(Ordering::Relaxed),
            old_system_events_removed: self.old_system_events_removed.load(Ordering::Relaxed),
            old_user_stream_media_usage_removed: self
                .old_user_stream_media_usage_removed
                .load(Ordering::Relaxed),
            orphaned_media_items_removed: self.orphaned_media_items_removed.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleanupMetricsSnapshot {
    pub expired_tokens_removed: u64,
    pub stale_sessions_marked: u64,
    pub tmdb_cache_removed: u64,
    pub old_jobs_removed: u64,
    pub old_audit_logs_removed: u64,
    pub old_system_events_removed: u64,
    pub old_user_stream_media_usage_removed: u64,
    pub orphaned_media_items_removed: u64,
}

/// Delete expired access tokens.
///
/// Returns the number of tokens deleted.
pub async fn cleanup_expired_tokens(
    pool: &PgPool,
    metrics: &CleanupMetrics,
) -> anyhow::Result<u64> {
    let now = Utc::now();
    let result = sqlx::query("DELETE FROM access_tokens WHERE expires_at < $1")
        .bind(now)
        .execute(pool)
        .await?;

    let count = result.rows_affected();
    if count > 0 {
        metrics
            .expired_tokens_removed
            .fetch_add(count, Ordering::Relaxed);
        info!(count, "cleaned up expired access tokens");
    }
    Ok(count)
}

/// Mark inactive playback sessions as not active.
///
/// Sessions without a heartbeat for longer than `stale_threshold_seconds` are marked inactive.
/// Returns the number of sessions marked inactive.
pub async fn cleanup_stale_sessions(
    pool: &PgPool,
    metrics: &CleanupMetrics,
    stale_threshold_seconds: i64,
) -> anyhow::Result<u64> {
    let threshold = Utc::now() - Duration::seconds(stale_threshold_seconds);
    let result = sqlx::query(
        "UPDATE playback_sessions SET is_active = false WHERE is_active = true AND last_heartbeat_at < $1",
    )
    .bind(threshold)
    .execute(pool)
    .await?;

    let count = result.rows_affected();
    if count > 0 {
        metrics
            .stale_sessions_marked
            .fetch_add(count, Ordering::Relaxed);
        info!(
            count,
            threshold_seconds = stale_threshold_seconds,
            "marked stale playback sessions inactive"
        );
    }
    Ok(count)
}

/// Delete expired TMDB cache entries.
///
/// Returns the number of cache entries deleted.
pub async fn cleanup_tmdb_cache(pool: &PgPool, metrics: &CleanupMetrics) -> anyhow::Result<u64> {
    let now = Utc::now();
    let result = sqlx::query("DELETE FROM tmdb_cache WHERE expires_at < $1")
        .bind(now)
        .execute(pool)
        .await?;

    let count = result.rows_affected();
    if count > 0 {
        metrics
            .tmdb_cache_removed
            .fetch_add(count, Ordering::Relaxed);
        info!(count, "cleaned up expired TMDB cache entries");
    }
    Ok(count)
}

/// Delete old completed or dead-letter jobs.
///
/// Jobs with status 'completed' or 'failed' (dead_letter = true) older than `retention_days` are deleted.
/// Returns the number of jobs deleted.
pub async fn cleanup_old_jobs(
    pool: &PgPool,
    metrics: &CleanupMetrics,
    retention_days: i64,
) -> anyhow::Result<u64> {
    let threshold = Utc::now() - Duration::days(retention_days);
    let result = sqlx::query(
        "DELETE FROM jobs WHERE (status = 'completed' OR dead_letter = true) AND finished_at < $1",
    )
    .bind(threshold)
    .execute(pool)
    .await?;

    let count = result.rows_affected();
    if count > 0 {
        metrics.old_jobs_removed.fetch_add(count, Ordering::Relaxed);
        info!(count, retention_days, "cleaned up old jobs");
    }
    Ok(count)
}

/// Delete old audit logs.
///
/// Audit logs older than `retention_days` are deleted.
/// Returns the number of logs deleted.
pub async fn cleanup_old_audit_logs(
    pool: &PgPool,
    metrics: &CleanupMetrics,
    retention_days: i64,
) -> anyhow::Result<u64> {
    let threshold = Utc::now() - Duration::days(retention_days);
    let result = sqlx::query("DELETE FROM audit_logs WHERE created_at < $1")
        .bind(threshold)
        .execute(pool)
        .await?;

    let count = result.rows_affected();
    if count > 0 {
        metrics
            .old_audit_logs_removed
            .fetch_add(count, Ordering::Relaxed);
        info!(count, retention_days, "cleaned up old audit logs");
    }
    Ok(count)
}

/// Delete old system events.
///
/// System events older than `retention_days` are deleted.
/// Returns the number of events deleted.
pub async fn cleanup_old_system_events(
    pool: &PgPool,
    metrics: &CleanupMetrics,
    retention_days: i64,
) -> anyhow::Result<u64> {
    let threshold = Utc::now() - Duration::days(retention_days);
    let result = sqlx::query("DELETE FROM system_events WHERE created_at < $1")
        .bind(threshold)
        .execute(pool)
        .await?;

    let count = result.rows_affected();
    if count > 0 {
        metrics
            .old_system_events_removed
            .fetch_add(count, Ordering::Relaxed);
        info!(count, retention_days, "cleaned up old system events");
    }
    Ok(count)
}

/// Delete old per-media traffic usage rows.
///
/// Keeps a rolling `retention_days` window (inclusive of today).
/// Returns the number of deleted rows.
pub async fn cleanup_old_user_stream_media_usage(
    pool: &PgPool,
    metrics: &CleanupMetrics,
    retention_days: i64,
) -> anyhow::Result<u64> {
    let keep_days = retention_days.max(1).min(i64::from(i32::MAX));
    let result = sqlx::query(
        "DELETE FROM user_stream_usage_media_daily WHERE usage_date < current_date - ($1::INT - 1)",
    )
    .bind(keep_days as i32)
    .execute(pool)
    .await?;

    let count = result.rows_affected();
    if count > 0 {
        metrics
            .old_user_stream_media_usage_removed
            .fetch_add(count, Ordering::Relaxed);
        info!(
            count,
            keep_days, "cleaned up old per-media traffic usage rows"
        );
    }
    Ok(count)
}

/// Delete orphaned media items whose source files no longer exist on disk.
///
/// Removes Movie/Episode items with missing `.strm` files, then cleans up
/// Season/Series entries that have no remaining children.
/// Returns total number of items removed.
pub async fn cleanup_orphaned_media_items(
    pool: &PgPool,
    metrics: &CleanupMetrics,
) -> anyhow::Result<u64> {
    // Step 1: Find Movie/Episode items whose path no longer exists
    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, path FROM media_items WHERE item_type IN ('Movie', 'Episode') AND path IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;

    let orphan_ids: Vec<Uuid> = rows
        .into_iter()
        .filter(|(_, path)| !Path::new(path).exists())
        .map(|(id, _)| id)
        .collect();

    let mut total_removed = 0_u64;

    // Step 2: Batch delete orphaned Movie/Episode items
    if !orphan_ids.is_empty() {
        let result = sqlx::query("DELETE FROM media_items WHERE id = ANY($1)")
            .bind(&orphan_ids)
            .execute(pool)
            .await?;
        total_removed += result.rows_affected();
    }

    // Step 3: Delete orphan Seasons (no child episodes in same series/season bucket)
    let result = sqlx::query(
        r#"
DELETE FROM media_items s
WHERE s.item_type = 'Season'
  AND NOT EXISTS (
      SELECT 1
      FROM media_items e
      WHERE e.item_type = 'Episode'
        AND e.series_id = s.series_id
        AND COALESCE(e.season_number, 0) = COALESCE(s.season_number, 0)
  )
        "#,
    )
    .execute(pool)
    .await?;
    total_removed += result.rows_affected();

    // Step 4: Delete orphan Series (no child seasons/episodes)
    let result = sqlx::query(
        "DELETE FROM media_items WHERE item_type = 'Series' AND NOT EXISTS (SELECT 1 FROM media_items e WHERE e.series_id = media_items.id)",
    )
    .execute(pool)
    .await?;
    total_removed += result.rows_affected();

    if total_removed > 0 {
        metrics
            .orphaned_media_items_removed
            .fetch_add(total_removed, Ordering::Relaxed);
        info!(total_removed, "cleaned up orphaned media items");
    }
    Ok(total_removed)
}

/// Combined cleanup for logs (audit_logs + system_events).
///
/// Returns total count of deleted records.
pub async fn cleanup_old_logs(
    pool: &PgPool,
    metrics: &CleanupMetrics,
    retention_days: i64,
) -> anyhow::Result<u64> {
    let audit_count = cleanup_old_audit_logs(pool, metrics, retention_days).await?;
    let events_count = cleanup_old_system_events(pool, metrics, retention_days).await?;
    Ok(audit_count + events_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_metrics_snapshot() {
        let metrics = CleanupMetrics::default();
        metrics.expired_tokens_removed.store(10, Ordering::Relaxed);
        metrics.stale_sessions_marked.store(5, Ordering::Relaxed);
        metrics.tmdb_cache_removed.store(20, Ordering::Relaxed);
        metrics.old_jobs_removed.store(15, Ordering::Relaxed);
        metrics.old_audit_logs_removed.store(100, Ordering::Relaxed);
        metrics
            .old_system_events_removed
            .store(50, Ordering::Relaxed);
        metrics
            .old_user_stream_media_usage_removed
            .store(12, Ordering::Relaxed);
        metrics
            .orphaned_media_items_removed
            .store(7, Ordering::Relaxed);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.expired_tokens_removed, 10);
        assert_eq!(snapshot.stale_sessions_marked, 5);
        assert_eq!(snapshot.tmdb_cache_removed, 20);
        assert_eq!(snapshot.old_jobs_removed, 15);
        assert_eq!(snapshot.old_audit_logs_removed, 100);
        assert_eq!(snapshot.old_system_events_removed, 50);
        assert_eq!(snapshot.old_user_stream_media_usage_removed, 12);
        assert_eq!(snapshot.orphaned_media_items_removed, 7);
    }

    #[test]
    fn cleanup_metrics_default() {
        let metrics = CleanupMetrics::default();
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.expired_tokens_removed, 0);
        assert_eq!(snapshot.stale_sessions_marked, 0);
        assert_eq!(snapshot.tmdb_cache_removed, 0);
        assert_eq!(snapshot.old_jobs_removed, 0);
        assert_eq!(snapshot.old_audit_logs_removed, 0);
        assert_eq!(snapshot.old_system_events_removed, 0);
        assert_eq!(snapshot.old_user_stream_media_usage_removed, 0);
        assert_eq!(snapshot.orphaned_media_items_removed, 0);
    }

    #[test]
    fn cleanup_metrics_atomic_increment() {
        let metrics = CleanupMetrics::default();

        metrics
            .expired_tokens_removed
            .fetch_add(5, Ordering::Relaxed);
        metrics
            .expired_tokens_removed
            .fetch_add(3, Ordering::Relaxed);

        assert_eq!(metrics.expired_tokens_removed.load(Ordering::Relaxed), 8);
    }
}
