ALTER TABLE libraries
    ADD COLUMN IF NOT EXISTS scraper_policy JSONB NOT NULL DEFAULT '{}'::jsonb;

INSERT INTO task_definitions (task_key, display_name, enabled, cron_expr, default_payload, max_attempts)
SELECT 'scraper_fill', '刮削补齐', enabled, cron_expr, default_payload, max_attempts
FROM task_definitions
WHERE task_key = 'tmdb_fill'
ON CONFLICT (task_key) DO NOTHING;

DELETE FROM task_definitions WHERE task_key = 'tmdb_fill';

UPDATE jobs
SET kind = 'scraper_fill'
WHERE kind = 'tmdb_fill';
