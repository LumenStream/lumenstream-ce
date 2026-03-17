-- Retire deprecated cache_prewarm task via forward-only migration.
-- Do not mutate previously applied baseline migration files.

DELETE FROM task_definitions
WHERE task_key = 'cache_prewarm';

-- Mark non-terminal cache_prewarm jobs as cancelled to avoid retry churn
-- now that runtime execution path has been removed.
UPDATE jobs
SET status = 'cancelled',
    finished_at = COALESCE(finished_at, now()),
    error = NULL,
    next_retry_at = NULL,
    cancel_requested = false,
    progress = jsonb_build_object(
        'phase', 'cancelled',
        'total', 1,
        'completed', 1,
        'percent', 100,
        'message', '任务已下线（cache_prewarm）',
        'detail', jsonb_build_object('reason', 'task_retired'),
        'updated_at', now()
    )
WHERE kind = 'cache_prewarm'
  AND status IN ('queued', 'pending', 'running');
