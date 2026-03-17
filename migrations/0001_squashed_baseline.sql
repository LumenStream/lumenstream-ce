-- Squashed baseline migration (all-in-one for fresh deploy).

-- ===== users & auth =====
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    is_admin BOOLEAN NOT NULL DEFAULT FALSE,
    is_disabled BOOLEAN NOT NULL DEFAULT FALSE,
    role TEXT NOT NULL DEFAULT 'Viewer',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_users_role CHECK (role IN ('Admin', 'Viewer'))
);

CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);

CREATE TABLE IF NOT EXISTS access_tokens (
    token TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_access_tokens_user_id ON access_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_access_tokens_expires_at ON access_tokens(expires_at);

CREATE TABLE IF NOT EXISTS user_sessions (
    id UUID PRIMARY KEY,
    token TEXT NOT NULL UNIQUE REFERENCES access_tokens(token) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    client TEXT,
    device_name TEXT,
    device_id TEXT,
    remote_addr TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_user_sessions_user_id ON user_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_user_sessions_last_seen_at ON user_sessions(last_seen_at DESC);

CREATE TABLE IF NOT EXISTS admin_api_keys (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    key_hash TEXT NOT NULL UNIQUE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_admin_api_keys_created_at ON admin_api_keys(created_at DESC);

CREATE TABLE IF NOT EXISTS auth_risk_events (
    id UUID PRIMARY KEY,
    remote_addr TEXT,
    username TEXT,
    reason TEXT NOT NULL,
    detail JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_auth_risk_events_remote_addr ON auth_risk_events(remote_addr);
CREATE INDEX IF NOT EXISTS idx_auth_risk_events_created_at ON auth_risk_events(created_at DESC);

CREATE TABLE IF NOT EXISTS user_profiles (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    email TEXT,
    display_name TEXT,
    remark TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_user_profiles_email ON user_profiles(email);

-- ===== libraries & media =====
CREATE TABLE IF NOT EXISTS libraries (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    root_path TEXT NOT NULL UNIQUE,
    library_type TEXT NOT NULL DEFAULT 'Mixed',
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    scan_interval_hours INTEGER NOT NULL DEFAULT 6,
    scraper_policy JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_libraries_library_type ON libraries(library_type);

CREATE TABLE IF NOT EXISTS library_scan_state (
    library_id UUID PRIMARY KEY REFERENCES libraries(id) ON DELETE CASCADE,
    last_scan_started_at TIMESTAMPTZ,
    last_scan_finished_at TIMESTAMPTZ,
    last_scan_mode TEXT NOT NULL DEFAULT 'full',
    last_scan_cursor TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS media_items (
    id UUID PRIMARY KEY,
    library_id UUID REFERENCES libraries(id) ON DELETE SET NULL,
    item_type TEXT NOT NULL,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    series_id UUID,
    season_number INT,
    episode_number INT,
    runtime_ticks BIGINT,
    bitrate INT,
    stream_url TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    search_text TEXT,
    search_pinyin TEXT,
    search_initials TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_media_items_item_type ON media_items(item_type);
CREATE INDEX IF NOT EXISTS idx_media_items_series_id ON media_items(series_id);
CREATE INDEX IF NOT EXISTS idx_media_items_library_id ON media_items(library_id);
CREATE INDEX IF NOT EXISTS idx_media_items_updated_at ON media_items(updated_at DESC);

CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX IF NOT EXISTS idx_media_items_search_text_trgm ON media_items USING gin (search_text gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_media_items_search_pinyin_trgm ON media_items USING gin (search_pinyin gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_media_items_search_initials_trgm ON media_items USING gin (search_initials gin_trgm_ops);

CREATE TABLE IF NOT EXISTS subtitles (
    id UUID PRIMARY KEY,
    media_item_id UUID NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
    path TEXT NOT NULL UNIQUE,
    language TEXT,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_subtitles_media_item_id ON subtitles(media_item_id);

CREATE TABLE IF NOT EXISTS raw_sidecars (
    id UUID PRIMARY KEY,
    media_item_id UUID NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
    sidecar_type TEXT NOT NULL,
    path TEXT,
    raw_content JSONB NOT NULL DEFAULT '{}'::jsonb,
    normalized_content JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(media_item_id, sidecar_type, path)
);

CREATE INDEX IF NOT EXISTS idx_raw_sidecars_media_item_id ON raw_sidecars(media_item_id);

-- ===== people & cast =====
CREATE TABLE IF NOT EXISTS people (
    id UUID PRIMARY KEY,
    tmdb_id BIGINT UNIQUE,
    name TEXT NOT NULL,
    profile_path TEXT,
    image_path TEXT,
    primary_image_tag TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_people_name ON people(name);
CREATE INDEX IF NOT EXISTS idx_people_tmdb_id ON people(tmdb_id);

CREATE TABLE IF NOT EXISTS media_item_people (
    media_item_id UUID NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
    person_id UUID NOT NULL REFERENCES people(id) ON DELETE CASCADE,
    person_type TEXT NOT NULL,
    role TEXT,
    sort_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (media_item_id, person_id, person_type)
);

CREATE INDEX IF NOT EXISTS idx_media_item_people_item ON media_item_people(media_item_id, sort_order);
CREATE INDEX IF NOT EXISTS idx_media_item_people_person ON media_item_people(person_id);

-- ===== playback =====
CREATE TABLE IF NOT EXISTS playback_sessions (
    id UUID PRIMARY KEY,
    play_session_id TEXT NOT NULL UNIQUE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    media_item_id UUID REFERENCES media_items(id) ON DELETE SET NULL,
    device_name TEXT,
    client_name TEXT,
    play_method TEXT,
    position_ticks BIGINT NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    last_heartbeat_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_playback_sessions_user_id ON playback_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_playback_sessions_updated_at ON playback_sessions(updated_at DESC);

CREATE TABLE IF NOT EXISTS watch_states (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    media_item_id UUID NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
    playback_position_ticks BIGINT NOT NULL DEFAULT 0,
    played BOOLEAN NOT NULL DEFAULT FALSE,
    last_played_at TIMESTAMPTZ,
    is_favorite BOOLEAN DEFAULT FALSE,
    play_count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, media_item_id)
);

CREATE TABLE IF NOT EXISTS media_play_events_daily (
    usage_date DATE NOT NULL,
    media_item_id UUID NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    play_session_id TEXT NOT NULL,
    play_method TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (usage_date, media_item_id, play_session_id)
);

CREATE INDEX IF NOT EXISTS idx_media_play_events_daily_date_count ON media_play_events_daily(usage_date DESC, media_item_id);
CREATE INDEX IF NOT EXISTS idx_media_play_events_daily_user ON media_play_events_daily(user_id, usage_date DESC);

-- ===== jobs & tasks =====
CREATE TABLE IF NOT EXISTS jobs (
    id UUID PRIMARY KEY,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    progress JSONB NOT NULL DEFAULT '{}'::jsonb,
    result JSONB,
    error TEXT,
    attempts INT NOT NULL DEFAULT 0,
    max_attempts INT NOT NULL DEFAULT 3,
    next_retry_at TIMESTAMPTZ,
    cancel_requested BOOLEAN NOT NULL DEFAULT FALSE,
    dead_letter BOOLEAN NOT NULL DEFAULT FALSE,
    trigger_type TEXT NOT NULL DEFAULT 'manual',
    scheduled_for TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status);
CREATE INDEX IF NOT EXISTS idx_jobs_created_at ON jobs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_jobs_kind_created_at ON jobs(kind, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_jobs_trigger_type_created_at ON jobs(trigger_type, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_jobs_scheduled_for ON jobs(scheduled_for);

CREATE TABLE IF NOT EXISTS job_dead_letters (
    id UUID PRIMARY KEY,
    job_id UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    reason TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_job_dead_letters_job_id ON job_dead_letters(job_id);

CREATE TABLE IF NOT EXISTS task_definitions (
    task_key TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT FALSE,
    cron_expr TEXT NOT NULL,
    default_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    max_attempts INT NOT NULL DEFAULT 3,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO task_definitions (task_key, display_name, enabled, cron_expr, default_payload, max_attempts)
VALUES
    ('cleanup_maintenance', '系统清理维护', true, '0 0 * * * *', '{}'::jsonb, 1),
    ('retry_dispatch', '失败任务重试分发', true, '0 * * * * *', '{}'::jsonb, 1),
    ('billing_expire', '计费过期处理', true, '0 */5 * * * *', '{}'::jsonb, 1),
    ('scan_library', '媒体库扫描', false, '0 */30 * * * *', '{"mode":"incremental"}'::jsonb, 3),
    ('metadata_repair', '元数据修复', false, '0 30 3 * * *', '{}'::jsonb, 3),
    ('subtitle_sync', '字幕同步', false, '0 45 3 * * *', '{}'::jsonb, 3),
    ('scraper_fill', '刮削补齐', false, '0 15 4 * * *', '{}'::jsonb, 3),
    ('cache_prewarm', '缓存预热', false, '0 0 5 * * *', '{"limit":100}'::jsonb, 3),
    ('search_reindex', '搜索索引重建', false, '0 0 2 * * *', '{"batch_size":500}'::jsonb, 3)
ON CONFLICT (task_key) DO NOTHING;

-- ===== audit & events =====
CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID PRIMARY KEY,
    actor_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id TEXT,
    detail JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_actor_user_id ON audit_logs(actor_user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);

CREATE TABLE IF NOT EXISTS system_events (
    id UUID PRIMARY KEY,
    event_type TEXT NOT NULL,
    level TEXT NOT NULL DEFAULT 'info',
    source TEXT NOT NULL,
    detail JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_system_events_event_type ON system_events(event_type);
CREATE INDEX IF NOT EXISTS idx_system_events_created_at ON system_events(created_at DESC);

-- ===== tmdb cache =====
CREATE TABLE IF NOT EXISTS tmdb_cache (
    cache_key TEXT PRIMARY KEY,
    query TEXT NOT NULL,
    item_type TEXT NOT NULL,
    response JSONB NOT NULL DEFAULT '{}'::jsonb,
    has_result BOOLEAN NOT NULL DEFAULT FALSE,
    expires_at TIMESTAMPTZ NOT NULL,
    hit_count BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_tmdb_cache_expires_at ON tmdb_cache(expires_at);

CREATE TABLE IF NOT EXISTS tmdb_failures (
    id UUID PRIMARY KEY,
    media_item_id UUID REFERENCES media_items(id) ON DELETE SET NULL,
    item_name TEXT NOT NULL,
    item_type TEXT NOT NULL,
    attempts INT NOT NULL DEFAULT 0,
    error TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_tmdb_failures_item_id ON tmdb_failures(media_item_id);
CREATE INDEX IF NOT EXISTS idx_tmdb_failures_created_at ON tmdb_failures(created_at DESC);

-- ===== storage & lumenbackend =====
CREATE TABLE IF NOT EXISTS storage_configs (
    id UUID PRIMARY KEY,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    config JSONB NOT NULL DEFAULT '{}'::jsonb,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(kind, name)
);

CREATE INDEX IF NOT EXISTS idx_storage_configs_kind ON storage_configs(kind);

CREATE TABLE IF NOT EXISTS web_settings (
    key TEXT PRIMARY KEY,
    value JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS playback_domains (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    base_url TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    priority INT NOT NULL DEFAULT 0,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_playback_domains_enabled_priority
    ON playback_domains(enabled, is_default DESC, priority DESC, updated_at DESC);

CREATE TABLE IF NOT EXISTS user_playback_domain_preferences (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    domain_id UUID NOT NULL REFERENCES playback_domains(id) ON DELETE CASCADE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_user_playback_domain_preferences_domain_id
    ON user_playback_domain_preferences(domain_id);

CREATE TABLE IF NOT EXISTS lumenbackend_nodes (
    node_id TEXT PRIMARY KEY,
    name TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    last_seen_at TIMESTAMPTZ,
    last_version TEXT,
    last_status JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_lumenbackend_nodes_enabled_last_seen
    ON lumenbackend_nodes(enabled, last_seen_at DESC NULLS LAST);

CREATE TABLE IF NOT EXISTS lumenbackend_runtime_configs (
    id UUID PRIMARY KEY,
    scope TEXT NOT NULL,
    scope_key TEXT,
    version BIGINT NOT NULL,
    config JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(scope, scope_key, version)
);

CREATE INDEX IF NOT EXISTS idx_lumenbackend_runtime_configs_scope_latest
    ON lumenbackend_runtime_configs(scope, scope_key, version DESC);

-- Migrate legacy lumenbackend_nodes from web_settings into playback_domains
INSERT INTO playback_domains (id, name, base_url, enabled, priority, is_default)
SELECT
    (
        substr(md5(node), 1, 8) || '-' ||
        substr(md5(node), 9, 4) || '-' ||
        substr(md5(node), 13, 4) || '-' ||
        substr(md5(node), 17, 4) || '-' ||
        substr(md5(node), 21, 12)
    )::uuid,
    format('legacy-%s', row_number() OVER (ORDER BY node)),
    node,
    true,
    0,
    row_number() OVER (ORDER BY node) = 1
FROM (
    SELECT DISTINCT trim(node) AS node
    FROM (
        SELECT jsonb_array_elements_text(value->'storage'->'lumenbackend_nodes') AS node
        FROM web_settings
        WHERE key = 'global'
          AND jsonb_typeof(value->'storage'->'lumenbackend_nodes') = 'array'
    ) t
    WHERE trim(node) <> ''
) nodes
WHERE NOT EXISTS (SELECT 1 FROM playback_domains);

-- ===== stream policies =====
CREATE TABLE IF NOT EXISTS user_stream_policies (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ,
    max_concurrent_streams INT,
    traffic_quota_bytes BIGINT,
    traffic_window_days INT NOT NULL DEFAULT 30,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_user_stream_policies_max_concurrent_streams
        CHECK (max_concurrent_streams IS NULL OR max_concurrent_streams >= 0),
    CONSTRAINT chk_user_stream_policies_traffic_quota_bytes
        CHECK (traffic_quota_bytes IS NULL OR traffic_quota_bytes >= 0),
    CONSTRAINT chk_user_stream_policies_window_days
        CHECK (traffic_window_days > 0)
);

CREATE INDEX IF NOT EXISTS idx_user_stream_policies_expires_at
    ON user_stream_policies(expires_at);

CREATE TABLE IF NOT EXISTS user_stream_usage_daily (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    usage_date DATE NOT NULL,
    bytes_served BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY(user_id, usage_date),
    CONSTRAINT chk_user_stream_usage_daily_bytes_served CHECK (bytes_served >= 0)
);

CREATE INDEX IF NOT EXISTS idx_user_stream_usage_daily_usage_date
    ON user_stream_usage_daily(usage_date DESC);

-- ===== billing =====
CREATE TABLE IF NOT EXISTS wallet_accounts (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    balance NUMERIC(18,2) NOT NULL DEFAULT 0,
    total_recharged NUMERIC(18,2) NOT NULL DEFAULT 0,
    total_spent NUMERIC(18,2) NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_wallet_accounts_balance_non_negative CHECK (balance >= 0),
    CONSTRAINT chk_wallet_accounts_total_recharged_non_negative CHECK (total_recharged >= 0),
    CONSTRAINT chk_wallet_accounts_total_spent_non_negative CHECK (total_spent >= 0)
);

CREATE TABLE IF NOT EXISTS wallet_ledger (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    entry_type TEXT NOT NULL,
    amount NUMERIC(18,2) NOT NULL,
    balance_after NUMERIC(18,2) NOT NULL,
    reference_type TEXT,
    reference_id TEXT,
    note TEXT,
    meta JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_wallet_ledger_non_zero_amount CHECK (amount <> 0)
);

CREATE INDEX IF NOT EXISTS idx_wallet_ledger_user_created_at
    ON wallet_ledger(user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS billing_plans (
    id UUID PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    price NUMERIC(18,2) NOT NULL,
    duration_days INT NOT NULL,
    traffic_quota_bytes BIGINT NOT NULL,
    traffic_window_days INT NOT NULL DEFAULT 30,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_billing_plans_price_positive CHECK (price > 0),
    CONSTRAINT chk_billing_plans_duration_days_positive CHECK (duration_days > 0),
    CONSTRAINT chk_billing_plans_traffic_quota_positive CHECK (traffic_quota_bytes > 0),
    CONSTRAINT chk_billing_plans_traffic_window_days_positive CHECK (traffic_window_days > 0)
);

CREATE INDEX IF NOT EXISTS idx_billing_plans_enabled ON billing_plans(enabled);

CREATE TABLE IF NOT EXISTS billing_plan_subscriptions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    plan_id UUID NOT NULL REFERENCES billing_plans(id),
    plan_code TEXT NOT NULL,
    plan_name TEXT NOT NULL,
    plan_price NUMERIC(18,2) NOT NULL,
    duration_days INT NOT NULL,
    traffic_quota_bytes BIGINT NOT NULL,
    traffic_window_days INT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    started_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    replaced_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_billing_plan_subscriptions_status
        CHECK (status IN ('active', 'replaced', 'expired')),
    CONSTRAINT chk_billing_plan_subscriptions_duration_days_positive
        CHECK (duration_days > 0),
    CONSTRAINT chk_billing_plan_subscriptions_traffic_quota_positive
        CHECK (traffic_quota_bytes > 0),
    CONSTRAINT chk_billing_plan_subscriptions_traffic_window_days_positive
        CHECK (traffic_window_days > 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_billing_plan_subscriptions_active_user
    ON billing_plan_subscriptions(user_id) WHERE status = 'active';

CREATE INDEX IF NOT EXISTS idx_billing_plan_subscriptions_user_created_at
    ON billing_plan_subscriptions(user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS billing_recharge_orders (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    out_trade_no TEXT NOT NULL UNIQUE,
    channel TEXT NOT NULL,
    amount NUMERIC(18,2) NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    subject TEXT NOT NULL,
    notify_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    provider_trade_no TEXT,
    paid_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_billing_recharge_orders_amount_positive CHECK (amount > 0),
    CONSTRAINT chk_billing_recharge_orders_status
        CHECK (status IN ('pending', 'paid', 'expired', 'failed'))
);

CREATE INDEX IF NOT EXISTS idx_billing_recharge_orders_user_created_at
    ON billing_recharge_orders(user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_billing_recharge_orders_status
    ON billing_recharge_orders(status, created_at DESC);

-- ===== notifications =====
CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    notification_type TEXT NOT NULL DEFAULT 'info',
    is_read BOOLEAN NOT NULL DEFAULT FALSE,
    meta JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    read_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_notifications_user_unread
    ON notifications(user_id, is_read, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_notifications_user_created
    ON notifications(user_id, created_at DESC);

-- ===== playlists =====
CREATE TABLE IF NOT EXISTS playlists (
    id UUID PRIMARY KEY,
    owner_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    is_public BOOLEAN NOT NULL DEFAULT FALSE,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_user_id, name)
);

CREATE INDEX IF NOT EXISTS idx_playlists_owner_default_updated
    ON playlists(owner_user_id, is_default DESC, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_playlists_owner_public_updated
    ON playlists(owner_user_id, is_public, updated_at DESC);

CREATE TABLE IF NOT EXISTS playlist_items (
    playlist_id UUID NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    media_item_id UUID NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
    added_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (playlist_id, media_item_id)
);

CREATE INDEX IF NOT EXISTS idx_playlist_items_playlist_added
    ON playlist_items(playlist_id, added_at DESC);

-- ===== invite system =====
CREATE TABLE IF NOT EXISTS user_invite_codes (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    code TEXT NOT NULL UNIQUE,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    reset_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_user_invite_codes_enabled_updated_at
    ON user_invite_codes(enabled, updated_at DESC);

CREATE TABLE IF NOT EXISTS user_invite_relations (
    id UUID PRIMARY KEY,
    inviter_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    invitee_user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    invite_code TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_user_invite_relations_not_self CHECK (inviter_user_id <> invitee_user_id)
);

CREATE INDEX IF NOT EXISTS idx_user_invite_relations_inviter_created_at
    ON user_invite_relations(inviter_user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS invite_rebate_records (
    id UUID PRIMARY KEY,
    invitee_user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    inviter_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recharge_order_id UUID NOT NULL UNIQUE REFERENCES billing_recharge_orders(id) ON DELETE CASCADE,
    recharge_amount NUMERIC(18,2) NOT NULL,
    rebate_rate NUMERIC(5,4) NOT NULL,
    rebate_amount NUMERIC(18,2) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_invite_rebate_rate_range CHECK (rebate_rate >= 0 AND rebate_rate <= 1),
    CONSTRAINT chk_invite_recharge_amount_positive CHECK (recharge_amount > 0),
    CONSTRAINT chk_invite_rebate_amount_non_negative CHECK (rebate_amount >= 0),
    CONSTRAINT chk_invite_rebate_records_not_self CHECK (inviter_user_id <> invitee_user_id)
);

CREATE INDEX IF NOT EXISTS idx_invite_rebate_records_inviter_created_at
    ON invite_rebate_records(inviter_user_id, created_at DESC);
