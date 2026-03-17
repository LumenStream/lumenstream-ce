-- Per-user media traffic aggregation (daily).
-- Keeps recent media-level usage records for user-center traffic breakdown.

CREATE TABLE IF NOT EXISTS user_stream_usage_media_daily (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    media_item_id UUID NOT NULL,
    usage_date DATE NOT NULL,
    bytes_served BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, media_item_id, usage_date),
    CONSTRAINT chk_user_stream_usage_media_daily_bytes_served CHECK (bytes_served >= 0)
);

CREATE INDEX IF NOT EXISTS idx_user_stream_usage_media_daily_user_date
    ON user_stream_usage_media_daily(user_id, usage_date DESC);

CREATE INDEX IF NOT EXISTS idx_user_stream_usage_media_daily_usage_date
    ON user_stream_usage_media_daily(usage_date DESC);

CREATE INDEX IF NOT EXISTS idx_user_stream_usage_media_daily_media_item
    ON user_stream_usage_media_daily(media_item_id);
