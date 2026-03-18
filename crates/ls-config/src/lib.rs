use std::{collections::HashSet, env, fs, path::Path};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file {path}: {source}")]
    Read {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to parse config file {path}: {source}")]
    Parse {
        path: String,
        source: serde_yaml::Error,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub cors_allow_origins: Vec<String>,
    /// Maximum request body size in bytes (default: 10 MiB).
    #[serde(default = "default_max_upload_body_bytes")]
    pub max_upload_body_bytes: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            base_url: String::new(),
            cors_allow_origins: vec![],
            max_upload_body_bytes: default_max_upload_body_bytes(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_database_url")]
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: default_database_url(),
            max_connections: default_max_connections(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InviteConfig {
    #[serde(default)]
    pub force_on_register: bool,
    #[serde(default)]
    pub invitee_bonus_enabled: bool,
    #[serde(default)]
    pub invitee_bonus_amount: Decimal,
    #[serde(default)]
    pub inviter_rebate_enabled: bool,
    #[serde(default)]
    pub inviter_rebate_rate: Decimal,
}

impl Default for InviteConfig {
    fn default() -> Self {
        Self {
            force_on_register: false,
            invitee_bonus_enabled: false,
            invitee_bonus_amount: Decimal::ZERO,
            inviter_rebate_enabled: false,
            inviter_rebate_rate: Decimal::ZERO,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    #[serde(default = "default_token_ttl_hours")]
    pub token_ttl_hours: i64,
    #[serde(default = "default_admin_username")]
    pub bootstrap_admin_user: String,
    #[serde(default = "default_admin_password")]
    pub bootstrap_admin_password: String,
    #[serde(default = "default_admin_api_key_prefix")]
    pub admin_api_key_prefix: String,
    #[serde(default = "default_max_failed_attempts")]
    pub max_failed_attempts: i32,
    #[serde(default = "default_risk_window_seconds")]
    pub risk_window_seconds: i64,
    #[serde(default = "default_risk_block_seconds")]
    pub risk_block_seconds: i64,
    #[serde(default)]
    pub invite: InviteConfig,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            token_ttl_hours: default_token_ttl_hours(),
            bootstrap_admin_user: default_admin_username(),
            bootstrap_admin_password: default_admin_password(),
            admin_api_key_prefix: default_admin_api_key_prefix(),
            max_failed_attempts: default_max_failed_attempts(),
            risk_window_seconds: default_risk_window_seconds(),
            risk_block_seconds: default_risk_block_seconds(),
            invite: InviteConfig::default(),
        }
    }
}

impl AuthConfig {
    pub fn validate_bootstrap_credentials(&self) -> Result<(), &'static str> {
        let username = self.bootstrap_admin_user.trim();
        if username.is_empty() {
            return Err("auth.bootstrap_admin_user is required");
        }

        let password = self.bootstrap_admin_password.trim();
        if password.is_empty() {
            return Err("auth.bootstrap_admin_password is required");
        }

        if username.eq_ignore_ascii_case("admin") && password == "admin123" {
            return Err("auth.bootstrap_admin_password cannot use legacy default credentials");
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanConfig {
    #[serde(default = "default_library_name")]
    pub default_library_name: String,
    #[serde(default)]
    pub default_library_paths: Vec<String>,
    #[serde(default = "default_subtitle_exts")]
    pub subtitle_extensions: Vec<String>,
    #[serde(default = "default_local_media_exts")]
    pub local_media_exts: Vec<String>,
    #[serde(default = "default_scan_grace_seconds")]
    pub incremental_grace_seconds: i64,
    #[serde(default = "default_mediainfo_cache_dir")]
    pub mediainfo_cache_dir: String,
}

#[derive(Debug, Deserialize)]
struct ScanConfigWire {
    #[serde(default = "default_library_name")]
    default_library_name: String,
    #[serde(default)]
    default_library_paths: Vec<String>,
    #[serde(default)]
    default_library_path: String,
    #[serde(default = "default_subtitle_exts")]
    subtitle_extensions: Vec<String>,
    #[serde(default = "default_local_media_exts")]
    local_media_exts: Vec<String>,
    #[serde(default = "default_scan_grace_seconds")]
    incremental_grace_seconds: i64,
    #[serde(default = "default_mediainfo_cache_dir")]
    mediainfo_cache_dir: String,
}

impl<'de> Deserialize<'de> for ScanConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = ScanConfigWire::deserialize(deserializer)?;
        let mut merged_paths = wire
            .default_library_paths
            .into_iter()
            .filter_map(|path| normalize_scan_library_path(path.as_str()))
            .collect::<Vec<_>>();
        if let Some(legacy_path) = normalize_scan_library_path(&wire.default_library_path) {
            merged_paths.push(legacy_path);
        }

        let mut dedup = HashSet::new();
        let mut default_library_paths = Vec::new();
        for path in merged_paths {
            let key = path.to_lowercase();
            if dedup.insert(key) {
                default_library_paths.push(path);
            }
        }

        Ok(Self {
            default_library_name: wire.default_library_name,
            default_library_paths,
            subtitle_extensions: wire.subtitle_extensions,
            local_media_exts: normalize_local_media_exts(wire.local_media_exts),
            incremental_grace_seconds: wire.incremental_grace_seconds,
            mediainfo_cache_dir: wire.mediainfo_cache_dir,
        })
    }
}

fn normalize_scan_library_path(raw: &str) -> Option<String> {
    let mut value = raw.trim().to_string();
    while value.len() > 1 && value.ends_with('/') {
        value.pop();
    }
    if value.is_empty() { None } else { Some(value) }
}

fn normalize_media_extension(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_start_matches('.').to_ascii_lowercase();
    if trimmed.is_empty() || trimmed == "strm" {
        None
    } else {
        Some(trimmed)
    }
}

fn normalize_local_media_exts(raw: Vec<String>) -> Vec<String> {
    let mut dedup = HashSet::new();
    raw.into_iter()
        .filter_map(|item| normalize_media_extension(&item))
        .filter(|item| dedup.insert(item.clone()))
        .collect()
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            default_library_name: default_library_name(),
            default_library_paths: Vec::new(),
            subtitle_extensions: default_subtitle_exts(),
            local_media_exts: default_local_media_exts(),
            incremental_grace_seconds: default_scan_grace_seconds(),
            mediainfo_cache_dir: default_mediainfo_cache_dir(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    #[serde(default)]
    pub gdrive_enabled: bool,
    #[serde(default)]
    pub s3_enabled: bool,
    #[serde(default)]
    pub gdrive_accounts: Vec<String>,
    #[serde(default = "default_s3_cache_dir")]
    pub s3_cache_dir: String,
    #[serde(default = "default_s3_cache_ttl_seconds")]
    pub s3_cache_ttl_seconds: i64,
    #[serde(default)]
    pub segment_gateway_base_url: String,
    #[serde(default)]
    pub prefer_segment_gateway: bool,
    #[serde(default)]
    pub lumenbackend_enabled: bool,
    #[serde(default)]
    pub lumenbackend_nodes: Vec<String>,
    #[serde(default = "default_lumenbackend_route")]
    pub lumenbackend_route: String,
    #[serde(default = "default_local_stream_route")]
    pub local_stream_route: String,
    #[serde(default)]
    pub lumenbackend_stream_signing_key: String,
    #[serde(default = "default_lumenbackend_stream_token_ttl_seconds")]
    pub lumenbackend_stream_token_ttl_seconds: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            gdrive_enabled: false,
            s3_enabled: false,
            gdrive_accounts: Vec::new(),
            s3_cache_dir: default_s3_cache_dir(),
            s3_cache_ttl_seconds: default_s3_cache_ttl_seconds(),
            segment_gateway_base_url: String::new(),
            prefer_segment_gateway: false,
            lumenbackend_enabled: false,
            lumenbackend_nodes: Vec::new(),
            lumenbackend_route: default_lumenbackend_route(),
            local_stream_route: default_local_stream_route(),
            lumenbackend_stream_signing_key: String::new(),
            lumenbackend_stream_token_ttl_seconds: default_lumenbackend_stream_token_ttl_seconds(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TmdbConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_tmdb_language")]
    pub language: String,
    #[serde(default = "default_tmdb_timeout_seconds")]
    pub timeout_seconds: u64,
    #[serde(default = "default_tmdb_request_interval_ms")]
    pub request_interval_ms: u64,
    #[serde(default = "default_tmdb_cache_ttl_seconds")]
    pub cache_ttl_seconds: i64,
    #[serde(default = "default_tmdb_retry_attempts")]
    pub retry_attempts: u32,
    #[serde(default = "default_tmdb_retry_backoff_ms")]
    pub retry_backoff_ms: u64,
    #[serde(default = "default_tmdb_person_image_cache_dir")]
    pub person_image_cache_dir: String,
}

impl Default for TmdbConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: String::new(),
            language: default_tmdb_language(),
            timeout_seconds: default_tmdb_timeout_seconds(),
            request_interval_ms: default_tmdb_request_interval_ms(),
            cache_ttl_seconds: default_tmdb_cache_ttl_seconds(),
            retry_attempts: default_tmdb_retry_attempts(),
            retry_backoff_ms: default_tmdb_retry_backoff_ms(),
            person_image_cache_dir: default_tmdb_person_image_cache_dir(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScraperConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_scraper_default_strategy")]
    pub default_strategy: String,
    #[serde(default = "default_scraper_providers")]
    pub providers: Vec<String>,
    #[serde(default = "default_scraper_default_routes")]
    pub default_routes: ScraperDefaultRoutes,
    #[serde(default)]
    pub tvdb: ScraperTvdbConfig,
    #[serde(default)]
    pub bangumi: ScraperBangumiConfig,
}

impl Default for ScraperConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_strategy: default_scraper_default_strategy(),
            providers: default_scraper_providers(),
            default_routes: default_scraper_default_routes(),
            tvdb: ScraperTvdbConfig::default(),
            bangumi: ScraperBangumiConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScraperDefaultRoutes {
    #[serde(default)]
    pub movie: Vec<String>,
    #[serde(default)]
    pub series: Vec<String>,
    #[serde(default)]
    pub image: Vec<String>,
}

impl Default for ScraperDefaultRoutes {
    fn default() -> Self {
        default_scraper_default_routes()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScraperTvdbConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_scraper_tvdb_base_url")]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub pin: String,
    #[serde(default = "default_scraper_provider_timeout_seconds")]
    pub timeout_seconds: u64,
}

impl Default for ScraperTvdbConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: default_scraper_tvdb_base_url(),
            api_key: String::new(),
            pin: String::new(),
            timeout_seconds: default_scraper_provider_timeout_seconds(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScraperBangumiConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_scraper_bangumi_base_url")]
    pub base_url: String,
    #[serde(default)]
    pub access_token: String,
    #[serde(default = "default_scraper_provider_timeout_seconds")]
    pub timeout_seconds: u64,
    #[serde(default = "default_scraper_bangumi_user_agent")]
    pub user_agent: String,
}

impl Default for ScraperBangumiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: default_scraper_bangumi_base_url(),
            access_token: String::new(),
            timeout_seconds: default_scraper_provider_timeout_seconds(),
            user_agent: default_scraper_bangumi_user_agent(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    #[serde(default)]
    pub admin_allow_ips: Vec<String>,
    #[serde(default = "default_trust_x_forwarded_for")]
    pub trust_x_forwarded_for: bool,
    #[serde(default)]
    pub trusted_proxies: Vec<String>,
    #[serde(default = "default_user_max_concurrent_streams")]
    pub default_user_max_concurrent_streams: i32,
    #[serde(default = "default_user_traffic_quota_bytes")]
    pub default_user_traffic_quota_bytes: i64,
    #[serde(default = "default_user_traffic_window_days")]
    pub default_user_traffic_window_days: i32,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            admin_allow_ips: Vec::new(),
            trust_x_forwarded_for: default_trust_x_forwarded_for(),
            trusted_proxies: Vec::new(),
            default_user_max_concurrent_streams: default_user_max_concurrent_streams(),
            default_user_traffic_quota_bytes: default_user_traffic_quota_bytes(),
            default_user_traffic_window_days: default_user_traffic_window_days(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ObservabilityConfig {
    #[serde(default = "default_metrics_enabled")]
    pub metrics_enabled: bool,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            metrics_enabled: default_metrics_enabled(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
    #[serde(default = "default_log_output")]
    pub output: String,
    #[serde(default = "default_log_file_path")]
    pub file_path: String,
    #[serde(default = "default_log_max_size_mb")]
    pub max_size_mb: u64,
    #[serde(default = "default_log_max_files")]
    pub max_files: u32,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            output: default_log_output(),
            file_path: default_log_file_path(),
            max_size_mb: default_log_max_size_mb(),
            max_files: default_log_max_files(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JobsConfig {
    #[serde(default = "default_retry_base_seconds")]
    pub retry_base_seconds: i64,
    #[serde(default = "default_retry_max_seconds")]
    pub retry_max_seconds: i64,
}

impl Default for JobsConfig {
    fn default() -> Self {
        Self {
            retry_base_seconds: default_retry_base_seconds(),
            retry_max_seconds: default_retry_max_seconds(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchedulerConfig {
    #[serde(default = "default_scheduler_enabled")]
    pub enabled: bool,
    #[serde(default = "default_scheduler_cleanup_interval_seconds")]
    pub cleanup_interval_seconds: i64,
    #[serde(default = "default_scheduler_job_retry_interval_seconds")]
    pub job_retry_interval_seconds: i64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: default_scheduler_enabled(),
            cleanup_interval_seconds: default_scheduler_cleanup_interval_seconds(),
            job_retry_interval_seconds: default_scheduler_job_retry_interval_seconds(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EpayConfig {
    #[serde(default)]
    pub gateway_url: String,
    #[serde(default)]
    pub pid: String,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub notify_url: String,
    #[serde(default)]
    pub return_url: String,
    #[serde(default)]
    pub sitename: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BillingConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_billing_min_recharge_amount")]
    pub min_recharge_amount: Decimal,
    #[serde(default = "default_billing_max_recharge_amount")]
    pub max_recharge_amount: Decimal,
    #[serde(default = "default_billing_order_expire_minutes")]
    pub order_expire_minutes: i64,
    #[serde(default = "default_billing_channels")]
    pub channels: Vec<String>,
    #[serde(default)]
    pub epay: EpayConfig,
}

impl Default for BillingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_recharge_amount: default_billing_min_recharge_amount(),
            max_recharge_amount: default_billing_max_recharge_amount(),
            order_expire_minutes: default_billing_order_expire_minutes(),
            channels: default_billing_channels(),
            epay: EpayConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentMoviePilotFilterConfig {
    #[serde(default = "default_agent_mp_min_seeders")]
    pub min_seeders: i32,
    #[serde(default = "default_agent_mp_max_movie_size_gb")]
    pub max_movie_size_gb: f64,
    #[serde(default = "default_agent_mp_max_episode_size_gb")]
    pub max_episode_size_gb: f64,
    #[serde(default = "default_agent_mp_preferred_resource_pix")]
    pub preferred_resource_pix: Vec<String>,
    #[serde(default = "default_agent_mp_preferred_video_encode")]
    pub preferred_video_encode: Vec<String>,
    #[serde(default = "default_agent_mp_preferred_resource_type")]
    pub preferred_resource_type: Vec<String>,
    #[serde(default = "default_agent_mp_preferred_labels")]
    pub preferred_labels: Vec<String>,
    #[serde(default = "default_agent_mp_excluded_keywords")]
    pub excluded_keywords: Vec<String>,
}

impl Default for AgentMoviePilotFilterConfig {
    fn default() -> Self {
        Self {
            min_seeders: default_agent_mp_min_seeders(),
            max_movie_size_gb: default_agent_mp_max_movie_size_gb(),
            max_episode_size_gb: default_agent_mp_max_episode_size_gb(),
            preferred_resource_pix: default_agent_mp_preferred_resource_pix(),
            preferred_video_encode: default_agent_mp_preferred_video_encode(),
            preferred_resource_type: default_agent_mp_preferred_resource_type(),
            preferred_labels: default_agent_mp_preferred_labels(),
            excluded_keywords: default_agent_mp_excluded_keywords(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentLlmConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_agent_llm_base_url")]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_agent_llm_model")]
    pub model: String,
}

impl Default for AgentLlmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: default_agent_llm_base_url(),
            api_key: String::new(),
            model: default_agent_llm_model(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentMoviePilotConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default = "default_agent_mp_timeout_seconds")]
    pub timeout_seconds: i64,
    #[serde(default = "default_true")]
    pub search_download_enabled: bool,
    #[serde(default = "default_true")]
    pub subscribe_fallback_enabled: bool,
    #[serde(default)]
    pub filter: AgentMoviePilotFilterConfig,
}

impl Default for AgentMoviePilotConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: String::new(),
            username: String::new(),
            password: String::new(),
            timeout_seconds: default_agent_mp_timeout_seconds(),
            search_download_enabled: true,
            subscribe_fallback_enabled: true,
            filter: AgentMoviePilotFilterConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_agent_auto_mode")]
    pub auto_mode: String,
    #[serde(default)]
    pub missing_scan_enabled: bool,
    #[serde(default = "default_agent_missing_scan_cron")]
    pub missing_scan_cron: String,
    #[serde(default = "default_true")]
    pub auto_close_on_library_hit: bool,
    #[serde(default = "default_true")]
    pub review_required_on_parse_ambiguity: bool,
    #[serde(default = "default_true")]
    pub feedback_auto_route: bool,
    #[serde(default)]
    pub llm: AgentLlmConfig,
    #[serde(default)]
    pub moviepilot: AgentMoviePilotConfig,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_mode: default_agent_auto_mode(),
            missing_scan_enabled: false,
            missing_scan_cron: default_agent_missing_scan_cron(),
            auto_close_on_library_hit: true,
            review_required_on_parse_ambiguity: true,
            feedback_auto_route: true,
            llm: AgentLlmConfig::default(),
            moviepilot: AgentMoviePilotConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum EditionChannel {
    #[default]
    Ce,
    Ee,
}

impl EditionChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ce => "ce",
            Self::Ee => "ee",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "ce" | "community" | "community-edition" => Some(Self::Ce),
            "ee" | "enterprise" | "commercial" | "pro" => Some(Self::Ee),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct EditionFeatureOverrides {
    #[serde(default)]
    pub billing: Option<bool>,
    #[serde(default)]
    pub advanced_traffic_controls: Option<bool>,
    #[serde(default)]
    pub invite_rewards: Option<bool>,
    #[serde(default)]
    pub audit_log_export: Option<bool>,
    #[serde(default)]
    pub request_agent: Option<bool>,
    #[serde(default)]
    pub playback_routing: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EditionConfig {
    #[serde(default)]
    pub channel: EditionChannel,
    #[serde(default)]
    pub overrides: EditionFeatureOverrides,
}

impl Default for EditionConfig {
    fn default() -> Self {
        Self {
            channel: EditionChannel::Ce,
            overrides: EditionFeatureOverrides::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct EditionCapabilities {
    pub edition: String,
    pub billing_enabled: bool,
    pub advanced_traffic_controls_enabled: bool,
    pub invite_rewards_enabled: bool,
    pub audit_log_export_enabled: bool,
    pub request_agent_enabled: bool,
    pub playback_routing_enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub scan: ScanConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub tmdb: TmdbConfig,
    #[serde(default)]
    pub scraper: ScraperConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub observability: ObservabilityConfig,
    #[serde(default)]
    pub log: LogConfig,
    #[serde(default)]
    pub jobs: JobsConfig,
    #[serde(default)]
    pub scheduler: SchedulerConfig,
    #[serde(default)]
    pub billing: BillingConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub edition: EditionConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WebAppConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub scan: ScanConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub tmdb: TmdbConfig,
    #[serde(default)]
    pub scraper: ScraperConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub observability: ObservabilityConfig,
    #[serde(default)]
    pub log: LogConfig,
    #[serde(default)]
    pub jobs: JobsConfig,
    #[serde(default)]
    pub scheduler: SchedulerConfig,
    #[serde(default)]
    pub billing: BillingConfig,
    #[serde(default)]
    pub agent: AgentConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct BootstrapFileConfig {
    #[serde(default)]
    database: DatabaseConfig,
}

impl AppConfig {
    pub fn edition_capabilities(&self) -> EditionCapabilities {
        let (
            mut billing_enabled,
            mut advanced_traffic_controls_enabled,
            mut invite_rewards_enabled,
            mut audit_log_export_enabled,
        ) = match self.edition.channel {
            EditionChannel::Ce => (false, false, false, false),
            EditionChannel::Ee => (true, true, true, true),
        };
        let mut request_agent_enabled = true;
        let mut playback_routing_enabled = true;

        if let Some(value) = self.edition.overrides.billing {
            billing_enabled = value;
        }
        if let Some(value) = self.edition.overrides.advanced_traffic_controls {
            advanced_traffic_controls_enabled = value;
        }
        if let Some(value) = self.edition.overrides.invite_rewards {
            invite_rewards_enabled = value;
        }
        if let Some(value) = self.edition.overrides.audit_log_export {
            audit_log_export_enabled = value;
        }
        if let Some(value) = self.edition.overrides.request_agent {
            request_agent_enabled = value;
        }
        if let Some(value) = self.edition.overrides.playback_routing {
            playback_routing_enabled = value;
        }

        EditionCapabilities {
            edition: self.edition.channel.as_str().to_string(),
            billing_enabled,
            advanced_traffic_controls_enabled,
            invite_rewards_enabled,
            audit_log_export_enabled,
            request_agent_enabled,
            playback_routing_enabled,
        }
    }

    fn normalize_for_edition(&mut self) {
        let capabilities = self.edition_capabilities();

        if !capabilities.billing_enabled {
            self.billing.enabled = false;
        }

        if !capabilities.invite_rewards_enabled {
            self.auth.invite.invitee_bonus_enabled = false;
            self.auth.invite.invitee_bonus_amount = Decimal::ZERO;
            self.auth.invite.inviter_rebate_enabled = false;
            self.auth.invite.inviter_rebate_rate = Decimal::ZERO;
        }

        if !capabilities.request_agent_enabled {
            self.agent.enabled = false;
            self.agent.missing_scan_enabled = false;
            self.agent.moviepilot.enabled = false;
        }
    }

    pub fn normalize_web_config_for_edition(&self, web: &mut WebAppConfig) {
        let capabilities = self.edition_capabilities();

        if !capabilities.billing_enabled {
            web.billing.enabled = false;
        }

        if !capabilities.invite_rewards_enabled {
            web.auth.invite.invitee_bonus_enabled = false;
            web.auth.invite.invitee_bonus_amount = Decimal::ZERO;
            web.auth.invite.inviter_rebate_enabled = false;
            web.auth.invite.inviter_rebate_rate = Decimal::ZERO;
        }

        if !capabilities.request_agent_enabled {
            web.agent.enabled = false;
            web.agent.missing_scan_enabled = false;
            web.agent.moviepilot.enabled = false;
        }
    }

    pub fn load_default() -> Result<Self, ConfigError> {
        let path = env::var("LS_CONFIG_PATH").unwrap_or_else(|_| "config.yaml".to_string());
        Self::load_from_path(path)
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path_ref = path.as_ref();
        let mut cfg = AppConfig::default();

        if path_ref.exists() {
            let content = fs::read(path_ref).map_err(|source| ConfigError::Read {
                path: path_ref.display().to_string(),
                source,
            })?;
            let file_cfg: BootstrapFileConfig =
                serde_yaml::from_slice(&content).map_err(|source| ConfigError::Parse {
                    path: path_ref.display().to_string(),
                    source,
                })?;
            cfg.database = file_cfg.database;
        }

        if let Ok(v) = env::var("LS_DATABASE_URL") {
            if !v.trim().is_empty() {
                cfg.database.url = v;
            }
        }

        if let Ok(v) = env::var("LS_DATABASE_MAX_CONNECTIONS") {
            if let Ok(parsed) = v.parse::<u32>() {
                cfg.database.max_connections = parsed;
            }
        }

        if let Ok(v) = env::var("LS_CORS_ALLOW_ORIGINS") {
            let origins = parse_string_list_env(&v);
            if !origins.is_empty() {
                cfg.server.cors_allow_origins = origins;
            }
        }

        if let Ok(v) = env::var("LS_EDITION")
            && let Some(parsed) = EditionChannel::parse(&v)
        {
            cfg.edition.channel = parsed;
        }

        if let Ok(v) = env::var("LS_EDITION_BILLING_ENABLED")
            && let Some(parsed) = parse_bool_env(&v)
        {
            cfg.edition.overrides.billing = Some(parsed);
        }

        if let Ok(v) = env::var("LS_EDITION_ADVANCED_TRAFFIC_CONTROLS_ENABLED")
            && let Some(parsed) = parse_bool_env(&v)
        {
            cfg.edition.overrides.advanced_traffic_controls = Some(parsed);
        }

        if let Ok(v) = env::var("LS_EDITION_INVITE_REWARDS_ENABLED")
            && let Some(parsed) = parse_bool_env(&v)
        {
            cfg.edition.overrides.invite_rewards = Some(parsed);
        }

        if let Ok(v) = env::var("LS_EDITION_AUDIT_LOG_EXPORT_ENABLED")
            && let Some(parsed) = parse_bool_env(&v)
        {
            cfg.edition.overrides.audit_log_export = Some(parsed);
        }

        if let Ok(v) = env::var("LS_EDITION_REQUEST_AGENT_ENABLED")
            && let Some(parsed) = parse_bool_env(&v)
        {
            cfg.edition.overrides.request_agent = Some(parsed);
        }

        if let Ok(v) = env::var("LS_EDITION_PLAYBACK_ROUTING_ENABLED")
            && let Some(parsed) = parse_bool_env(&v)
        {
            cfg.edition.overrides.playback_routing = Some(parsed);
        }

        if let Ok(v) = env::var("LS_BOOTSTRAP_ADMIN_USER") {
            cfg.auth.bootstrap_admin_user = v;
        }

        if let Ok(v) = env::var("LS_BOOTSTRAP_ADMIN_PASSWORD") {
            cfg.auth.bootstrap_admin_password = v;
        }

        if let Ok(v) = env::var("LS_INVITE_FORCE_ON_REGISTER") {
            if let Some(parsed) = parse_bool_env(&v) {
                cfg.auth.invite.force_on_register = parsed;
            }
        }

        if let Ok(v) = env::var("LS_INVITEE_BONUS_ENABLED") {
            if let Some(parsed) = parse_bool_env(&v) {
                cfg.auth.invite.invitee_bonus_enabled = parsed;
            }
        }

        if let Ok(v) = env::var("LS_INVITEE_BONUS_AMOUNT") {
            if let Ok(parsed) = v.parse::<Decimal>() {
                cfg.auth.invite.invitee_bonus_amount = normalize_money(parsed).max(Decimal::ZERO);
            }
        }

        if let Ok(v) = env::var("LS_INVITER_REBATE_ENABLED") {
            if let Some(parsed) = parse_bool_env(&v) {
                cfg.auth.invite.inviter_rebate_enabled = parsed;
            }
        }

        if let Ok(v) = env::var("LS_INVITER_REBATE_RATE") {
            if let Ok(parsed) = v.parse::<Decimal>() {
                cfg.auth.invite.inviter_rebate_rate = normalize_ratio(parsed);
            }
        }

        if let Ok(v) = env::var("LS_TMDB_API_KEY") {
            if !v.trim().is_empty() {
                cfg.tmdb.api_key = v;
            }
        }

        if let Ok(v) = env::var("LS_SCRAPER_ENABLED") {
            if let Some(parsed) = parse_bool_env(&v) {
                cfg.scraper.enabled = parsed;
            }
        }

        if let Ok(v) = env::var("LS_TVDB_ENABLED")
            && let Some(parsed) = parse_bool_env(&v)
        {
            cfg.scraper.tvdb.enabled = parsed;
        }
        if let Ok(v) = env::var("LS_TVDB_BASE_URL")
            && !v.trim().is_empty()
        {
            cfg.scraper.tvdb.base_url = v;
        }
        if let Ok(v) = env::var("LS_TVDB_API_KEY")
            && !v.trim().is_empty()
        {
            cfg.scraper.tvdb.api_key = v;
        }
        if let Ok(v) = env::var("LS_TVDB_PIN")
            && !v.trim().is_empty()
        {
            cfg.scraper.tvdb.pin = v;
        }
        if let Ok(v) = env::var("LS_TVDB_TIMEOUT_SECONDS")
            && let Ok(parsed) = v.parse::<u64>()
        {
            cfg.scraper.tvdb.timeout_seconds = parsed.max(1);
        }

        if let Ok(v) = env::var("LS_BANGUMI_ENABLED")
            && let Some(parsed) = parse_bool_env(&v)
        {
            cfg.scraper.bangumi.enabled = parsed;
        }
        if let Ok(v) = env::var("LS_BANGUMI_BASE_URL")
            && !v.trim().is_empty()
        {
            cfg.scraper.bangumi.base_url = v;
        }
        if let Ok(v) = env::var("LS_BANGUMI_ACCESS_TOKEN")
            && !v.trim().is_empty()
        {
            cfg.scraper.bangumi.access_token = v;
        }
        if let Ok(v) = env::var("LS_BANGUMI_TIMEOUT_SECONDS")
            && let Ok(parsed) = v.parse::<u64>()
        {
            cfg.scraper.bangumi.timeout_seconds = parsed.max(1);
        }
        if let Ok(v) = env::var("LS_BANGUMI_USER_AGENT")
            && !v.trim().is_empty()
        {
            cfg.scraper.bangumi.user_agent = v;
        }

        if let Ok(v) = env::var("LS_LUMENBACKEND_STREAM_SIGNING_KEY") {
            if !v.trim().is_empty() {
                cfg.storage.lumenbackend_stream_signing_key = v;
            }
        }

        if let Ok(v) = env::var("LS_LUMENBACKEND_ENABLED")
            && let Some(parsed) = parse_bool_env(&v)
        {
            cfg.storage.lumenbackend_enabled = parsed;
        }

        if let Ok(v) = env::var("LS_LUMENBACKEND_NODES") {
            let nodes = parse_string_list_env(&v);
            if !nodes.is_empty() {
                cfg.storage.lumenbackend_nodes = nodes;
            }
        }

        if let Ok(v) = env::var("LS_LUMENBACKEND_ROUTE")
            && !v.trim().is_empty()
        {
            cfg.storage.lumenbackend_route = v;
        }

        if let Ok(v) = env::var("LS_LOCAL_STREAM_ROUTE")
            && !v.trim().is_empty()
        {
            cfg.storage.local_stream_route = v;
        }

        if let Ok(v) = env::var("LS_LUMENBACKEND_STREAM_TOKEN_TTL_SECONDS") {
            if let Ok(parsed) = v.parse::<u64>() {
                cfg.storage.lumenbackend_stream_token_ttl_seconds = parsed.max(1);
            }
        }

        if let Ok(v) = env::var("LS_SCAN_LOCAL_MEDIA_EXTS") {
            let values = parse_string_list_env(&v);
            if !values.is_empty() {
                cfg.scan.local_media_exts = normalize_local_media_exts(values);
            }
        }

        if let Ok(v) = env::var("LS_DEFAULT_USER_MAX_CONCURRENT_STREAMS") {
            if let Ok(parsed) = v.parse::<i32>() {
                cfg.security.default_user_max_concurrent_streams = parsed;
            }
        }

        if let Ok(v) = env::var("LS_DEFAULT_USER_TRAFFIC_QUOTA_BYTES") {
            if let Ok(parsed) = v.parse::<i64>() {
                cfg.security.default_user_traffic_quota_bytes = parsed;
            }
        }

        if let Ok(v) = env::var("LS_DEFAULT_USER_TRAFFIC_WINDOW_DAYS") {
            if let Ok(parsed) = v.parse::<i32>() {
                cfg.security.default_user_traffic_window_days = parsed.max(1);
            }
        }

        if let Ok(v) = env::var("LS_TRUST_X_FORWARDED_FOR") {
            if let Some(parsed) = parse_bool_env(&v) {
                cfg.security.trust_x_forwarded_for = parsed;
            }
        }

        if let Ok(v) = env::var("LS_TRUSTED_PROXIES") {
            let trusted_proxies = parse_string_list_env(&v);
            cfg.security.trusted_proxies = trusted_proxies;
        }

        if let Ok(v) = env::var("LS_BILLING_ENABLED") {
            if let Some(parsed) = parse_bool_env(&v) {
                cfg.billing.enabled = parsed;
            }
        }

        if let Ok(v) = env::var("LS_BILLING_MIN_RECHARGE_AMOUNT") {
            if let Ok(parsed) = v.parse::<Decimal>() {
                cfg.billing.min_recharge_amount = normalize_money(parsed);
            }
        }

        if let Ok(v) = env::var("LS_BILLING_MAX_RECHARGE_AMOUNT") {
            if let Ok(parsed) = v.parse::<Decimal>() {
                cfg.billing.max_recharge_amount = normalize_money(parsed);
            }
        }

        cfg.auth.invite.invitee_bonus_amount =
            cfg.auth.invite.invitee_bonus_amount.max(Decimal::ZERO);
        cfg.auth.invite.inviter_rebate_rate = normalize_ratio(cfg.auth.invite.inviter_rebate_rate);

        if cfg.billing.min_recharge_amount < Decimal::ZERO {
            cfg.billing.min_recharge_amount = Decimal::ZERO;
        }

        if cfg.billing.max_recharge_amount < cfg.billing.min_recharge_amount {
            cfg.billing.max_recharge_amount = cfg.billing.min_recharge_amount;
        }

        if let Ok(v) = env::var("LS_BILLING_ORDER_EXPIRE_MINUTES") {
            if let Ok(parsed) = v.parse::<i64>() {
                cfg.billing.order_expire_minutes = parsed.max(1);
            }
        }

        if let Ok(v) = env::var("LS_BILLING_CHANNELS") {
            let channels = v
                .split(',')
                .map(|part| part.trim().to_ascii_lowercase())
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>();
            if !channels.is_empty() {
                cfg.billing.channels = channels;
            }
        }

        if let Ok(v) = env::var("LS_BILLING_EPAY_GATEWAY_URL") {
            if !v.trim().is_empty() {
                cfg.billing.epay.gateway_url = v;
            }
        }

        if let Ok(v) = env::var("LS_BILLING_EPAY_PID") {
            if !v.trim().is_empty() {
                cfg.billing.epay.pid = v;
            }
        }

        if let Ok(v) = env::var("LS_BILLING_EPAY_KEY") {
            if !v.trim().is_empty() {
                cfg.billing.epay.key = v;
            }
        }

        if let Ok(v) = env::var("LS_BILLING_EPAY_NOTIFY_URL") {
            if !v.trim().is_empty() {
                cfg.billing.epay.notify_url = v;
            }
        }

        if let Ok(v) = env::var("LS_BILLING_EPAY_RETURN_URL") {
            if !v.trim().is_empty() {
                cfg.billing.epay.return_url = v;
            }
        }

        if let Ok(v) = env::var("LS_BILLING_EPAY_SITENAME") {
            if !v.trim().is_empty() {
                cfg.billing.epay.sitename = v;
            }
        }

        if let Ok(v) = env::var("LS_AGENT_ENABLED") {
            if let Some(parsed) = parse_bool_env(&v) {
                cfg.agent.enabled = parsed;
            }
        }

        if let Ok(v) = env::var("LS_AGENT_MISSING_SCAN_ENABLED") {
            if let Some(parsed) = parse_bool_env(&v) {
                cfg.agent.missing_scan_enabled = parsed;
            }
        }

        if let Ok(v) = env::var("LS_AGENT_MOVIEPILOT_ENABLED") {
            if let Some(parsed) = parse_bool_env(&v) {
                cfg.agent.moviepilot.enabled = parsed;
            }
        }

        if let Ok(v) = env::var("LS_AGENT_MOVIEPILOT_BASE_URL") {
            if !v.trim().is_empty() {
                cfg.agent.moviepilot.base_url = v;
            }
        }

        if let Ok(v) = env::var("LS_AGENT_MOVIEPILOT_USERNAME") {
            if !v.trim().is_empty() {
                cfg.agent.moviepilot.username = v;
            }
        }

        if let Ok(v) = env::var("LS_AGENT_MOVIEPILOT_PASSWORD") {
            if !v.trim().is_empty() {
                cfg.agent.moviepilot.password = v;
            }
        }

        // Log config env overrides
        if let Ok(v) = env::var("LS_LOG_LEVEL") {
            if !v.trim().is_empty() {
                cfg.log.level = v.trim().to_ascii_lowercase();
            }
        }

        if let Ok(v) = env::var("LS_LOG_FORMAT") {
            if !v.trim().is_empty() {
                cfg.log.format = v.trim().to_ascii_lowercase();
            }
        }

        if let Ok(v) = env::var("LS_LOG_OUTPUT") {
            if !v.trim().is_empty() {
                cfg.log.output = v.trim().to_ascii_lowercase();
            }
        }

        if let Ok(v) = env::var("LS_LOG_FILE_PATH") {
            if !v.trim().is_empty() {
                cfg.log.file_path = v;
            }
        }

        if let Ok(v) = env::var("LS_LOG_MAX_SIZE_MB") {
            if let Ok(parsed) = v.parse::<u64>() {
                cfg.log.max_size_mb = parsed.max(1);
            }
        }

        if let Ok(v) = env::var("LS_LOG_MAX_FILES") {
            if let Ok(parsed) = v.parse::<u32>() {
                cfg.log.max_files = parsed.max(1);
            }
        }

        cfg.normalize_for_edition();

        Ok(cfg)
    }

    pub fn web_config(&self) -> WebAppConfig {
        let mut web = WebAppConfig {
            server: self.server.clone(),
            auth: self.auth.clone(),
            scan: self.scan.clone(),
            storage: self.storage.clone(),
            tmdb: self.tmdb.clone(),
            scraper: self.normalized_scraper_config(),
            security: self.security.clone(),
            observability: self.observability.clone(),
            log: self.log.clone(),
            jobs: self.jobs.clone(),
            scheduler: self.scheduler.clone(),
            billing: self.billing.clone(),
            agent: self.agent.clone(),
        };
        self.normalize_web_config_for_edition(&mut web);
        web
    }

    pub fn apply_web_config(&mut self, web: &WebAppConfig) {
        let mut normalized = web.clone();
        self.normalize_web_config_for_edition(&mut normalized);

        self.server = normalized.server;
        self.auth = normalized.auth;
        self.scan = normalized.scan;
        self.storage = normalized.storage;
        let scraper_enabled = normalized.scraper.enabled || normalized.tmdb.enabled;
        self.tmdb = normalized.tmdb;
        self.tmdb.enabled = scraper_enabled;
        self.scraper = normalized.scraper;
        self.scraper.enabled = scraper_enabled;
        self.security = normalized.security;
        self.observability = normalized.observability;
        self.log = normalized.log;
        self.jobs = normalized.jobs;
        self.scheduler = normalized.scheduler;
        self.billing = normalized.billing;
        self.agent = normalized.agent;
        self.normalize_for_edition();
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    pub fn normalized_scraper_config(&self) -> ScraperConfig {
        let mut scraper = self.scraper.clone();
        scraper.enabled = scraper.enabled || self.tmdb.enabled;
        if scraper.providers.is_empty() {
            scraper.providers = default_scraper_providers();
        }
        if scraper.default_routes.movie.is_empty() {
            scraper.default_routes.movie = default_scraper_default_routes().movie;
        }
        if scraper.default_routes.series.is_empty() {
            scraper.default_routes.series = default_scraper_default_routes().series;
        }
        if scraper.default_routes.image.is_empty() {
            scraper.default_routes.image = default_scraper_default_routes().image;
        }
        if scraper.default_strategy.trim().is_empty() {
            scraper.default_strategy = default_scraper_default_strategy();
        }
        if scraper.tvdb.base_url.trim().is_empty() {
            scraper.tvdb.base_url = default_scraper_tvdb_base_url();
        }
        if scraper.tvdb.timeout_seconds == 0 {
            scraper.tvdb.timeout_seconds = default_scraper_provider_timeout_seconds();
        }
        if scraper.bangumi.base_url.trim().is_empty() {
            scraper.bangumi.base_url = default_scraper_bangumi_base_url();
        }
        if scraper.bangumi.timeout_seconds == 0 {
            scraper.bangumi.timeout_seconds = default_scraper_provider_timeout_seconds();
        }
        if scraper.bangumi.user_agent.trim().is_empty() {
            scraper.bangumi.user_agent = default_scraper_bangumi_user_agent();
        }
        scraper
    }
}

fn parse_bool_env(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn parse_string_list_env(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn normalize_money(raw: Decimal) -> Decimal {
    raw.round_dp(2)
}

fn normalize_ratio(raw: Decimal) -> Decimal {
    raw.clamp(Decimal::ZERO, Decimal::ONE).round_dp(4)
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_true() -> bool {
    true
}

fn default_port() -> u16 {
    8096
}

fn default_max_upload_body_bytes() -> usize {
    10 * 1024 * 1024 // 10 MiB
}

fn default_database_url() -> String {
    "postgres://postgres:postgres@127.0.0.1:5432/lumenstream".to_string()
}

fn default_max_connections() -> u32 {
    20
}

fn default_token_ttl_hours() -> i64 {
    24 * 30
}

fn default_admin_username() -> String {
    String::new()
}

fn default_admin_password() -> String {
    String::new()
}

fn default_admin_api_key_prefix() -> String {
    "lsadm".to_string()
}

fn default_max_failed_attempts() -> i32 {
    10
}

fn default_risk_window_seconds() -> i64 {
    300
}

fn default_risk_block_seconds() -> i64 {
    900
}

fn default_agent_auto_mode() -> String {
    "automatic".to_string()
}

fn default_agent_missing_scan_cron() -> String {
    "0 */30 * * * *".to_string()
}

fn default_agent_llm_base_url() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_agent_llm_model() -> String {
    "gpt-4o-mini".to_string()
}

fn default_agent_mp_timeout_seconds() -> i64 {
    20
}

fn default_agent_mp_min_seeders() -> i32 {
    5
}

fn default_agent_mp_max_movie_size_gb() -> f64 {
    35.0
}

fn default_agent_mp_max_episode_size_gb() -> f64 {
    5.0
}

fn default_agent_mp_preferred_resource_pix() -> Vec<String> {
    vec!["2160P".to_string(), "4K".to_string(), "1080P".to_string()]
}

fn default_agent_mp_preferred_video_encode() -> Vec<String> {
    vec!["X265".to_string(), "H265".to_string(), "X264".to_string()]
}

fn default_agent_mp_preferred_resource_type() -> Vec<String> {
    vec![
        "WEB-DL".to_string(),
        "BluRay".to_string(),
        "Remux".to_string(),
    ]
}

fn default_agent_mp_preferred_labels() -> Vec<String> {
    vec!["中字".to_string(), "中文".to_string()]
}

fn default_agent_mp_excluded_keywords() -> Vec<String> {
    vec!["CAM".to_string(), "TS".to_string(), "TC".to_string()]
}

fn default_library_name() -> String {
    "Default Library".to_string()
}

fn default_subtitle_exts() -> Vec<String> {
    vec![
        "srt".to_string(),
        "ass".to_string(),
        "ssa".to_string(),
        "sub".to_string(),
        "smi".to_string(),
    ]
}

fn default_local_media_exts() -> Vec<String> {
    [
        "mp4", "mkv", "flv", "avi", "mov", "m4v", "ts", "m2ts", "wmv", "iso",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn default_scan_grace_seconds() -> i64 {
    30
}

fn default_mediainfo_cache_dir() -> String {
    "./cache/mediainfo".to_string()
}

fn default_s3_cache_dir() -> String {
    "./cache/segments".to_string()
}

fn default_s3_cache_ttl_seconds() -> i64 {
    24 * 3600
}

fn default_lumenbackend_route() -> String {
    "v1/streams/gdrive".to_string()
}

fn default_local_stream_route() -> String {
    "v1/streams/local".to_string()
}

fn default_lumenbackend_stream_token_ttl_seconds() -> u64 {
    24 * 3600
}

fn default_tmdb_language() -> String {
    "zh-CN".to_string()
}

fn default_tmdb_timeout_seconds() -> u64 {
    8
}

fn default_tmdb_request_interval_ms() -> u64 {
    200
}

fn default_tmdb_cache_ttl_seconds() -> i64 {
    6 * 3600
}

fn default_tmdb_retry_attempts() -> u32 {
    3
}

fn default_tmdb_retry_backoff_ms() -> u64 {
    300
}

fn default_tmdb_person_image_cache_dir() -> String {
    "./cache".to_string()
}

fn default_scraper_default_strategy() -> String {
    "primary_with_fallback".to_string()
}

fn default_scraper_providers() -> Vec<String> {
    vec![
        "tmdb".to_string(),
        "tvdb".to_string(),
        "bangumi".to_string(),
    ]
}

fn default_scraper_default_routes() -> ScraperDefaultRoutes {
    ScraperDefaultRoutes {
        movie: vec!["tmdb".to_string(), "tvdb".to_string()],
        series: vec!["tmdb".to_string(), "tvdb".to_string()],
        image: vec!["tmdb".to_string(), "tvdb".to_string()],
    }
}

fn default_scraper_tvdb_base_url() -> String {
    "https://api4.thetvdb.com/v4".to_string()
}

fn default_scraper_bangumi_base_url() -> String {
    "https://api.bgm.tv".to_string()
}

fn default_scraper_bangumi_user_agent() -> String {
    "lumenstream/0.1".to_string()
}

fn default_scraper_provider_timeout_seconds() -> u64 {
    15
}

fn default_user_max_concurrent_streams() -> i32 {
    2
}

fn default_user_traffic_quota_bytes() -> i64 {
    500_i64 * 1024 * 1024 * 1024
}

fn default_user_traffic_window_days() -> i32 {
    30
}

fn default_trust_x_forwarded_for() -> bool {
    false
}

fn default_billing_min_recharge_amount() -> Decimal {
    Decimal::new(100, 2)
}

fn default_billing_max_recharge_amount() -> Decimal {
    Decimal::new(200000, 2)
}

fn default_billing_order_expire_minutes() -> i64 {
    30
}

fn default_billing_channels() -> Vec<String> {
    vec!["alipay".to_string(), "wxpay".to_string()]
}

fn default_metrics_enabled() -> bool {
    true
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "pretty".to_string()
}

fn default_log_output() -> String {
    "stdout".to_string()
}

fn default_log_file_path() -> String {
    "./logs/lumenstream.log".to_string()
}

fn default_log_max_size_mb() -> u64 {
    100
}

fn default_log_max_files() -> u32 {
    5
}

fn default_retry_base_seconds() -> i64 {
    15
}

fn default_retry_max_seconds() -> i64 {
    900
}

fn default_scheduler_enabled() -> bool {
    true
}

fn default_scheduler_cleanup_interval_seconds() -> i64 {
    3600
}

fn default_scheduler_job_retry_interval_seconds() -> i64 {
    60
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        sync::{Mutex, OnceLock},
        time::{SystemTime, UNIX_EPOCH},
    };

    struct EnvVarGuard {
        keys: Vec<&'static str>,
    }

    impl EnvVarGuard {
        fn new(keys: &[&'static str]) -> Self {
            Self {
                keys: keys.to_vec(),
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            for key in &self.keys {
                unsafe { env::remove_var(key) };
            }
        }
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn unique_temp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("ls_config_{name}_{nanos}.yaml"))
    }

    #[test]
    fn defaults_include_tmdb_retry_and_cache_settings() {
        let cfg = AppConfig::default();
        assert!(cfg.auth.bootstrap_admin_user.is_empty());
        assert!(cfg.auth.bootstrap_admin_password.is_empty());
        assert_eq!(cfg.tmdb.request_interval_ms, 200);
        assert_eq!(cfg.tmdb.cache_ttl_seconds, 21600);
        assert_eq!(cfg.tmdb.retry_attempts, 3);
        assert_eq!(cfg.tmdb.retry_backoff_ms, 300);
        assert_eq!(cfg.tmdb.person_image_cache_dir, "./cache");
        assert_eq!(cfg.scraper.default_strategy, "primary_with_fallback");
        assert_eq!(
            cfg.scraper.providers,
            vec![
                "tmdb".to_string(),
                "tvdb".to_string(),
                "bangumi".to_string()
            ]
        );
        assert_eq!(
            cfg.scraper.default_routes.movie,
            vec!["tmdb".to_string(), "tvdb".to_string()]
        );
        assert_eq!(
            cfg.scraper.default_routes.series,
            vec!["tmdb".to_string(), "tvdb".to_string()]
        );
        assert_eq!(
            cfg.scraper.default_routes.image,
            vec!["tmdb".to_string(), "tvdb".to_string()]
        );
        assert_eq!(cfg.scraper.tvdb.base_url, "https://api4.thetvdb.com/v4");
        assert_eq!(cfg.scraper.tvdb.timeout_seconds, 15);
        assert_eq!(cfg.scraper.bangumi.base_url, "https://api.bgm.tv");
        assert_eq!(cfg.scraper.bangumi.timeout_seconds, 15);
        assert_eq!(cfg.scraper.bangumi.user_agent, "lumenstream/0.1");
        assert!(!cfg.auth.invite.force_on_register);
        assert!(!cfg.auth.invite.invitee_bonus_enabled);
        assert_eq!(cfg.auth.invite.invitee_bonus_amount, Decimal::ZERO);
        assert!(!cfg.auth.invite.inviter_rebate_enabled);
        assert_eq!(cfg.auth.invite.inviter_rebate_rate, Decimal::ZERO);
        assert_eq!(
            cfg.scan.local_media_exts,
            vec![
                "mp4", "mkv", "flv", "avi", "mov", "m4v", "ts", "m2ts", "wmv", "iso"
            ]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
        );
        assert!(!cfg.storage.lumenbackend_enabled);
        assert!(cfg.storage.lumenbackend_nodes.is_empty());
        assert_eq!(cfg.storage.lumenbackend_route, "v1/streams/gdrive");
        assert_eq!(cfg.storage.local_stream_route, "v1/streams/local");
        assert_eq!(cfg.storage.lumenbackend_stream_token_ttl_seconds, 24 * 3600);
        assert!(cfg.storage.lumenbackend_stream_signing_key.is_empty());
        assert!(!cfg.security.trust_x_forwarded_for);
        assert!(cfg.security.trusted_proxies.is_empty());
        assert_eq!(cfg.security.default_user_max_concurrent_streams, 2);
        assert_eq!(cfg.security.default_user_traffic_window_days, 30);
        assert_eq!(
            cfg.security.default_user_traffic_quota_bytes,
            500_i64 * 1024 * 1024 * 1024
        );
        assert_eq!(cfg.server.max_upload_body_bytes, 10 * 1024 * 1024);
        assert_eq!(cfg.bind_addr(), "0.0.0.0:8096");
        assert_eq!(cfg.edition.channel, EditionChannel::Ce);
        assert!(!cfg.billing.enabled);
        assert_eq!(cfg.billing.min_recharge_amount, Decimal::new(100, 2));
        assert_eq!(cfg.billing.max_recharge_amount, Decimal::new(200000, 2));
        assert_eq!(cfg.billing.order_expire_minutes, 30);
        assert_eq!(
            cfg.billing.channels,
            vec!["alipay".to_string(), "wxpay".to_string()]
        );
    }

    #[test]
    fn scan_config_accepts_legacy_default_library_path() {
        let scan: ScanConfig = serde_json::from_value(serde_json::json!({
            "default_library_name": "Default Library",
            "default_library_path": "/media/default"
        }))
        .expect("parse scan config");

        assert_eq!(
            scan.default_library_paths,
            vec!["/media/default".to_string()]
        );
    }

    #[test]
    fn scan_config_merges_and_dedups_library_paths() {
        let scan: ScanConfig = serde_json::from_value(serde_json::json!({
            "default_library_paths": ["/media/a", " /media/b/ ", "/media/a"],
            "default_library_path": "/media/B"
        }))
        .expect("parse scan config");

        assert_eq!(
            scan.default_library_paths,
            vec!["/media/a".to_string(), "/media/b".to_string()]
        );
    }

    #[test]
    fn load_from_path_only_reads_database_section() {
        let _lock = env_lock().lock().expect("env lock");
        let _guard = EnvVarGuard::new(&["LS_BOOTSTRAP_ADMIN_USER", "LS_BOOTSTRAP_ADMIN_PASSWORD"]);

        let path = unique_temp_path("bootstrap_db_only");
        fs::write(
            &path,
            r#"
server:
  host: "127.0.0.1"
  port: 9000
database:
  url: "postgres://cfg_user:cfg_pwd@127.0.0.1:5432/custom"
  max_connections: 42
auth:
  bootstrap_admin_user: "from-yaml"
  bootstrap_admin_password: "from-yaml"
"#,
        )
        .expect("write config");

        let cfg = AppConfig::load_from_path(&path).expect("load config");
        fs::remove_file(&path).ok();

        assert_eq!(
            cfg.database.url,
            "postgres://cfg_user:cfg_pwd@127.0.0.1:5432/custom"
        );
        assert_eq!(cfg.database.max_connections, 42);

        // Non-database settings are now managed by Web settings during runtime.
        assert_eq!(cfg.server.host, "0.0.0.0");
        assert_eq!(cfg.server.port, 8096);
        assert!(cfg.auth.bootstrap_admin_user.is_empty());
        assert!(cfg.auth.bootstrap_admin_password.is_empty());
    }

    #[test]
    fn env_overrides_bootstrap_admin_credentials() {
        let _lock = env_lock().lock().expect("env lock");
        let _guard = EnvVarGuard::new(&["LS_BOOTSTRAP_ADMIN_USER", "LS_BOOTSTRAP_ADMIN_PASSWORD"]);

        unsafe { env::set_var("LS_BOOTSTRAP_ADMIN_USER", "secure-admin") };
        unsafe { env::set_var("LS_BOOTSTRAP_ADMIN_PASSWORD", "change-me-now") };

        let path = unique_temp_path("bootstrap_env_only");
        let cfg =
            AppConfig::load_from_path(&path).expect("load config with bootstrap env overrides");

        assert_eq!(cfg.auth.bootstrap_admin_user, "secure-admin");
        assert_eq!(cfg.auth.bootstrap_admin_password, "change-me-now");
    }

    #[test]
    fn validate_bootstrap_credentials_rejects_empty_fields() {
        let auth = AuthConfig::default();
        assert_eq!(
            auth.validate_bootstrap_credentials(),
            Err("auth.bootstrap_admin_user is required")
        );

        let auth = AuthConfig {
            bootstrap_admin_user: "admin".to_string(),
            bootstrap_admin_password: " ".to_string(),
            ..AuthConfig::default()
        };
        assert_eq!(
            auth.validate_bootstrap_credentials(),
            Err("auth.bootstrap_admin_password is required")
        );
    }

    #[test]
    fn validate_bootstrap_credentials_rejects_legacy_default_pair() {
        let auth = AuthConfig {
            bootstrap_admin_user: "admin".to_string(),
            bootstrap_admin_password: "admin123".to_string(),
            ..AuthConfig::default()
        };

        assert_eq!(
            auth.validate_bootstrap_credentials(),
            Err("auth.bootstrap_admin_password cannot use legacy default credentials")
        );
    }

    #[test]
    fn validate_bootstrap_credentials_accepts_explicit_secure_values() {
        let auth = AuthConfig {
            bootstrap_admin_user: "root-admin".to_string(),
            bootstrap_admin_password: "S3cure#Pass".to_string(),
            ..AuthConfig::default()
        };

        assert_eq!(auth.validate_bootstrap_credentials(), Ok(()));
    }

    #[test]
    fn env_overrides_bootstrap_database_and_stream_policy_defaults() {
        let _lock = env_lock().lock().expect("env lock");
        let _guard = EnvVarGuard::new(&[
            "LS_DATABASE_URL",
            "LS_DATABASE_MAX_CONNECTIONS",
            "LS_TMDB_API_KEY",
            "LS_SCRAPER_ENABLED",
            "LS_TVDB_ENABLED",
            "LS_TVDB_BASE_URL",
            "LS_TVDB_API_KEY",
            "LS_TVDB_PIN",
            "LS_TVDB_TIMEOUT_SECONDS",
            "LS_BANGUMI_ENABLED",
            "LS_BANGUMI_BASE_URL",
            "LS_BANGUMI_ACCESS_TOKEN",
            "LS_BANGUMI_TIMEOUT_SECONDS",
            "LS_BANGUMI_USER_AGENT",
            "LS_INVITE_FORCE_ON_REGISTER",
            "LS_INVITEE_BONUS_ENABLED",
            "LS_INVITEE_BONUS_AMOUNT",
            "LS_INVITER_REBATE_ENABLED",
            "LS_INVITER_REBATE_RATE",
            "LS_LUMENBACKEND_ENABLED",
            "LS_LUMENBACKEND_NODES",
            "LS_LUMENBACKEND_ROUTE",
            "LS_LOCAL_STREAM_ROUTE",
            "LS_LUMENBACKEND_STREAM_SIGNING_KEY",
            "LS_LUMENBACKEND_STREAM_TOKEN_TTL_SECONDS",
            "LS_SCAN_LOCAL_MEDIA_EXTS",
            "LS_DEFAULT_USER_MAX_CONCURRENT_STREAMS",
            "LS_DEFAULT_USER_TRAFFIC_QUOTA_BYTES",
            "LS_DEFAULT_USER_TRAFFIC_WINDOW_DAYS",
            "LS_TRUST_X_FORWARDED_FOR",
            "LS_TRUSTED_PROXIES",
            "LS_EDITION",
            "LS_EDITION_BILLING_ENABLED",
            "LS_EDITION_ADVANCED_TRAFFIC_CONTROLS_ENABLED",
            "LS_EDITION_INVITE_REWARDS_ENABLED",
            "LS_EDITION_AUDIT_LOG_EXPORT_ENABLED",
            "LS_EDITION_REQUEST_AGENT_ENABLED",
            "LS_EDITION_PLAYBACK_ROUTING_ENABLED",
            "LS_BILLING_ENABLED",
            "LS_BILLING_MIN_RECHARGE_AMOUNT",
            "LS_BILLING_MAX_RECHARGE_AMOUNT",
            "LS_BILLING_ORDER_EXPIRE_MINUTES",
            "LS_BILLING_CHANNELS",
            "LS_BILLING_EPAY_GATEWAY_URL",
            "LS_BILLING_EPAY_PID",
            "LS_BILLING_EPAY_KEY",
            "LS_BILLING_EPAY_NOTIFY_URL",
            "LS_BILLING_EPAY_RETURN_URL",
            "LS_BILLING_EPAY_SITENAME",
        ]);

        unsafe {
            env::set_var(
                "LS_DATABASE_URL",
                "postgres://env_user:env_pwd@127.0.0.1:5432/env",
            )
        };
        unsafe { env::set_var("LS_DATABASE_MAX_CONNECTIONS", "77") };
        unsafe { env::set_var("LS_TMDB_API_KEY", "tmdb-env-key") };
        unsafe { env::set_var("LS_SCRAPER_ENABLED", "true") };
        unsafe { env::set_var("LS_TVDB_ENABLED", "true") };
        unsafe { env::set_var("LS_TVDB_BASE_URL", "https://tvdb.example.com/v4") };
        unsafe { env::set_var("LS_TVDB_API_KEY", "tvdb-key") };
        unsafe { env::set_var("LS_TVDB_PIN", "tvdb-pin") };
        unsafe { env::set_var("LS_TVDB_TIMEOUT_SECONDS", "18") };
        unsafe { env::set_var("LS_BANGUMI_ENABLED", "true") };
        unsafe { env::set_var("LS_BANGUMI_BASE_URL", "https://bangumi.example.com") };
        unsafe { env::set_var("LS_BANGUMI_ACCESS_TOKEN", "bgm-token") };
        unsafe { env::set_var("LS_BANGUMI_TIMEOUT_SECONDS", "25") };
        unsafe { env::set_var("LS_BANGUMI_USER_AGENT", "custom-agent/1.0") };
        unsafe { env::set_var("LS_INVITE_FORCE_ON_REGISTER", "true") };
        unsafe { env::set_var("LS_INVITEE_BONUS_ENABLED", "yes") };
        unsafe { env::set_var("LS_INVITEE_BONUS_AMOUNT", "8.88") };
        unsafe { env::set_var("LS_INVITER_REBATE_ENABLED", "1") };
        unsafe { env::set_var("LS_INVITER_REBATE_RATE", "0.13579") };
        unsafe { env::set_var("LS_LUMENBACKEND_ENABLED", "true") };
        unsafe {
            env::set_var(
                "LS_LUMENBACKEND_NODES",
                "http://stream-gateway:8096, https://stream.example.com",
            )
        };
        unsafe { env::set_var("LS_LUMENBACKEND_ROUTE", "v1/streams/gdrive") };
        unsafe { env::set_var("LS_LOCAL_STREAM_ROUTE", "v1/streams/local") };
        unsafe {
            env::set_var(
                "LS_LUMENBACKEND_STREAM_SIGNING_KEY",
                "ls-lumenbackend-secret",
            )
        };
        unsafe { env::set_var("LS_LUMENBACKEND_STREAM_TOKEN_TTL_SECONDS", "45") };
        unsafe { env::set_var("LS_SCAN_LOCAL_MEDIA_EXTS", "mp4, mkv, .iso, strm") };
        unsafe { env::set_var("LS_DEFAULT_USER_MAX_CONCURRENT_STREAMS", "3") };
        unsafe { env::set_var("LS_DEFAULT_USER_TRAFFIC_QUOTA_BYTES", "123456") };
        unsafe { env::set_var("LS_DEFAULT_USER_TRAFFIC_WINDOW_DAYS", "15") };
        unsafe { env::set_var("LS_TRUST_X_FORWARDED_FOR", "true") };
        unsafe { env::set_var("LS_TRUSTED_PROXIES", "10.0.0.0/8, 203.0.113.10") };
        unsafe { env::set_var("LS_EDITION", "ee") };
        unsafe { env::set_var("LS_EDITION_BILLING_ENABLED", "true") };
        unsafe { env::set_var("LS_EDITION_ADVANCED_TRAFFIC_CONTROLS_ENABLED", "true") };
        unsafe { env::set_var("LS_EDITION_INVITE_REWARDS_ENABLED", "true") };
        unsafe { env::set_var("LS_EDITION_AUDIT_LOG_EXPORT_ENABLED", "true") };
        unsafe { env::set_var("LS_BILLING_ENABLED", "true") };
        unsafe { env::set_var("LS_BILLING_MIN_RECHARGE_AMOUNT", "9.90") };
        unsafe { env::set_var("LS_BILLING_MAX_RECHARGE_AMOUNT", "999.50") };
        unsafe { env::set_var("LS_BILLING_ORDER_EXPIRE_MINUTES", "45") };
        unsafe { env::set_var("LS_BILLING_CHANNELS", "alipay, wxpay, qqpay") };
        unsafe { env::set_var("LS_BILLING_EPAY_GATEWAY_URL", "https://epay.example.com") };
        unsafe { env::set_var("LS_BILLING_EPAY_PID", "10001") };
        unsafe { env::set_var("LS_BILLING_EPAY_KEY", "epay-secret") };
        unsafe {
            env::set_var(
                "LS_BILLING_EPAY_NOTIFY_URL",
                "https://lumenstream.example.com/billing/epay/notify",
            )
        };
        unsafe {
            env::set_var(
                "LS_BILLING_EPAY_RETURN_URL",
                "https://lumenstream.example.com/billing/epay/return",
            )
        };
        unsafe { env::set_var("LS_BILLING_EPAY_SITENAME", "LumenStream") };

        let path = unique_temp_path("missing");
        let cfg = AppConfig::load_from_path(&path).expect("load config with env overrides");

        assert_eq!(
            cfg.database.url,
            "postgres://env_user:env_pwd@127.0.0.1:5432/env"
        );
        assert_eq!(cfg.database.max_connections, 77);
        assert_eq!(cfg.tmdb.api_key, "tmdb-env-key");
        assert!(cfg.scraper.enabled);
        assert!(cfg.scraper.tvdb.enabled);
        assert_eq!(cfg.scraper.tvdb.base_url, "https://tvdb.example.com/v4");
        assert_eq!(cfg.scraper.tvdb.api_key, "tvdb-key");
        assert_eq!(cfg.scraper.tvdb.pin, "tvdb-pin");
        assert_eq!(cfg.scraper.tvdb.timeout_seconds, 18);
        assert!(cfg.scraper.bangumi.enabled);
        assert_eq!(cfg.scraper.bangumi.base_url, "https://bangumi.example.com");
        assert_eq!(cfg.scraper.bangumi.access_token, "bgm-token");
        assert_eq!(cfg.scraper.bangumi.timeout_seconds, 25);
        assert_eq!(cfg.scraper.bangumi.user_agent, "custom-agent/1.0");
        assert!(cfg.auth.invite.force_on_register);
        assert!(cfg.auth.invite.invitee_bonus_enabled);
        assert_eq!(cfg.auth.invite.invitee_bonus_amount, Decimal::new(888, 2));
        assert!(cfg.auth.invite.inviter_rebate_enabled);
        assert_eq!(cfg.auth.invite.inviter_rebate_rate, Decimal::new(1358, 4));

        assert!(cfg.storage.lumenbackend_enabled);
        assert_eq!(
            cfg.storage.lumenbackend_nodes,
            vec![
                "http://stream-gateway:8096".to_string(),
                "https://stream.example.com".to_string()
            ]
        );
        assert_eq!(cfg.storage.lumenbackend_route, "v1/streams/gdrive");
        assert_eq!(cfg.storage.local_stream_route, "v1/streams/local");
        assert_eq!(
            cfg.storage.lumenbackend_stream_signing_key,
            "ls-lumenbackend-secret"
        );
        assert_eq!(cfg.storage.lumenbackend_stream_token_ttl_seconds, 45);
        assert_eq!(
            cfg.scan.local_media_exts,
            vec!["mp4".to_string(), "mkv".to_string(), "iso".to_string()]
        );
        assert_eq!(cfg.security.default_user_max_concurrent_streams, 3);
        assert_eq!(cfg.security.default_user_traffic_quota_bytes, 123456);
        assert_eq!(cfg.security.default_user_traffic_window_days, 15);
        assert!(cfg.security.trust_x_forwarded_for);
        assert_eq!(cfg.edition.channel, EditionChannel::Ee);
        assert_eq!(
            cfg.security.trusted_proxies,
            vec!["10.0.0.0/8".to_string(), "203.0.113.10".to_string()]
        );
        assert!(cfg.billing.enabled);
        assert_eq!(cfg.billing.min_recharge_amount, Decimal::new(990, 2));
        assert_eq!(cfg.billing.max_recharge_amount, Decimal::new(99950, 2));
        assert_eq!(cfg.billing.order_expire_minutes, 45);
        assert_eq!(
            cfg.billing.channels,
            vec![
                "alipay".to_string(),
                "wxpay".to_string(),
                "qqpay".to_string()
            ]
        );
        assert_eq!(cfg.billing.epay.gateway_url, "https://epay.example.com");
        assert_eq!(cfg.billing.epay.pid, "10001");
        assert_eq!(cfg.billing.epay.key, "epay-secret");
        assert_eq!(
            cfg.billing.epay.notify_url,
            "https://lumenstream.example.com/billing/epay/notify"
        );
        assert_eq!(
            cfg.billing.epay.return_url,
            "https://lumenstream.example.com/billing/epay/return"
        );
        assert_eq!(cfg.billing.epay.sitename, "LumenStream");
    }

    #[test]
    fn apply_web_config_updates_only_non_database_sections() {
        let mut cfg = AppConfig::default();
        cfg.database.url = "postgres://keep/me".to_string();
        cfg.database.max_connections = 11;
        cfg.edition.channel = EditionChannel::Ee;
        cfg.edition.overrides.billing = Some(true);
        cfg.edition.overrides.invite_rewards = Some(true);

        let mut web = cfg.web_config();
        web.server.host = "127.0.0.1".to_string();
        web.server.port = 18096;
        web.auth.bootstrap_admin_user = "root".to_string();
        web.auth.invite.force_on_register = true;
        web.auth.invite.invitee_bonus_enabled = true;
        web.auth.invite.invitee_bonus_amount = Decimal::new(500, 2);
        web.auth.invite.inviter_rebate_enabled = true;
        web.auth.invite.inviter_rebate_rate = Decimal::new(1000, 4);
        web.tmdb.api_key = "tmdb-key".to_string();
        web.scraper.enabled = true;
        web.scraper.default_strategy = "primary_with_fallback".to_string();
        web.scraper.tvdb.enabled = true;
        web.scraper.tvdb.api_key = "tvdb-key".to_string();
        web.scraper.bangumi.enabled = true;
        web.scraper.bangumi.access_token = "bgm-token".to_string();
        web.scan.local_media_exts = vec!["mp4".to_string(), "mkv".to_string()];
        web.storage.lumenbackend_enabled = true;
        web.storage.lumenbackend_route = "cdn".to_string();
        web.storage.local_stream_route = "v1/streams/local".to_string();
        web.storage.lumenbackend_nodes = vec!["https://lumenbackend-us.example.com".to_string()];
        web.billing.enabled = true;
        web.billing.min_recharge_amount = Decimal::new(500, 2);
        web.billing.max_recharge_amount = Decimal::new(500000, 2);
        web.billing.channels = vec!["alipay".to_string()];
        web.billing.epay.key = "billing-secret".to_string();

        cfg.apply_web_config(&web);

        assert_eq!(cfg.server.host, "127.0.0.1");
        assert_eq!(cfg.server.port, 18096);
        assert_eq!(cfg.auth.bootstrap_admin_user, "root");
        assert!(cfg.auth.invite.force_on_register);
        assert!(cfg.auth.invite.invitee_bonus_enabled);
        assert_eq!(cfg.auth.invite.invitee_bonus_amount, Decimal::new(500, 2));
        assert!(cfg.auth.invite.inviter_rebate_enabled);
        assert_eq!(cfg.auth.invite.inviter_rebate_rate, Decimal::new(1000, 4));
        assert_eq!(cfg.tmdb.api_key, "tmdb-key");
        assert!(cfg.scraper.enabled);
        assert_eq!(cfg.scraper.default_strategy, "primary_with_fallback");
        assert!(cfg.scraper.tvdb.enabled);
        assert_eq!(cfg.scraper.tvdb.api_key, "tvdb-key");
        assert!(cfg.scraper.bangumi.enabled);
        assert_eq!(cfg.scraper.bangumi.access_token, "bgm-token");
        assert_eq!(
            cfg.scan.local_media_exts,
            vec!["mp4".to_string(), "mkv".to_string()]
        );
        assert!(cfg.storage.lumenbackend_enabled);
        assert_eq!(cfg.storage.lumenbackend_route, "cdn");
        assert_eq!(cfg.storage.local_stream_route, "v1/streams/local");
        assert_eq!(
            cfg.storage.lumenbackend_nodes,
            vec!["https://lumenbackend-us.example.com".to_string()]
        );
        assert!(cfg.billing.enabled);
        assert_eq!(cfg.billing.min_recharge_amount, Decimal::new(500, 2));
        assert_eq!(cfg.billing.max_recharge_amount, Decimal::new(500000, 2));
        assert_eq!(cfg.billing.channels, vec!["alipay".to_string()]);
        assert_eq!(cfg.billing.epay.key, "billing-secret");

        assert_eq!(cfg.database.url, "postgres://keep/me");
        assert_eq!(cfg.database.max_connections, 11);
    }

    #[test]
    fn ce_edition_masks_commercial_runtime_settings() {
        let mut cfg = AppConfig::default();

        let mut web = cfg.web_config();
        web.billing.enabled = true;
        web.auth.invite.invitee_bonus_enabled = true;
        web.auth.invite.invitee_bonus_amount = Decimal::new(500, 2);
        web.auth.invite.inviter_rebate_enabled = true;
        web.auth.invite.inviter_rebate_rate = Decimal::new(1000, 4);

        cfg.apply_web_config(&web);

        assert!(!cfg.billing.enabled);
        assert!(!cfg.auth.invite.invitee_bonus_enabled);
        assert_eq!(cfg.auth.invite.invitee_bonus_amount, Decimal::ZERO);
        assert!(!cfg.auth.invite.inviter_rebate_enabled);
        assert_eq!(cfg.auth.invite.inviter_rebate_rate, Decimal::ZERO);
    }

    #[test]
    fn scheduler_config_defaults() {
        let cfg = SchedulerConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.cleanup_interval_seconds, 3600);
        assert_eq!(cfg.job_retry_interval_seconds, 60);
    }

    #[test]
    fn scheduler_config_in_app_config() {
        let cfg = AppConfig::default();
        assert!(cfg.scheduler.enabled);
        assert_eq!(cfg.scheduler.cleanup_interval_seconds, 3600);
        assert_eq!(cfg.scheduler.job_retry_interval_seconds, 60);
    }

    #[test]
    fn scheduler_config_in_web_config() {
        let cfg = AppConfig::default();
        let web = cfg.web_config();
        assert!(web.scheduler.enabled);
        assert_eq!(web.scheduler.cleanup_interval_seconds, 3600);
        assert_eq!(web.scheduler.job_retry_interval_seconds, 60);
    }

    #[test]
    fn apply_web_config_updates_scheduler() {
        let mut cfg = AppConfig::default();
        let mut web = cfg.web_config();
        web.scheduler.enabled = false;
        web.scheduler.cleanup_interval_seconds = 7200;
        web.scheduler.job_retry_interval_seconds = 120;

        cfg.apply_web_config(&web);

        assert!(!cfg.scheduler.enabled);
        assert_eq!(cfg.scheduler.cleanup_interval_seconds, 7200);
        assert_eq!(cfg.scheduler.job_retry_interval_seconds, 120);
    }

    #[test]
    fn scan_config_normalizes_local_media_exts() {
        let parsed: ScanConfig = serde_yaml::from_str(
            r#"
default_library_name: Demo
local_media_exts:
  - ".MP4"
  - "mkv"
  - "strm"
  - "mp4"
"#,
        )
        .expect("scan config");

        assert_eq!(
            parsed.local_media_exts,
            vec!["mp4".to_string(), "mkv".to_string()]
        );
    }

    #[test]
    fn log_config_defaults() {
        let cfg = LogConfig::default();
        assert_eq!(cfg.level, "info");
        assert_eq!(cfg.format, "pretty");
        assert_eq!(cfg.output, "stdout");
        assert_eq!(cfg.file_path, "./logs/lumenstream.log");
        assert_eq!(cfg.max_size_mb, 100);
        assert_eq!(cfg.max_files, 5);
    }

    #[test]
    fn log_config_in_app_config() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.log.level, "info");
        assert_eq!(cfg.log.format, "pretty");
        assert_eq!(cfg.log.output, "stdout");
        assert_eq!(cfg.log.file_path, "./logs/lumenstream.log");
        assert_eq!(cfg.log.max_size_mb, 100);
        assert_eq!(cfg.log.max_files, 5);
    }

    #[test]
    fn log_config_in_web_config() {
        let cfg = AppConfig::default();
        let web = cfg.web_config();
        assert_eq!(web.log.level, "info");
        assert_eq!(web.log.format, "pretty");
        assert_eq!(web.log.output, "stdout");
    }

    #[test]
    fn apply_web_config_updates_log() {
        let mut cfg = AppConfig::default();
        let mut web = cfg.web_config();
        web.log.level = "debug".to_string();
        web.log.format = "json".to_string();
        web.log.output = "both".to_string();
        web.log.file_path = "/var/log/lumenstream.log".to_string();
        web.log.max_size_mb = 200;
        web.log.max_files = 10;

        cfg.apply_web_config(&web);

        assert_eq!(cfg.log.level, "debug");
        assert_eq!(cfg.log.format, "json");
        assert_eq!(cfg.log.output, "both");
        assert_eq!(cfg.log.file_path, "/var/log/lumenstream.log");
        assert_eq!(cfg.log.max_size_mb, 200);
        assert_eq!(cfg.log.max_files, 10);
    }

    #[test]
    fn log_config_env_overrides() {
        let _lock = env_lock().lock().expect("env lock");
        let _guard = EnvVarGuard::new(&[
            "LS_LOG_LEVEL",
            "LS_LOG_FORMAT",
            "LS_LOG_OUTPUT",
            "LS_LOG_FILE_PATH",
            "LS_LOG_MAX_SIZE_MB",
            "LS_LOG_MAX_FILES",
        ]);

        unsafe { env::set_var("LS_LOG_LEVEL", "DEBUG") };
        unsafe { env::set_var("LS_LOG_FORMAT", "JSON") };
        unsafe { env::set_var("LS_LOG_OUTPUT", "BOTH") };
        unsafe { env::set_var("LS_LOG_FILE_PATH", "/var/log/custom.log") };
        unsafe { env::set_var("LS_LOG_MAX_SIZE_MB", "250") };
        unsafe { env::set_var("LS_LOG_MAX_FILES", "7") };

        let path = unique_temp_path("log_env");
        let cfg = AppConfig::load_from_path(&path).expect("load config");

        assert_eq!(cfg.log.level, "debug");
        assert_eq!(cfg.log.format, "json");
        assert_eq!(cfg.log.output, "both");
        assert_eq!(cfg.log.file_path, "/var/log/custom.log");
        assert_eq!(cfg.log.max_size_mb, 250);
        assert_eq!(cfg.log.max_files, 7);
    }

    #[test]
    fn env_overrides_cors_allow_origins() {
        let _lock = env_lock().lock().expect("env lock");
        let _guard = EnvVarGuard::new(&["LS_CORS_ALLOW_ORIGINS"]);

        unsafe {
            env::set_var(
                "LS_CORS_ALLOW_ORIGINS",
                "https://a.example.com, https://b.example.com",
            )
        };

        let path = unique_temp_path("cors_env");
        let cfg = AppConfig::load_from_path(&path).expect("load config");

        assert_eq!(
            cfg.server.cors_allow_origins,
            vec!["https://a.example.com", "https://b.example.com"]
        );
    }
}
