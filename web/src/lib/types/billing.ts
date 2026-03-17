// WalletAccount from backend - balance is Decimal serialized as string
export interface Wallet {
  user_id: string;
  balance: string;
  total_recharged: string;
  total_spent: string;
  updated_at: string;
}

// BillingPlan from backend - price is Decimal serialized as string
export interface Plan {
  id: string;
  code: string;
  name: string;
  price: string;
  duration_days: number;
  traffic_quota_bytes: number;
  traffic_window_days: number;
  permission_group_id?: string | null;
  permission_group_name?: string | null;
  enabled: boolean;
  updated_at: string;
}

export interface BillingPermissionGroup {
  id: string;
  code: string;
  name: string;
  enabled: boolean;
  domain_ids: string[];
  updated_at: string;
}

// BillingRechargeOrder from backend - amount is Decimal serialized as string
export interface RechargeOrder {
  id: string;
  user_id: string;
  out_trade_no: string;
  channel: string;
  amount: string;
  status: string;
  subject: string;
  provider_trade_no?: string | null;
  paid_at?: string | null;
  expires_at: string;
  created_at: string;
  updated_at: string;
}

export type RechargeOrderStatus = "pending" | "paid" | "expired" | "cancelled";

// CreateRechargeOrderRequest - amount is Decimal
export interface CreateRechargeOrderRequest {
  amount: string;
  channel?: string;
  subject?: string;
}

// BillingProration from backend
export interface BillingProration {
  time_ratio: string;
  traffic_ratio: string;
  applied_ratio: string;
  credit_amount: string;
  traffic_used_bytes: number;
  traffic_remaining_bytes: number;
}

// BillingPurchaseResult from backend
export interface PurchaseResult {
  wallet: Wallet;
  subscription: Subscription;
  charged_amount: string;
  proration?: BillingProration | null;
}

// BillingPlanSubscription from backend - prices are Decimal serialized as string
export interface Subscription {
  id: string;
  user_id: string;
  plan_id: string;
  plan_code: string;
  plan_name: string;
  plan_price: string;
  duration_days: number;
  traffic_quota_bytes: number;
  traffic_window_days: number;
  status: string;
  started_at: string;
  expires_at: string;
  replaced_at?: string | null;
  updated_at: string;
}

export type SubscriptionStatus = "active" | "expired" | "cancelled";

// WalletLedgerEntry from backend - amounts are Decimal serialized as string
export interface LedgerEntry {
  id: string;
  user_id: string;
  entry_type: string;
  amount: string;
  balance_after: string;
  reference_type?: string | null;
  reference_id?: string | null;
  note?: string | null;
  meta: Record<string, unknown>;
  created_at: string;
}

export type LedgerEntryType = "recharge" | "purchase" | "adjustment" | "refund";

// AdminAdjustWalletBalanceRequest from backend - amount is Decimal
export interface AdjustBalanceRequest {
  amount: string;
  note?: string;
}

// AdjustBalanceResult - returns updated wallet
export type AdjustBalanceResult = Wallet;

// AdminUpsertBillingPlanRequest from backend - price is Decimal
export interface CreatePlanRequest {
  id?: string;
  code: string;
  name: string;
  price: string;
  duration_days: number;
  traffic_quota_bytes: number;
  traffic_window_days: number;
  permission_group_id?: string | null;
  enabled?: boolean;
}

// UpdatePlanRequest - same as CreatePlanRequest but all fields optional
export interface UpdatePlanRequest {
  id?: string;
  code?: string;
  name?: string;
  price?: string;
  duration_days?: number;
  traffic_quota_bytes?: number;
  traffic_window_days?: number;
  permission_group_id?: string | null;
  enabled?: boolean;
}

export interface UpsertBillingPermissionGroupRequest {
  id?: string;
  code: string;
  name: string;
  enabled?: boolean;
  domain_ids: string[];
}

// AdminAssignSubscriptionRequest - admin assigns a plan to a user
export interface AssignSubscriptionRequest {
  plan_id: string;
  duration_days?: number; // Optional override duration, defaults to plan duration
}

// AdminUpdateSubscriptionRequest - admin updates subscription expiry
export interface UpdateSubscriptionRequest {
  expires_at: string;
}

// EPay gateway configuration
export interface EPayConfig {
  gateway_url: string;
  pid: string;
  key?: string; // Write-only: not returned by GET, only sent on update
  notify_url: string;
  return_url: string;
  sitename: string;
}

// Billing system settings
export interface BillingSettings {
  enabled: boolean;
  min_recharge_amount: string;
  max_recharge_amount: string;
  order_expire_minutes: number;
  channels: string[];
}

// Combined billing configuration
export interface BillingConfig {
  epay: EPayConfig;
  billing: BillingSettings;
}

// Request type for updating billing config (key is optional)
export interface UpdateBillingConfigRequest {
  epay?: Partial<EPayConfig>;
  billing?: Partial<BillingSettings>;
}
