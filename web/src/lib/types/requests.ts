export type AgentRequestType =
  | "intake"
  | "media_request"
  | "feedback"
  | "missing_episode"
  | "missing_season";

export type AgentRequestUserStatus =
  | "processing"
  | "success"
  | "failed"
  | "action_required"
  | "closed";

export interface AgentRequest {
  id: string;
  request_type: AgentRequestType;
  source: string;
  user_id?: string | null;
  title: string;
  content: string;
  media_type: string;
  tmdb_id?: number | null;
  media_item_id?: string | null;
  series_id?: string | null;
  season_numbers: number[];
  episode_numbers: number[];
  status_user: AgentRequestUserStatus;
  status_admin: string;
  agent_stage: string;
  priority: number;
  auto_handled: boolean;
  admin_note: string;
  agent_note: string;
  provider_payload: Record<string, unknown>;
  provider_result: Record<string, unknown>;
  last_error?: string | null;
  created_at: string;
  updated_at: string;
  closed_at?: string | null;
}

export interface AgentRequestEvent {
  id: string;
  request_id: string;
  event_type: string;
  actor_user_id?: string | null;
  actor_username?: string | null;
  summary: string;
  detail: Record<string, unknown>;
  created_at: string;
}

export type AgentWorkflowStepStatus = "pending" | "active" | "completed" | "blocked" | "failed";

export interface AgentWorkflowStepState {
  step: string;
  label: string;
  status: AgentWorkflowStepStatus;
}

export interface AgentManualAction {
  action: string;
  label: string;
  description: string;
}

export interface AgentRequestDetail {
  request: AgentRequest;
  events: AgentRequestEvent[];
  workflow_kind: string;
  workflow_steps: AgentWorkflowStepState[];
  required_capabilities: string[];
  manual_actions: AgentManualAction[];
}

export interface AgentCreateRequest {
  request_type: AgentRequestType;
  source?: string;
  title: string;
  content?: string;
  media_type?: string;
  tmdb_id?: number | null;
  media_item_id?: string | null;
  series_id?: string | null;
  season_numbers?: number[];
  episode_numbers?: number[];
}

export interface AgentReviewRequest {
  action: "approve" | "reject" | "ignore" | "manual_complete";
  note?: string;
}

export interface AgentRequestsQuery {
  limit?: number;
  request_type?: AgentRequestType;
  status_admin?: string;
}

export interface AgentMoviePilotFilterSettings {
  min_seeders: number;
  max_movie_size_gb: number;
  max_episode_size_gb: number;
  preferred_resource_pix: string[];
  preferred_video_encode: string[];
  preferred_resource_type: string[];
  preferred_labels: string[];
  excluded_keywords: string[];
}

export interface AgentLlmSettings {
  enabled: boolean;
  base_url: string;
  api_key: string;
  model: string;
}

export interface AgentMoviePilotSettings {
  enabled: boolean;
  base_url: string;
  username: string;
  password: string;
  timeout_seconds: number;
  search_download_enabled: boolean;
  subscribe_fallback_enabled: boolean;
  filter: AgentMoviePilotFilterSettings;
}

export interface AgentSettings {
  enabled: boolean;
  auto_mode: string;
  missing_scan_enabled: boolean;
  missing_scan_cron: string;
  auto_close_on_library_hit: boolean;
  review_required_on_parse_ambiguity: boolean;
  feedback_auto_route: boolean;
  llm: AgentLlmSettings;
  moviepilot: AgentMoviePilotSettings;
}

export interface AgentProviderStatus {
  provider_id: string;
  display_name: string;
  provider_kind: string;
  enabled: boolean;
  configured: boolean;
  healthy: boolean;
  capabilities: string[];
  message: string;
  checked_at?: string | null;
}
