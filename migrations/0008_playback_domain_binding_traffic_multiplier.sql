-- Bind playback domains to LumenBackend nodes and support per-line traffic multiplier.
-- Also persist real bytes separately from billed bytes for traffic analytics.

ALTER TABLE playback_domains
    ADD COLUMN IF NOT EXISTS lumenbackend_node_id TEXT,
    ADD COLUMN IF NOT EXISTS traffic_multiplier DOUBLE PRECISION NOT NULL DEFAULT 1.0;

UPDATE playback_domains
SET traffic_multiplier = 1.0
WHERE traffic_multiplier <= 0
   OR traffic_multiplier = 'NaN'::DOUBLE PRECISION
   OR traffic_multiplier = 'Infinity'::DOUBLE PRECISION
   OR traffic_multiplier = '-Infinity'::DOUBLE PRECISION;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'fk_playback_domains_lumenbackend_node_id'
    ) THEN
        ALTER TABLE playback_domains
            ADD CONSTRAINT fk_playback_domains_lumenbackend_node_id
            FOREIGN KEY (lumenbackend_node_id)
            REFERENCES lumenbackend_nodes(node_id)
            ON DELETE SET NULL;
    END IF;
END
$$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_playback_domains_traffic_multiplier_positive'
    ) THEN
        ALTER TABLE playback_domains
            ADD CONSTRAINT chk_playback_domains_traffic_multiplier_positive
            CHECK (traffic_multiplier > 0);
    END IF;
END
$$;

CREATE INDEX IF NOT EXISTS idx_playback_domains_lumenbackend_node
    ON playback_domains(lumenbackend_node_id)
    WHERE lumenbackend_node_id IS NOT NULL;

ALTER TABLE user_stream_usage_daily
    ADD COLUMN IF NOT EXISTS real_bytes_served BIGINT NOT NULL DEFAULT 0;

UPDATE user_stream_usage_daily
SET real_bytes_served = bytes_served
WHERE real_bytes_served = 0
  AND bytes_served > 0;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_user_stream_usage_daily_real_bytes_served'
    ) THEN
        ALTER TABLE user_stream_usage_daily
            ADD CONSTRAINT chk_user_stream_usage_daily_real_bytes_served
            CHECK (real_bytes_served >= 0);
    END IF;
END
$$;

ALTER TABLE user_stream_usage_media_daily
    ADD COLUMN IF NOT EXISTS real_bytes_served BIGINT NOT NULL DEFAULT 0;

UPDATE user_stream_usage_media_daily
SET real_bytes_served = bytes_served
WHERE real_bytes_served = 0
  AND bytes_served > 0;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_user_stream_usage_media_daily_real_bytes_served'
    ) THEN
        ALTER TABLE user_stream_usage_media_daily
            ADD CONSTRAINT chk_user_stream_usage_media_daily_real_bytes_served
            CHECK (real_bytes_served >= 0);
    END IF;
END
$$;
