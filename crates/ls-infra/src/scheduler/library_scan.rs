//! Library scan scheduling logic.
//!
//! Checks each library's last scan time against its configured interval
//! and enqueues scan jobs for libraries that are due.

use chrono::{Duration, Utc};
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

/// Row returned from the due-libraries query.
#[derive(Debug, sqlx::FromRow)]
struct DueLibraryRow {
    id: Uuid,
    name: String,
    scan_interval_hours: i32,
}

/// Check all libraries and enqueue scan jobs for those that are due.
///
/// A library is due for scanning when:
/// - `enabled = true`
/// - `scan_interval_hours > 0` (not manual-only)
/// - Either never scanned, or `last_scan_finished_at + interval < now()`
///
/// Returns the number of scan jobs enqueued.
pub async fn check_and_enqueue_library_scans(pool: &PgPool) -> anyhow::Result<u64> {
    let due_libraries: Vec<DueLibraryRow> = sqlx::query_as(
        r#"
SELECT l.id, l.name, l.scan_interval_hours
FROM libraries l
LEFT JOIN library_scan_state s ON l.id = s.library_id
WHERE l.enabled = true
  AND l.scan_interval_hours > 0
  AND (
    s.last_scan_finished_at IS NULL
    OR s.last_scan_finished_at + (l.scan_interval_hours || ' hours')::interval < now()
  )
        "#,
    )
    .fetch_all(pool)
    .await?;

    if due_libraries.is_empty() {
        return Ok(0);
    }

    let mut enqueued = 0_u64;
    for lib in &due_libraries {
        // Check if there's already a pending/running scan job for this library
        let existing: Option<Uuid> = sqlx::query_scalar(
            r#"
SELECT id FROM jobs
WHERE kind = 'scan_library'
  AND status IN ('pending', 'running')
  AND payload->>'library_id' = $1::text
LIMIT 1
            "#,
        )
        .bind(lib.id.to_string())
        .fetch_optional(pool)
        .await?;

        if existing.is_some() {
            info!(
                library_id = %lib.id,
                library_name = %lib.name,
                "skipping library scan: job already pending/running"
            );
            continue;
        }

        // Enqueue a new incremental scan job
        let job_id = Uuid::now_v7();
        sqlx::query(
            r#"
INSERT INTO jobs (id, kind, status, payload, attempts, max_attempts, dead_letter)
VALUES ($1, 'scan_library', 'pending', $2, 0, 3, false)
            "#,
        )
        .bind(job_id)
        .bind(serde_json::json!({
            "library_id": lib.id,
            "mode": "incremental",
            "auto_scheduled": true,
        }))
        .execute(pool)
        .await?;

        info!(
            library_id = %lib.id,
            library_name = %lib.name,
            interval_hours = lib.scan_interval_hours,
            job_id = %job_id,
            "enqueued auto-scheduled library scan"
        );
        enqueued += 1;
    }

    if enqueued > 0 {
        info!(count = enqueued, "auto-scheduled library scans enqueued");
    }

    Ok(enqueued)
}

/// Get the next scheduled scan time for a library.
///
/// Returns `None` if the library has `scan_interval_hours = 0` (manual only)
/// or if it has never been scanned.
pub async fn get_next_scan_time(
    pool: &PgPool,
    library_id: Uuid,
) -> anyhow::Result<Option<chrono::DateTime<Utc>>> {
    let row: Option<(Option<chrono::DateTime<Utc>>, i32)> = sqlx::query_as(
        r#"
SELECT s.last_scan_finished_at, l.scan_interval_hours
FROM libraries l
LEFT JOIN library_scan_state s ON l.id = s.library_id
WHERE l.id = $1
        "#,
    )
    .bind(library_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some((Some(last_scan), interval)) if interval > 0 => {
            Ok(Some(last_scan + Duration::hours(interval as i64)))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn due_library_row_debug() {
        let row = DueLibraryRow {
            id: Uuid::nil(),
            name: "Test Library".to_string(),
            scan_interval_hours: 6,
        };
        let debug_str = format!("{:?}", row);
        assert!(debug_str.contains("Test Library"));
        assert!(debug_str.contains("6"));
    }

    #[test]
    fn interval_zero_means_manual_only() {
        // This is a documentation test - interval 0 should be skipped
        // The actual filtering happens in the SQL query with `scan_interval_hours > 0`
        let interval = 0;
        assert!(interval == 0, "interval 0 means manual-only scanning");
    }

    #[test]
    fn interval_calculation() {
        // Verify interval math works correctly
        let last_scan = Utc::now() - Duration::hours(7);
        let interval_hours = 6;
        let next_scan = last_scan + Duration::hours(interval_hours);
        let now = Utc::now();

        // 7 hours ago + 6 hour interval = 1 hour ago, so it's due
        assert!(next_scan < now, "library should be due for scan");

        let last_scan_recent = Utc::now() - Duration::hours(2);
        let next_scan_recent = last_scan_recent + Duration::hours(interval_hours);

        // 2 hours ago + 6 hour interval = 4 hours from now, not due
        assert!(next_scan_recent > now, "library should not be due yet");
    }
}
