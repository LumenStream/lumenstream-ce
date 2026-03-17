export interface AdminInviteSettings {
  force_on_register: boolean;
  invitee_bonus_enabled?: boolean;
  invitee_bonus_amount?: string;
  inviter_rebate_enabled?: boolean;
  inviter_rebate_rate?: string;
}

export interface InviteSummary {
  code: string;
  enabled: boolean;
  invited_count: number;
  rebate_total?: string;
  invitee_bonus_enabled?: boolean;
}

export interface InviteRelation {
  id: string;
  inviter_user_id: string;
  inviter_username: string;
  invitee_user_id: string;
  invitee_username: string;
  invite_code: string;
  created_at: string;
}

export interface InviteRebateRecord {
  id: string;
  invitee_user_id: string;
  invitee_username: string;
  inviter_user_id: string;
  inviter_username: string;
  recharge_order_id: string;
  recharge_amount: string;
  rebate_rate: string;
  rebate_amount: string;
  created_at: string;
}

export interface AdminUserStreamPolicyPayload {
  user_id: string;
  username: string;
  policy: StreamPolicy;
  defaults: {
    max_concurrent_streams: number;
    traffic_quota_bytes: number | null;
    traffic_window_days: number;
  };
}

export interface StreamPolicy {
  user_id: string;
  expires_at: string | null;
  max_concurrent_streams: number | null;
  traffic_quota_bytes: number | null;
  traffic_window_days: number;
  updated_at: string;
}

export interface TrafficUsageDaily {
  usage_date: string;
  bytes_served: number;
  real_bytes_served?: number;
}

export interface TrafficUsage {
  user_id: string;
  window_days: number;
  used_bytes: number;
  real_used_bytes?: number;
  quota_bytes: number | null;
  remaining_bytes: number | null;
  daily: TrafficUsageDaily[];
}

export interface TrafficUsageMediaItem {
  media_item_id: string;
  item_name: string;
  item_type: string;
  bytes_served: number;
  real_bytes_served: number;
  usage_days: number;
  last_usage_date: string;
}

export interface MyTrafficUsageMediaSummary {
  user_id: string;
  window_days: number;
  used_bytes: number;
  real_used_bytes: number;
  quota_bytes: number | null;
  remaining_bytes: number | null;
  unclassified_bytes: number;
  unclassified_real_bytes: number;
  items: TrafficUsageMediaItem[];
}

export interface TopTrafficUser {
  user_id: string;
  username: string;
  used_bytes: number;
}
