ALTER TABLE library_scan_state
    ADD COLUMN IF NOT EXISTS last_subtitle_sync_started_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS last_subtitle_sync_finished_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS last_subtitle_sync_mode TEXT NOT NULL DEFAULT 'incremental';

UPDATE task_definitions
SET default_payload = default_payload || '{"mode":"incremental"}'::jsonb,
    updated_at = now()
WHERE task_key = 'subtitle_sync'
  AND COALESCE(default_payload->>'mode', '') = '';
