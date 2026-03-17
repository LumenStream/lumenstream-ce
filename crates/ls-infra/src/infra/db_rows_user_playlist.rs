#[derive(Debug, Clone, FromRow)]
struct NotificationRow {
    id: Uuid,
    user_id: Uuid,
    title: String,
    message: String,
    notification_type: String,
    is_read: bool,
    meta: Value,
    created_at: DateTime<Utc>,
    read_at: Option<DateTime<Utc>>,
}

impl From<NotificationRow> for Notification {
    fn from(value: NotificationRow) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            title: value.title,
            message: value.message,
            notification_type: value.notification_type,
            is_read: value.is_read,
            meta: value.meta,
            created_at: value.created_at,
            read_at: value.read_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct PlaylistRow {
    id: Uuid,
    owner_user_id: Uuid,
    name: String,
    description: String,
    is_public: bool,
    is_default: bool,
    playlist_type: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    item_count: i64,
}

impl From<PlaylistRow> for Playlist {
    fn from(value: PlaylistRow) -> Self {
        Self {
            id: value.id,
            owner_user_id: value.owner_user_id,
            name: value.name,
            description: value.description,
            is_public: value.is_public,
            is_default: value.is_default,
            playlist_type: value.playlist_type,
            item_count: value.item_count,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct PlaylistItemRow {
    playlist_id: Uuid,
    media_item_id: Uuid,
    added_at: DateTime<Utc>,
}

impl From<PlaylistItemRow> for PlaylistItem {
    fn from(value: PlaylistItemRow) -> Self {
        Self {
            playlist_id: value.playlist_id,
            media_item_id: value.media_item_id,
            added_at: value.added_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct UserRow {
    id: Uuid,
    username: String,
    password_hash: String,
    role: String,
    is_admin: bool,
    is_disabled: bool,
}

#[derive(Debug, Clone, FromRow)]
struct UserProfileRow {
    user_id: Uuid,
    email: Option<String>,
    display_name: Option<String>,
    remark: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<UserProfileRow> for UserProfile {
    fn from(value: UserProfileRow) -> Self {
        Self {
            user_id: value.user_id,
            email: value.email,
            display_name: value.display_name,
            remark: value.remark,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct AdminUserSummaryRow {
    id: Uuid,
    username: String,
    role: String,
    is_admin: bool,
    is_disabled: bool,
    created_at: DateTime<Utc>,
    email: Option<String>,
    display_name: Option<String>,
    remark: Option<String>,
    active_auth_sessions: i64,
    active_playback_sessions: i64,
    subscription_name: Option<String>,
    used_bytes: i64,
}

impl From<AdminUserSummaryRow> for AdminUserSummaryItem {
    fn from(value: AdminUserSummaryRow) -> Self {
        Self {
            id: value.id,
            username: value.username,
            email: value.email,
            display_name: value.display_name,
            role: value.role,
            is_admin: value.is_admin,
            is_disabled: value.is_disabled,
            active_auth_sessions: value.active_auth_sessions,
            active_playback_sessions: value.active_playback_sessions,
            subscription_name: value.subscription_name,
            used_bytes: value.used_bytes,
            created_at: value.created_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct UserSessionsSummaryRow {
    active_auth_sessions: i64,
    active_playback_sessions: i64,
    last_auth_seen_at: Option<DateTime<Utc>>,
    last_playback_seen_at: Option<DateTime<Utc>>,
}

impl From<UserSessionsSummaryRow> for UserSessionsSummary {
    fn from(value: UserSessionsSummaryRow) -> Self {
        Self {
            active_auth_sessions: value.active_auth_sessions,
            active_playback_sessions: value.active_playback_sessions,
            last_auth_seen_at: value.last_auth_seen_at,
            last_playback_seen_at: value.last_playback_seen_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct UserStreamPolicyRow {
    user_id: Uuid,
    expires_at: Option<DateTime<Utc>>,
    max_concurrent_streams: Option<i32>,
    traffic_quota_bytes: Option<i64>,
    traffic_window_days: i32,
    updated_at: DateTime<Utc>,
}

impl From<UserStreamPolicyRow> for UserStreamPolicy {
    fn from(value: UserStreamPolicyRow) -> Self {
        Self {
            user_id: value.user_id,
            expires_at: value.expires_at,
            max_concurrent_streams: value.max_concurrent_streams,
            traffic_quota_bytes: value.traffic_quota_bytes,
            traffic_window_days: value.traffic_window_days,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct UserTrafficUsageDailyRow {
    usage_date: chrono::NaiveDate,
    bytes_served: i64,
    real_bytes_served: i64,
}

impl From<UserTrafficUsageDailyRow> for UserTrafficUsageDaily {
    fn from(value: UserTrafficUsageDailyRow) -> Self {
        Self {
            usage_date: value.usage_date.to_string(),
            bytes_served: value.bytes_served,
            real_bytes_served: value.real_bytes_served,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct UserTrafficUsageMediaRow {
    media_item_id: Uuid,
    item_name: String,
    item_type: String,
    bytes_served: i64,
    real_bytes_served: i64,
    usage_days: i64,
    last_usage_date: chrono::NaiveDate,
}

impl From<UserTrafficUsageMediaRow> for UserTrafficUsageMediaItem {
    fn from(value: UserTrafficUsageMediaRow) -> Self {
        Self {
            media_item_id: value.media_item_id,
            item_name: value.item_name,
            item_type: value.item_type,
            bytes_served: value.bytes_served,
            real_bytes_served: value.real_bytes_served,
            usage_days: value.usage_days,
            last_usage_date: value.last_usage_date.to_string(),
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct TopTrafficUserRow {
    user_id: Uuid,
    username: String,
    used_bytes: i64,
}

impl From<TopTrafficUserRow> for TopTrafficUser {
    fn from(value: TopTrafficUserRow) -> Self {
        Self {
            user_id: value.user_id,
            username: value.username,
            used_bytes: value.used_bytes,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct TopPlayedMediaRow {
    media_item_id: Uuid,
    name: String,
    item_type: String,
    runtime_ticks: Option<i64>,
    bitrate: Option<i32>,
    production_year: Option<i32>,
    community_rating: Option<f64>,
    overview: Option<String>,
    play_count: i64,
    unique_users: i64,
}

impl From<TopPlayedMediaRow> for TopPlayedMediaItem {
    fn from(value: TopPlayedMediaRow) -> Self {
        Self {
            item_id: value.media_item_id,
            name: value.name,
            item_type: value.item_type,
            runtime_ticks: value.runtime_ticks,
            bitrate: value.bitrate,
            production_year: value.production_year,
            community_rating: value.community_rating,
            overview: value.overview,
            play_count: value.play_count,
            unique_users: value.unique_users,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct MediaItemRow {
    id: Uuid,
    item_type: String,
    name: String,
    path: String,
    runtime_ticks: Option<i64>,
    bitrate: Option<i32>,
    series_id: Option<Uuid>,
    season_number: Option<i32>,
    episode_number: Option<i32>,
    library_id: Option<Uuid>,
    metadata: Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct MediaItemWithTotalRow {
    id: Uuid,
    item_type: String,
    name: String,
    path: String,
    runtime_ticks: Option<i64>,
    bitrate: Option<i32>,
    series_id: Option<Uuid>,
    season_number: Option<i32>,
    episode_number: Option<i32>,
    library_id: Option<Uuid>,
    metadata: Value,
    created_at: chrono::DateTime<chrono::Utc>,
    total_count: i64,
}

impl From<MediaItemWithTotalRow> for MediaItemRow {
    fn from(value: MediaItemWithTotalRow) -> Self {
        Self {
            id: value.id,
            item_type: value.item_type,
            name: value.name,
            path: value.path,
            runtime_ticks: value.runtime_ticks,
            bitrate: value.bitrate,
            series_id: value.series_id,
            season_number: value.season_number,
            episode_number: value.episode_number,
            library_id: value.library_id,
            metadata: value.metadata,
            created_at: value.created_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct SeasonRow {
    id: Uuid,
    #[sqlx(rename = "series_id")]
    _series_id: Option<Uuid>,
    season_number: Option<i32>,
    name: String,
    path: String,
    #[sqlx(rename = "metadata")]
    _metadata: Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct NextUpRow {
    id: Uuid,
    item_type: String,
    name: String,
    path: String,
    runtime_ticks: Option<i64>,
    bitrate: Option<i32>,
    series_id: Option<Uuid>,
    season_number: Option<i32>,
    episode_number: Option<i32>,
    library_id: Option<Uuid>,
    metadata: Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct StreamTargetRow {
    stream_url: Option<String>,
    metadata: Value,
}

#[derive(Debug, Clone, FromRow)]
struct VersionedMediaSourceRow {
    id: Uuid,
    name: String,
    path: String,
    runtime_ticks: Option<i64>,
    bitrate: Option<i32>,
    stream_url: Option<String>,
    metadata: Value,
    version_group_id: Option<Uuid>,
    version_rank: i32,
}

#[derive(Debug, Clone, FromRow)]
struct RequestedVersionedMediaSourceRow {
    requested_id: Uuid,
    id: Uuid,
    name: String,
    path: String,
    runtime_ticks: Option<i64>,
    bitrate: Option<i32>,
    stream_url: Option<String>,
    metadata: Value,
    version_rank: i32,
}

#[derive(Debug, Clone, FromRow)]
struct CountRow {
    item_type: String,
    count: i64,
}

#[derive(Debug, Clone, FromRow)]
struct SearchHintRow {
    id: Uuid,
    item_type: String,
    name: String,
    metadata: Option<Value>,
}

#[derive(Debug, Clone, FromRow)]
struct GenreRow {
    genre: String,
    item_count: i64,
}

fn genre_to_id(genre: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    genre.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[derive(Debug, Clone, FromRow)]
struct SubtitleRow {
    path: String,
    language: Option<String>,
    is_default: bool,
}

#[derive(Debug, Clone, FromRow)]
struct SubtitleWithItemRow {
    media_item_id: Uuid,
    path: String,
    language: Option<String>,
    is_default: bool,
}

#[derive(Debug, Clone, FromRow)]
struct ResumeItemRow {
    id: Uuid,
    item_type: String,
    name: String,
    path: String,
    runtime_ticks: Option<i64>,
    bitrate: Option<i32>,
    series_id: Option<Uuid>,
    season_number: Option<i32>,
    episode_number: Option<i32>,
    library_id: Option<Uuid>,
    metadata: Value,
    created_at: chrono::DateTime<chrono::Utc>,
    playback_position_ticks: i64,
    played: bool,
}

#[derive(Debug, Clone, FromRow)]
struct WatchStateRow {
    playback_position_ticks: i64,
    played: bool,
    is_favorite: Option<bool>,
}

#[derive(Debug, Clone, FromRow)]
struct UserItemDataRow {
    playback_position_ticks: i64,
    play_count: i32,
    is_favorite: Option<bool>,
    played: bool,
    last_played_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
struct LibraryRow {
    id: Uuid,
    name: String,
    #[sqlx(default)]
    paths: Vec<String>,
    library_type: String,
    enabled: bool,
    scan_interval_hours: i32,
    scraper_policy: Value,
    created_at: DateTime<Utc>,
}

impl From<LibraryRow> for Library {
    fn from(value: LibraryRow) -> Self {
        let root_path = value.paths.first().cloned().unwrap_or_default();
        Self {
            id: value.id,
            name: value.name,
            root_path,
            paths: value.paths,
            library_type: value.library_type,
            enabled: value.enabled,
            scan_interval_hours: value.scan_interval_hours,
            scraper_policy: value.scraper_policy,
            created_at: value.created_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct LibraryItemStatRow {
    library_id: Uuid,
    item_count: i64,
    last_item_updated_at: Option<DateTime<Utc>>,
}

impl From<LibraryItemStatRow> for LibraryItemStat {
    fn from(value: LibraryItemStatRow) -> Self {
        Self {
            library_id: value.library_id,
            item_count: value.item_count,
            last_item_updated_at: value.last_item_updated_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct JobRow {
    id: Uuid,
    kind: String,
    status: String,
    payload: Value,
    progress: Value,
    result: Option<Value>,
    error: Option<String>,
    attempts: i32,
    max_attempts: i32,
    next_retry_at: Option<DateTime<Utc>>,
    #[sqlx(rename = "cancel_requested")]
    _cancel_requested: bool,
    dead_letter: bool,
    trigger_type: Option<String>,
    scheduled_for: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
}

impl From<JobRow> for Job {
    fn from(value: JobRow) -> Self {
        Self {
            id: value.id,
            kind: value.kind,
            status: value.status,
            payload: value.payload,
            progress: value.progress,
            result: value.result,
            error: value.error,
            attempts: value.attempts,
            max_attempts: value.max_attempts,
            next_retry_at: value.next_retry_at,
            dead_letter: value.dead_letter,
            trigger_type: value.trigger_type,
            scheduled_for: value.scheduled_for,
            created_at: value.created_at,
            started_at: value.started_at,
            finished_at: value.finished_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct TaskDefinitionRow {
    task_key: String,
    display_name: String,
    enabled: bool,
    cron_expr: String,
    default_payload: Value,
    max_attempts: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<TaskDefinitionRow> for TaskDefinition {
    fn from(value: TaskDefinitionRow) -> Self {
        Self {
            task_key: value.task_key,
            display_name: value.display_name,
            enabled: value.enabled,
            cron_expr: value.cron_expr,
            default_payload: value.default_payload,
            max_attempts: value.max_attempts,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct JobStatusCountRow {
    status: String,
    count: i64,
}

impl From<JobStatusCountRow> for JobStatusCount {
    fn from(value: JobStatusCountRow) -> Self {
        Self {
            status: value.status,
            count: value.count,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct AuthSessionRow {
    id: Uuid,
    user_id: Uuid,
    user_name: String,
    client: Option<String>,
    device_name: Option<String>,
    device_id: Option<String>,
    remote_addr: Option<String>,
    is_active: bool,
    created_at: DateTime<Utc>,
    last_seen_at: DateTime<Utc>,
}

impl From<AuthSessionRow> for AuthSession {
    fn from(value: AuthSessionRow) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            user_name: value.user_name,
            client: value.client,
            device_name: value.device_name,
            device_id: value.device_id,
            remote_addr: value.remote_addr,
            is_active: value.is_active,
            created_at: value.created_at,
            last_seen_at: value.last_seen_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct AdminApiKeyRow {
    id: Uuid,
    name: String,
    created_at: DateTime<Utc>,
    last_used_at: Option<DateTime<Utc>>,
}

impl From<AdminApiKeyRow> for AdminApiKey {
    fn from(value: AdminApiKeyRow) -> Self {
        Self {
            id: value.id,
            name: value.name,
            created_at: value.created_at,
            last_used_at: value.last_used_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct StorageConfigRow {
    id: Uuid,
    kind: String,
    name: String,
    config: Value,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<StorageConfigRow> for StorageConfigRecord {
    fn from(value: StorageConfigRow) -> Self {
        Self {
            id: value.id,
            kind: value.kind,
            name: value.name,
            config: value.config,
            enabled: value.enabled,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
