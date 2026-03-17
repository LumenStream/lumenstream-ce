use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum UserRole {
    Admin,
    #[default]
    Viewer,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Admin => "Admin",
            Self::Viewer => "Viewer",
        }
    }

    pub fn from_db(raw: &str) -> Self {
        match raw {
            "Admin" => Self::Admin,
            _ => Self::Viewer,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub role: UserRole,
    pub is_admin: bool,
    pub is_disabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub token: String,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub id: Uuid,
    pub name: String,
    pub root_path: String,
    #[serde(default)]
    pub paths: Vec<String>,
    pub library_type: String,
    pub enabled: bool,
    pub scan_interval_hours: i32,
    #[serde(default)]
    pub scraper_policy: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    pub id: Uuid,
    pub library_id: Option<Uuid>,
    pub item_type: String,
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub version_group_id: Option<Uuid>,
    #[serde(default)]
    pub version_rank: i32,
    pub series_id: Option<Uuid>,
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
    pub runtime_ticks: Option<i64>,
    pub bitrate: Option<i32>,
    pub stream_url: Option<String>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtitle {
    pub id: Uuid,
    pub media_item_id: Uuid,
    pub path: String,
    pub language: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackEvent {
    pub play_session_id: Option<String>,
    pub item_id: Option<Uuid>,
    pub position_ticks: Option<i64>,
    pub device_name: Option<String>,
    pub client_name: Option<String>,
    pub play_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminPlaybackSession {
    pub id: Uuid,
    pub play_session_id: String,
    pub user_id: Uuid,
    pub user_name: String,
    pub media_item_id: Option<Uuid>,
    pub media_item_name: Option<String>,
    pub device_name: Option<String>,
    pub client_name: Option<String>,
    pub play_method: Option<String>,
    pub position_ticks: i64,
    pub is_active: bool,
    pub last_heartbeat_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub client: Option<String>,
    pub device_name: Option<String>,
    pub device_id: Option<String>,
    pub remote_addr: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminApiKey {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedAdminApiKey {
    pub id: Uuid,
    pub name: String,
    pub api_key: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_username: Option<String>,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<String>,
    pub detail: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub kind: String,
    pub status: String,
    pub payload: Value,
    pub progress: Value,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub attempts: i32,
    pub max_attempts: i32,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub dead_letter: bool,
    pub trigger_type: Option<String>,
    pub scheduled_for: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub task_key: String,
    pub display_name: String,
    pub enabled: bool,
    pub cron_expr: String,
    pub default_payload: Value,
    pub max_attempts: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfigRecord {
    pub id: Uuid,
    pub kind: String,
    pub name: String,
    pub config: Value,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackDomain {
    pub id: Uuid,
    pub name: String,
    pub base_url: String,
    pub enabled: bool,
    pub priority: i32,
    pub is_default: bool,
    pub lumenbackend_node_id: Option<String>,
    pub traffic_multiplier: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPlaybackDomainPreference {
    pub user_id: Uuid,
    pub domain_id: Uuid,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LumenBackendNode {
    pub node_id: String,
    pub name: Option<String>,
    pub enabled: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub last_version: Option<String>,
    pub last_status: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub message: String,
    pub notification_type: String,
    pub is_read: bool,
    pub meta: Value,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub name: String,
    pub description: String,
    pub is_public: bool,
    pub is_default: bool,
    pub playlist_type: String,
    pub item_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistItem {
    pub playlist_id: Uuid,
    pub media_item_id: Uuid,
    pub added_at: DateTime<Utc>,
}
