CREATE TABLE IF NOT EXISTS agent_requests (
    id UUID PRIMARY KEY,
    request_type TEXT NOT NULL,
    source TEXT NOT NULL,
    user_id UUID NULL REFERENCES users(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    media_type TEXT NOT NULL DEFAULT 'unknown',
    tmdb_id BIGINT NULL,
    media_item_id UUID NULL REFERENCES media_items(id) ON DELETE SET NULL,
    series_id UUID NULL REFERENCES media_items(id) ON DELETE SET NULL,
    season_numbers JSONB NOT NULL DEFAULT '[]'::jsonb,
    episode_numbers JSONB NOT NULL DEFAULT '[]'::jsonb,
    status_user TEXT NOT NULL,
    status_admin TEXT NOT NULL,
    agent_stage TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    auto_handled BOOLEAN NOT NULL DEFAULT false,
    admin_note TEXT NOT NULL DEFAULT '',
    agent_note TEXT NOT NULL DEFAULT '',
    moviepilot_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    moviepilot_result JSONB NOT NULL DEFAULT '{}'::jsonb,
    last_error TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    closed_at TIMESTAMPTZ NULL
);

CREATE TABLE IF NOT EXISTS agent_request_events (
    id UUID PRIMARY KEY,
    request_id UUID NOT NULL REFERENCES agent_requests(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    actor_user_id UUID NULL REFERENCES users(id) ON DELETE SET NULL,
    actor_username TEXT NULL,
    summary TEXT NOT NULL,
    detail JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_agent_requests_user_created_at
    ON agent_requests(user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_agent_requests_admin_status_created_at
    ON agent_requests(status_admin, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_agent_requests_type_tmdb_open
    ON agent_requests(request_type, tmdb_id, status_admin, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_agent_requests_series_open
    ON agent_requests(series_id, status_admin, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_agent_request_events_request_created_at
    ON agent_request_events(request_id, created_at ASC);
