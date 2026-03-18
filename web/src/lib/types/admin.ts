import type { Subscription, Wallet } from "@/lib/types/billing";
import type { StreamPolicy, TrafficUsage } from "@/lib/types/edition-commercial";
import type { AgentSettings } from "@/lib/types/requests";

export type {
  AdminInviteSettings,
  AdminUserStreamPolicyPayload,
  InviteRelation,
  InviteRebateRecord,
  InviteSummary,
  MyTrafficUsageMediaSummary,
  StreamPolicy,
  TopTrafficUser,
  TrafficUsage,
  TrafficUsageDaily,
  TrafficUsageMediaItem,
} from "@/lib/types/edition-commercial";

export type UserRole = "Admin" | "Viewer";
export type LibraryType = "Movie" | "Series" | "Mixed";

export interface AdminLibrary {
  id: string;
  name: string;
  root_path: string;
  paths: string[];
  library_type?: LibraryType;
  enabled: boolean;
  scraper_policy?: Record<string, unknown>;
  created_at?: string;
}

export interface AdminLibraryStatusItem {
  id: string;
  name: string;
  root_path: string;
  paths: string[];
  library_type?: LibraryType;
  enabled: boolean;
  scraper_policy?: Record<string, unknown>;
  item_count: number;
  last_item_updated_at?: string | null;
}

export interface AdminLibraryStatusResponse {
  total: number;
  enabled: number;
  items: AdminLibraryStatusItem[];
}

export interface AdminUser {
  Id: string;
  Name: string;
  HasPassword: boolean;
  ServerId: string;
  Policy: {
    IsAdministrator: boolean;
    IsDisabled: boolean;
    Role?: string;
  };
}

export interface AdminTaskDefinition {
  task_key: string;
  display_name: string;
  enabled: boolean;
  cron_expr: string;
  default_payload: Record<string, unknown>;
  max_attempts: number;
  created_at: string;
  updated_at: string;
}

export interface AdminTaskRun {
  id: string;
  kind: string;
  status: string;
  payload: Record<string, unknown>;
  progress?: TaskRunProgress | null;
  result?: Record<string, unknown> | null;
  error?: string | null;
  attempts: number;
  max_attempts: number;
  next_retry_at?: string | null;
  dead_letter: boolean;
  trigger_type?: string | null;
  scheduled_for?: string | null;
  created_at: string;
  started_at?: string | null;
  finished_at?: string | null;
}

export interface TaskRunProgress {
  phase: string;
  total: number;
  completed: number;
  percent: number;
  message?: string;
  updated_at?: string;
  detail?: Record<string, unknown>;
}

export type AdminJob = AdminTaskRun;

export interface PlaybackSession {
  id: string;
  play_session_id: string;
  user_id: string;
  user_name: string;
  media_item_id?: string | null;
  media_item_name?: string | null;
  device_name?: string | null;
  client_name?: string | null;
  play_method?: string | null;
  position_ticks: number;
  is_active: boolean;
  last_heartbeat_at: string;
  updated_at: string;
}

export interface AuthSession {
  id: string;
  user_id: string;
  user_name: string;
  client?: string | null;
  device_name?: string | null;
  device_id?: string | null;
  remote_addr?: string | null;
  is_active: boolean;
  created_at: string;
  last_seen_at: string;
}

export interface AdminApiKey {
  id: string;
  name: string;
  created_at: string;
  last_used_at?: string | null;
}

export interface AdminCreatedApiKey {
  id: string;
  name: string;
  api_key: string;
  created_at: string;
}

export interface AuditLogEntry {
  id: string;
  actor_user_id?: string | null;
  actor_username?: string | null;
  action: string;
  target_type: string;
  target_id?: string | null;
  detail: Record<string, unknown>;
  created_at: string;
}

export interface AdminSystemSummary {
  generated_at_utc: string;
  server_id: string;
  transcoding_enabled: boolean;
  libraries_total: number;
  libraries_enabled: number;
  media_items_total: number;
  users_total: number;
  users_disabled: number;
  active_playback_sessions: number;
  active_auth_sessions: number;
  jobs_by_status: Record<string, number>;
  infra_metrics: Record<string, unknown>;
}

export interface AdminSystemFlags {
  strm_only_streaming: boolean;
  transcoding_enabled: boolean;
  scraper_enabled: boolean;
  tmdb_enabled: boolean;
  lumenbackend_enabled: boolean;
  prefer_segment_gateway: boolean;
  metrics_enabled: boolean;
}

export interface AdminSystemCapabilities {
  edition: string;
  strm_only_streaming: boolean;
  transcoding_enabled: boolean;
  billing_enabled: boolean;
  advanced_traffic_controls_enabled: boolean;
  invite_rewards_enabled: boolean;
  audit_log_export_enabled: boolean;
  request_agent_enabled: boolean;
  playback_routing_enabled: boolean;
  supported_stream_features: string[];
}

export interface PlaybackDomain {
  id: string;
  name: string;
  base_url: string;
  enabled: boolean;
  priority: number;
  is_default: boolean;
  lumenbackend_node_id: string | null;
  traffic_multiplier: number;
  created_at?: string;
  updated_at?: string;
}

export interface MePlaybackDomainsResponse {
  selected_domain_id: string | null;
  default_domain_id: string | null;
  available: PlaybackDomain[];
}

export interface LumenBackendNode {
  node_id: string;
  name: string | null;
  enabled: boolean;
  last_seen_at: string | null;
  last_version: string | null;
  last_status: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface LumenBackendNodeRuntimeConfig {
  node_id: string;
  version: number;
  config: Record<string, unknown>;
}

export type LumenBackendRuntimeFieldType =
  | "string"
  | "number"
  | "boolean"
  | "select"
  | "password"
  | "textarea";

export interface LumenBackendRuntimeSchemaFieldOption {
  label?: string;
  value: string;
}

export interface LumenBackendRuntimeSchemaFieldValidators {
  min?: number;
  max?: number;
  min_length?: number;
  max_length?: number;
  pattern?: string;
  url?: boolean;
}

export interface LumenBackendRuntimeSchemaFieldDependsOn {
  key: string;
  equals?: string | number | boolean;
  not_equals?: string | number | boolean;
  in?: Array<string | number | boolean>;
}

export interface LumenBackendRuntimeSchemaField {
  key: string;
  label: string;
  type: LumenBackendRuntimeFieldType;
  required?: boolean;
  default?: unknown;
  placeholder?: string;
  help?: string;
  options?: LumenBackendRuntimeSchemaFieldOption[];
  validators?: LumenBackendRuntimeSchemaFieldValidators;
  depends_on?: LumenBackendRuntimeSchemaFieldDependsOn;
}

export interface LumenBackendRuntimeSchemaSection {
  id: string;
  title: string;
  description?: string;
  fields: LumenBackendRuntimeSchemaField[];
}

export interface LumenBackendRuntimeSchemaDefinition {
  sections: LumenBackendRuntimeSchemaSection[];
}

export interface LumenBackendNodeRuntimeSchema {
  node_id: string;
  schema_version: string;
  schema_hash: string | null;
  schema: LumenBackendRuntimeSchemaDefinition;
  updated_at: string;
}

export interface TmdbCacheStats {
  total_entries: number;
  entries_with_result: number;
  expired_entries: number;
  total_hits: number;
}

export interface TmdbFailureEntry {
  id: string;
  media_item_id: string | null;
  item_name: string;
  item_type: string;
  attempts: number;
  error: string;
  created_at: string;
}

export interface TmdbConfig {
  enabled: boolean;
  api_key: string;
  language: string;
  timeout_seconds: number;
  request_interval_ms: number;
  cache_ttl_seconds: number;
  retry_attempts: number;
  retry_backoff_ms: number;
}

export interface ScraperConfig {
  enabled: boolean;
  default_strategy: string;
  providers: string[];
  default_routes: {
    movie: string[];
    series: string[];
    image: string[];
  };
  tvdb: {
    enabled: boolean;
    base_url: string;
    api_key: string;
    pin: string;
    timeout_seconds: number;
  };
  bangumi: {
    enabled: boolean;
    base_url: string;
    access_token: string;
    timeout_seconds: number;
    user_agent: string;
  };
}

export interface ScraperProviderStatus {
  provider_id: string;
  display_name: string;
  provider_kind: string;
  enabled: boolean;
  configured: boolean;
  healthy: boolean;
  capabilities: string[];
  scenarios: string[];
  message: string;
  checked_at?: string | null;
}

export interface ScraperSettingsResponse {
  settings: WebAppSettings;
  libraries: AdminLibrary[];
}

export type ScraperCacheStats = TmdbCacheStats;
export type ScraperFailureEntry = TmdbFailureEntry;

export interface WebAppSettings {
  server: {
    host: string;
    port: number;
    base_url: string;
    cors_allow_origins: string[];
  };
  auth: {
    token_ttl_hours: number;
    bootstrap_admin_user: string;
    bootstrap_admin_password: string;
    admin_api_key_prefix: string;
    max_failed_attempts: number;
    risk_window_seconds: number;
    risk_block_seconds: number;
    invite: {
      force_on_register: boolean;
      invitee_bonus_enabled: boolean;
      invitee_bonus_amount: string;
      inviter_rebate_enabled: boolean;
      inviter_rebate_rate: string;
    };
  };
  scan: Record<string, unknown>;
  storage: Record<string, unknown>;
  tmdb: TmdbConfig;
  scraper: ScraperConfig;
  security: {
    admin_allow_ips: string[];
    trust_x_forwarded_for: boolean;
    redact_sensitive_logs: boolean;
  };
  observability: Record<string, unknown>;
  jobs: Record<string, unknown>;
  agent: AgentSettings;
}

export interface AdminUpsertSettingsResponse {
  settings: WebAppSettings;
  restart_required: boolean;
}

export interface AdminUserSummaryItem {
  id: string;
  username: string;
  email: string | null;
  display_name: string | null;
  role: string;
  is_admin: boolean;
  is_disabled: boolean;
  active_auth_sessions: number;
  active_playback_sessions: number;
  subscription_name: string | null;
  used_bytes: number;
  created_at: string;
}

export interface AdminUserSummaryPage {
  page: number;
  page_size: number;
  total: number;
  items: AdminUserSummaryItem[];
}

export interface AdminUserProfileRecord {
  user_id: string;
  email: string | null;
  display_name: string | null;
  remark: string | null;
  created_at: string;
  updated_at: string;
}

export interface AdminUserSessionsSummary {
  active_auth_sessions: number;
  active_playback_sessions: number;
  last_auth_seen_at: string | null;
  last_playback_seen_at: string | null;
}

export interface AdminUserManageProfile {
  user: AdminUser;
  profile: AdminUserProfileRecord;
  stream_policy?: StreamPolicy | null;
  traffic_usage?: TrafficUsage | null;
  wallet?: Wallet | null;
  subscriptions?: Subscription[] | null;
  sessions_summary: AdminUserSessionsSummary;
}
