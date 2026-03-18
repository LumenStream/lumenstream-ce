use std::{
    cmp::Ordering as CmpOrdering,
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        RwLock,
    },
};

use anyhow::Context;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use hmac::{Hmac, Mac};
use md5::compute as md5_compute;
use meilisearch_sdk::{client::Client as MeiliClient, indexes::Index as MeiliIndex};
use ls_agent::{
    AgentProviderCapability, AgentProviderStatus, AgentRequest, AgentRequestCreateInput,
    AgentRequestDetail, AgentRequestEvent, LlmAgentExecutionPlan, LlmParseResult, LlmProvider,
    MoviePilotProvider, USER_STATUS_ACTION_REQUIRED, USER_STATUS_PROCESSING,
    admin_status_to_user_status,
    MoviePilotContext, MoviePilotExactSearchQuery, MoviePilotMediaInfo,
    build_download_payload_with_context,
    build_subscription_payload, choose_best_result, decode_search_contexts,
    infer_manual_actions, infer_workflow_kind, infer_workflow_steps, normalize_int_list,
    summarize_moviepilot_result, workflow_required_capabilities,
};
use ls_config::{AppConfig, AuthConfig, WebAppConfig};
use ls_scraper::{
    BangumiClient, BangumiScraperProvider, ImageAssetPatch, ScrapePlan, ScrapeResult,
    ScraperLibraryPolicy, ScraperPolicySettings, ScraperProviderDescriptor,
    ScraperProviderStatus, ScraperRoutePurpose, ScraperScenario, TmdbScraperProvider,
    TvdbClient, TvdbScraperProvider, infer_scenario_from_item_type, resolve_provider_chain,
};
use ls_domain::jellyfin::{
    AuthenticationResultDto, BaseItemDto, ChapterInfoDto, ItemCountsDto, MediaSourceInfoDto,
    MediaStreamDto, PlaybackInfoResponseDto, PlaybackProgressDto, QueryFilters, QueryResultDto,
    SearchHint, SearchHintResult, SessionInfoDto, SubtitleTrackDto, UserDataDto, UserDto,
    UserItemDataDto, UserPolicyDto,
};
use ls_domain::model::{
    AdminApiKey, AdminPlaybackSession, AuditLogEntry, AuthSession, CreatedAdminApiKey, Job,
    Library, Notification, PlaybackDomain, Playlist, PlaylistItem, LumenBackendNode,
    StorageConfigRecord, TaskDefinition, UserRole,
};
use regex::Regex;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use thiserror::Error;
use tokio::{sync::Mutex, sync::broadcast, time::Instant as TokioInstant};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub use db::{connect, migrate};

#[derive(Clone)]
pub struct AppInfra {
    pub pool: PgPool,
    config: Arc<RwLock<AppConfig>>,
    pub server_id: String,
    pub http_client: Client,
    search_backend: Option<SearchBackend>,
    metrics: Arc<InfraMetrics>,
    tmdb_last_request: Arc<Mutex<Option<TokioInstant>>>,
    resized_image_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
    notification_tx: broadcast::Sender<Notification>,
    task_run_tx: broadcast::Sender<TaskRunEvent>,
    recharge_order_tx: broadcast::Sender<RechargeOrderEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRunEvent {
    pub event: String,
    pub run: Job,
    pub emitted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RechargeOrderEvent {
    pub event: String,
    pub order: BillingRechargeOrder,
    pub emitted_at: DateTime<Utc>,
}

#[derive(Clone)]
struct SearchBackend {
    client: MeiliClient,
    index: MeiliIndex,
}

#[derive(Default)]
struct InfraMetrics {
    tmdb_http_requests_total: AtomicU64,
    tmdb_cache_hits_total: AtomicU64,
    tmdb_cache_misses_total: AtomicU64,
    tmdb_success_total: AtomicU64,
    tmdb_failure_total: AtomicU64,
    jobs_success_total: AtomicU64,
    jobs_failure_total: AtomicU64,
    jobs_retry_scheduled_total: AtomicU64,
    jobs_dead_letter_total: AtomicU64,
    cache_cleanup_removed_total: AtomicU64,
}

impl InfraMetrics {
    fn snapshot(&self) -> Value {
        let tmdb_hits = self.tmdb_cache_hits_total.load(Ordering::Relaxed);
        let tmdb_misses = self.tmdb_cache_misses_total.load(Ordering::Relaxed);
        let tmdb_cache_total = tmdb_hits + tmdb_misses;
        let tmdb_hit_rate = if tmdb_cache_total == 0 {
            0.0
        } else {
            tmdb_hits as f64 / tmdb_cache_total as f64
        };

        let jobs_success = self.jobs_success_total.load(Ordering::Relaxed);
        let jobs_failure = self.jobs_failure_total.load(Ordering::Relaxed);
        let jobs_total = jobs_success + jobs_failure;
        let job_failure_rate = if jobs_total == 0 {
            0.0
        } else {
            jobs_failure as f64 / jobs_total as f64
        };

        let scraper_requests = self.tmdb_http_requests_total.load(Ordering::Relaxed);
        let scraper_success = self.tmdb_success_total.load(Ordering::Relaxed);
        let scraper_failure = self.tmdb_failure_total.load(Ordering::Relaxed);

        json!({
            "tmdb_http_requests_total": scraper_requests,
            "tmdb_cache_hits_total": tmdb_hits,
            "tmdb_cache_misses_total": tmdb_misses,
            "tmdb_hit_rate": tmdb_hit_rate,
            "tmdb_success_total": scraper_success,
            "tmdb_failure_total": scraper_failure,
            "scraper_http_requests_total": scraper_requests,
            "scraper_cache_hits_total": tmdb_hits,
            "scraper_cache_misses_total": tmdb_misses,
            "scraper_hit_rate": tmdb_hit_rate,
            "scraper_success_total": scraper_success,
            "scraper_failure_total": scraper_failure,
            "jobs_success_total": jobs_success,
            "jobs_failure_total": jobs_failure,
            "jobs_retry_scheduled_total": self.jobs_retry_scheduled_total.load(Ordering::Relaxed),
            "jobs_dead_letter_total": self.jobs_dead_letter_total.load(Ordering::Relaxed),
            "job_failure_rate": job_failure_rate,
            "cache_cleanup_removed_total": self.cache_cleanup_removed_total.load(Ordering::Relaxed),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TmdbFillStatus {
    Filled,
    Skipped,
    Failed,
}

#[derive(Debug, Clone)]
struct TmdbPersonCandidate {
    tmdb_id: i64,
    name: String,
    person_type: String,
    role: Option<String>,
    profile_path: Option<String>,
    sort_order: i32,
}

const MEILI_URL: &str = "http://127.0.0.1:7700";
const MEILI_INDEX: &str = "lumenstream_media_items";
fn meili_api_key() -> Option<String> {
    std::env::var("MEILI_MASTER_KEY").ok()
}
const MEILI_MAX_HITS: usize = 500;
const PERSON_ASSOCIATED_MEDIA_LIMIT: i64 = 20;
const WEB_SETTINGS_KEY: &str = "global";
const STREAM_POLICY_ACTIVE_HEARTBEAT_GRACE_SECONDS: i64 = 120;
const MONEY_SCALE: u32 = 2;
const DEFAULT_FAVORITES_PLAYLIST_NAME: &str = "我的喜欢";
const DEFAULT_FAVORITES_PLAYLIST_DESCRIPTION: &str = "默认收藏夹";
const TMDB_API_BASE: &str = "https://api.themoviedb.org/3";
const TMDB_IMAGE_BASE: &str = "https://image.tmdb.org/t/p/original";
const TMDB_TOP_CAST_LIMIT: usize = 20;

fn validate_runtime_bootstrap_credentials(
    has_admin_user: bool,
    auth: &AuthConfig,
) -> anyhow::Result<()> {
    if has_admin_user {
        return Ok(());
    }

    auth.validate_bootstrap_credentials().map_err(|reason| {
        anyhow::anyhow!(
            "bootstrap admin credentials must be configured before first startup: {reason}"
        )
    })
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum InfraError {
    #[error("search backend unavailable")]
    SearchUnavailable,
    #[error("stream access denied: {reason}")]
    StreamAccessDenied { reason: StreamAccessDeniedReason },
    #[error("billing service is disabled")]
    BillingDisabled,
    #[error("billing plan not found")]
    BillingPlanNotFound,
    #[error("billing recharge order not found")]
    BillingOrderNotFound,
    #[error("billing amount is invalid")]
    BillingInvalidAmount,
    #[error("billing channel unsupported")]
    BillingChannelUnsupported,
    #[error("billing balance is insufficient")]
    BillingInsufficientBalance,
    #[error("billing signature is invalid")]
    BillingSignatureInvalid,
    #[error("billing recharge amount does not match order")]
    BillingOrderAmountMismatch,
    #[error("playlist not found")]
    PlaylistNotFound,
    #[error("playlist access denied")]
    PlaylistAccessDenied,
    #[error("playlist input is invalid")]
    PlaylistInvalidInput,
    #[error("playlist conflict")]
    PlaylistConflict,
    #[error("cannot delete default playlist")]
    PlaylistCannotDeleteDefault,
    #[error("media item not found")]
    MediaItemNotFound,
    #[error("invite code is required")]
    InviteCodeRequired,
    #[error("invite code is invalid")]
    InviteCodeInvalid,
    #[error("invite relationship already exists")]
    InviteRelationExists,
    #[error("username already exists")]
    UserAlreadyExists,
    #[error("task already has active run")]
    TaskRunAlreadyActive,
}

#[derive(Debug)]
pub enum AuthenticateUserResult {
    Success(AuthenticationResultDto),
    InvalidCredentials,
    PasswordResetRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordCheckResult {
    Valid,
    Invalid,
    PasswordResetRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamAccessDeniedReason {
    AccountExpired,
    ConcurrentLimitExceeded,
    TrafficQuotaExceeded,
}

impl StreamAccessDeniedReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AccountExpired => "account_expired",
            Self::ConcurrentLimitExceeded => "concurrent_limit_exceeded",
            Self::TrafficQuotaExceeded => "traffic_quota_exceeded",
        }
    }
}

impl std::fmt::Display for StreamAccessDeniedReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Default)]
pub struct ItemsQuery {
    pub user_id: Option<Uuid>,
    pub series_filter: Option<Uuid>,
    pub parent_id: Option<Uuid>,
    pub include_item_types: Vec<String>,
    pub exclude_item_types: Vec<String>,
    pub person_ids: Vec<Uuid>,
    pub search_term: Option<String>,
    pub limit: i64,
    pub start_index: i64,
    pub is_resumable: bool,
    // Wave 3: Sorting
    pub sort_by: Vec<String>,
    pub sort_order: String, // "Ascending" or "Descending"
    pub recursive: bool,
    // Wave 3: Filters
    pub genres: Vec<String>,
    pub tags: Vec<String>,
    pub years: Vec<i32>,
    pub is_favorite: Option<bool>,
    pub is_played: Option<bool>,
    pub min_community_rating: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LibraryItemStat {
    pub library_id: Uuid,
    pub item_count: i64,
    pub last_item_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobStatusCount {
    pub status: String,
    pub count: i64,
}

#[derive(Debug, Clone)]
pub struct TaskDefinitionUpdate {
    pub enabled: Option<bool>,
    pub cron_expr: Option<String>,
    pub default_payload: Option<Value>,
    pub max_attempts: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserStreamPolicy {
    pub user_id: Uuid,
    pub expires_at: Option<DateTime<Utc>>,
    pub max_concurrent_streams: Option<i32>,
    pub traffic_quota_bytes: Option<i64>,
    pub traffic_window_days: i32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UserStreamPolicyUpdate {
    pub expires_at: Option<Option<DateTime<Utc>>>,
    pub max_concurrent_streams: Option<Option<i32>>,
    pub traffic_quota_bytes: Option<Option<i64>>,
    pub traffic_window_days: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserTrafficUsageDaily {
    pub usage_date: String,
    pub bytes_served: i64,
    pub real_bytes_served: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserTrafficUsageSummary {
    pub user_id: Uuid,
    pub window_days: i32,
    pub used_bytes: i64,
    pub real_used_bytes: i64,
    pub quota_bytes: Option<i64>,
    pub remaining_bytes: Option<i64>,
    pub daily: Vec<UserTrafficUsageDaily>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserTrafficUsageMediaItem {
    pub media_item_id: Uuid,
    pub item_name: String,
    pub item_type: String,
    pub bytes_served: i64,
    pub real_bytes_served: i64,
    pub usage_days: i64,
    pub last_usage_date: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserTrafficUsageMediaSummary {
    pub user_id: Uuid,
    pub window_days: i32,
    pub used_bytes: i64,
    pub real_used_bytes: i64,
    pub quota_bytes: Option<i64>,
    pub remaining_bytes: Option<i64>,
    pub unclassified_bytes: i64,
    pub unclassified_real_bytes: i64,
    pub items: Vec<UserTrafficUsageMediaItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopTrafficUser {
    pub user_id: Uuid,
    pub username: String,
    pub used_bytes: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserProfile {
    pub user_id: Uuid,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct UserProfileUpdate {
    pub email: Option<Option<String>>,
    pub display_name: Option<Option<String>>,
    pub remark: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminUserSummaryItem {
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub role: String,
    pub is_admin: bool,
    pub is_disabled: bool,
    pub active_auth_sessions: i64,
    pub active_playback_sessions: i64,
    pub subscription_name: Option<String>,
    pub used_bytes: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminUserSummaryPage {
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
    pub items: Vec<AdminUserSummaryItem>,
}

#[derive(Debug, Clone)]
pub struct AdminUserSummaryQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    pub role: Option<String>,
    pub page: i64,
    pub page_size: i64,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserSessionsSummary {
    pub active_auth_sessions: i64,
    pub active_playback_sessions: i64,
    pub last_auth_seen_at: Option<DateTime<Utc>>,
    pub last_playback_seen_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct AdminUserManageProfile {
    pub user: UserDto,
    pub profile: UserProfile,
    pub stream_policy: UserStreamPolicy,
    pub traffic_usage: UserTrafficUsageSummary,
    pub wallet: Option<WalletAccount>,
    pub subscriptions: Vec<BillingPlanSubscription>,
    pub sessions_summary: UserSessionsSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopPlayedMediaItem {
    pub item_id: Uuid,
    pub name: String,
    pub item_type: String,
    pub runtime_ticks: Option<i64>,
    pub bitrate: Option<i32>,
    pub production_year: Option<i32>,
    pub community_rating: Option<f64>,
    pub overview: Option<String>,
    pub play_count: i64,
    pub unique_users: i64,
}

#[derive(Debug, Clone)]
pub struct PlaybackDomainUpdate {
    pub name: String,
    pub base_url: String,
    pub enabled: bool,
    pub priority: i32,
    pub is_default: bool,
    pub lumenbackend_node_id: Option<Option<String>>,
    pub traffic_multiplier: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct PlaylistUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_public: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct UserItemDataUpdate {
    pub played: Option<bool>,
    pub playback_position_ticks: Option<i64>,
    pub is_favorite: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct LumenBackendNodeRegister {
    pub node_id: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub status: Value,
}

#[derive(Debug, Clone)]
pub struct LumenBackendNodeHeartbeat {
    pub node_id: String,
    pub version: Option<String>,
    pub status: Value,
}

#[derive(Debug, Clone)]
pub struct LumenBackendRuntimeConfig {
    pub version: i64,
    pub config: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct LumenBackendNodeRuntimeConfig {
    pub node_id: String,
    pub version: i64,
    pub config: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct LumenBackendNodeRuntimeSchema {
    pub node_id: String,
    pub schema_version: String,
    pub schema_hash: Option<String>,
    pub schema: Value,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopPlayedMediaSummary {
    pub stat_date: String,
    pub window_days: i32,
    pub items: Vec<TopPlayedMediaItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WalletAccount {
    pub user_id: Uuid,
    pub balance: Decimal,
    pub total_recharged: Decimal,
    pub total_spent: Decimal,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WalletLedgerEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub entry_type: String,
    pub amount: Decimal,
    pub balance_after: Decimal,
    pub reference_type: Option<String>,
    pub reference_id: Option<String>,
    pub note: Option<String>,
    pub meta: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BillingPlan {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub price: Decimal,
    pub duration_days: i32,
    pub traffic_quota_bytes: i64,
    pub traffic_window_days: i32,
    pub permission_group_id: Option<Uuid>,
    pub permission_group_name: Option<String>,
    pub enabled: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct BillingPlanUpsert {
    pub id: Option<Uuid>,
    pub code: String,
    pub name: String,
    pub price: Decimal,
    pub duration_days: i32,
    pub traffic_quota_bytes: i64,
    pub traffic_window_days: i32,
    pub permission_group_id: Option<Uuid>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountPermissionGroup {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub enabled: bool,
    pub domain_ids: Vec<Uuid>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AccountPermissionGroupUpsert {
    pub id: Option<Uuid>,
    pub code: String,
    pub name: String,
    pub enabled: bool,
    pub domain_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BillingPlanSubscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub plan_id: Uuid,
    pub plan_code: String,
    pub plan_name: String,
    pub plan_price: Decimal,
    pub duration_days: i32,
    pub traffic_quota_bytes: i64,
    pub traffic_window_days: i32,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub replaced_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BillingRechargeOrder {
    pub id: Uuid,
    pub user_id: Uuid,
    pub out_trade_no: String,
    pub channel: String,
    pub amount: Decimal,
    pub status: String,
    pub subject: String,
    pub provider_trade_no: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EpayCheckout {
    pub order: BillingRechargeOrder,
    pub pay_url: String,
    pub pay_params: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct BillingProration {
    pub time_ratio: Decimal,
    pub traffic_ratio: Decimal,
    pub applied_ratio: Decimal,
    pub credit_amount: Decimal,
    pub traffic_used_bytes: i64,
    pub traffic_remaining_bytes: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BillingPurchaseResult {
    pub wallet: WalletAccount,
    pub subscription: BillingPlanSubscription,
    pub charged_amount: Decimal,
    pub proration: Option<BillingProration>,
}

#[derive(Debug, Clone)]
pub struct BillingRechargeOrderFilter {
    pub user_id: Option<Uuid>,
    pub status: Option<String>,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct InviteSummary {
    pub code: String,
    pub enabled: bool,
    pub invited_count: i64,
    pub rebate_total: Decimal,
    pub invitee_bonus_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct InviteRelationView {
    pub id: Uuid,
    pub inviter_user_id: Uuid,
    pub inviter_username: String,
    pub invitee_user_id: Uuid,
    pub invitee_username: String,
    pub invite_code: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InviteRebateView {
    pub id: Uuid,
    pub invitee_user_id: Uuid,
    pub invitee_username: String,
    pub inviter_user_id: Uuid,
    pub inviter_username: String,
    pub recharge_order_id: Uuid,
    pub recharge_amount: Decimal,
    pub rebate_rate: Decimal,
    pub rebate_amount: Decimal,
    pub created_at: DateTime<Utc>,
}

impl ItemsQuery {
    fn normalize(mut self) -> Self {
        self.limit = self.limit.clamp(1, 500);
        if self.start_index < 0 {
            self.start_index = 0;
        }
        self.include_item_types = self
            .include_item_types
            .into_iter()
            .filter(|v| !v.trim().is_empty())
            .collect();
        let mut seen_person_ids = HashSet::new();
        self.person_ids.retain(|id| seen_person_ids.insert(*id));
        let mut seen_tags = HashSet::new();
        self.tags = self
            .tags
            .into_iter()
            .map(|tag| tag.trim().to_string())
            .filter(|tag| !tag.is_empty())
            .filter(|tag| seen_tags.insert(tag.to_ascii_lowercase()))
            .collect();
        if let Some(term) = self.search_term.as_ref().map(|v| v.trim().to_string()) {
            if term.is_empty() {
                self.search_term = None;
            } else {
                self.search_term = Some(term);
            }
        }
        self
    }
}
