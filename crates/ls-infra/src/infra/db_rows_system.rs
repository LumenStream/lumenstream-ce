#[derive(Debug, Clone, FromRow)]
struct PlaybackDomainRow {
    id: Uuid,
    name: String,
    base_url: String,
    enabled: bool,
    priority: i32,
    is_default: bool,
    lumenbackend_node_id: Option<String>,
    traffic_multiplier: f64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<PlaybackDomainRow> for PlaybackDomain {
    fn from(value: PlaybackDomainRow) -> Self {
        Self {
            id: value.id,
            name: value.name,
            base_url: value.base_url,
            enabled: value.enabled,
            priority: value.priority,
            is_default: value.is_default,
            lumenbackend_node_id: value.lumenbackend_node_id,
            traffic_multiplier: value.traffic_multiplier,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct AccountPermissionGroupRow {
    id: Uuid,
    code: String,
    name: String,
    enabled: bool,
    domain_ids: Vec<Uuid>,
    updated_at: DateTime<Utc>,
}

impl From<AccountPermissionGroupRow> for AccountPermissionGroup {
    fn from(value: AccountPermissionGroupRow) -> Self {
        Self {
            id: value.id,
            code: value.code,
            name: value.name,
            enabled: value.enabled,
            domain_ids: value.domain_ids,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct LumenBackendNodeRow {
    node_id: String,
    name: Option<String>,
    enabled: bool,
    last_seen_at: Option<DateTime<Utc>>,
    last_version: Option<String>,
    last_status: Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<LumenBackendNodeRow> for LumenBackendNode {
    fn from(value: LumenBackendNodeRow) -> Self {
        Self {
            node_id: value.node_id,
            name: value.name,
            enabled: value.enabled,
            last_seen_at: value.last_seen_at,
            last_version: value.last_version,
            last_status: value.last_status,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct LumenBackendRuntimeConfigRow {
    version: i64,
    config: Value,
}

#[derive(Debug, Clone, FromRow)]
struct LumenBackendRuntimeSchemaRow {
    node_id: String,
    schema_version: String,
    schema_hash: Option<String>,
    schema: Value,
    updated_at: DateTime<Utc>,
}

impl From<LumenBackendRuntimeSchemaRow> for LumenBackendNodeRuntimeSchema {
    fn from(value: LumenBackendRuntimeSchemaRow) -> Self {
        Self {
            node_id: value.node_id,
            schema_version: value.schema_version,
            schema_hash: value.schema_hash,
            schema: value.schema,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TmdbCacheStatsRow {
    pub total_entries: i64,
    pub entries_with_result: i64,
    pub expired_entries: i64,
    pub total_hits: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TmdbFailureRow {
    pub id: Uuid,
    pub media_item_id: Option<Uuid>,
    pub item_name: String,
    pub item_type: String,
    pub attempts: i32,
    pub error: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct TmdbFillItemRow {
    id: Uuid,
    library_id: Option<Uuid>,
    name: String,
    item_type: String,
    path: String,
    season_number: Option<i32>,
    episode_number: Option<i32>,
    metadata: Value,
}

#[derive(Debug, Clone, FromRow)]
struct PersonRow {
    id: Uuid,
    name: String,
    image_path: Option<String>,
    primary_image_tag: Option<String>,
    metadata: Value,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct SearchIndexRow {
    id: Uuid,
    name: String,
    item_type: String,
    library_id: Option<Uuid>,
    series_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
struct SearchIndexDocument {
    id: String,
    name: String,
    name_pinyin: String,
    name_initials: String,
    item_type: String,
    library_id: Option<String>,
    series_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchIndexHit {
    id: String,
    item_type: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
struct TmdbCacheRow {
    response: Value,
    has_result: bool,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct PlaybackSessionRow {
    id: Uuid,
    play_session_id: String,
    user_id: Uuid,
    user_name: String,
    media_item_id: Option<Uuid>,
    media_item_name: Option<String>,
    device_name: Option<String>,
    client_name: Option<String>,
    play_method: Option<String>,
    position_ticks: i64,
    is_active: bool,
    last_heartbeat_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<PlaybackSessionRow> for AdminPlaybackSession {
    fn from(value: PlaybackSessionRow) -> Self {
        Self {
            id: value.id,
            play_session_id: value.play_session_id,
            user_id: value.user_id,
            user_name: value.user_name,
            media_item_id: value.media_item_id,
            media_item_name: value.media_item_name,
            device_name: value.device_name,
            client_name: value.client_name,
            play_method: value.play_method,
            position_ticks: value.position_ticks,
            is_active: value.is_active,
            last_heartbeat_at: value.last_heartbeat_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct AuditLogRow {
    id: Uuid,
    actor_user_id: Option<Uuid>,
    actor_username: Option<String>,
    action: String,
    target_type: String,
    target_id: Option<String>,
    detail: Value,
    created_at: DateTime<Utc>,
}

impl From<AuditLogRow> for AuditLogEntry {
    fn from(value: AuditLogRow) -> Self {
        Self {
            id: value.id,
            actor_user_id: value.actor_user_id,
            actor_username: value.actor_username,
            action: value.action,
            target_type: value.target_type,
            target_id: value.target_id,
            detail: value.detail,
            created_at: value.created_at,
        }
    }
}
