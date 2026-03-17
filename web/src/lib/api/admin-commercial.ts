import { apiRequest, getApiBaseUrl } from "@/lib/api/client";
import {
  mockAdminAdjustBalance,
  mockAdminAssignSubscription,
  mockAdminCancelSubscription,
  mockAdminCreatePermissionGroup,
  mockAdminCreatePlan,
  mockAdminGetBillingConfig,
  mockAdminGetInviteSettings,
  mockAdminGetUserLedger,
  mockAdminGetUserSubscriptions,
  mockAdminGetUserWallet,
  mockAdminListInviteRelations,
  mockAdminListInviteRebates,
  mockAdminListPermissionGroups,
  mockAdminListPlans,
  mockAdminListRechargeOrders,
  mockAdminUpdateBillingConfig,
  mockAdminUpdatePermissionGroup,
  mockAdminUpdatePlan,
  mockAdminUpdateSubscription,
  mockAdminUpsertInviteSettings,
  mockBuildAuditCsv,
  mockGetTopTrafficUsers,
  mockGetUserStreamPolicy,
  mockGetUserTrafficUsage,
  mockResetUserTrafficUsage,
  mockSetUserStreamPolicy,
} from "@/lib/mock/api";
import { runWithMock } from "@/lib/mock/mode";
import type {
  AdjustBalanceRequest,
  AdjustBalanceResult,
  AssignSubscriptionRequest,
  BillingConfig,
  BillingPermissionGroup,
  BillingSettings,
  CreatePlanRequest,
  EPayConfig,
  LedgerEntry,
  Plan,
  RechargeOrder,
  Subscription,
  UpsertBillingPermissionGroupRequest,
  UpdateBillingConfigRequest,
  UpdatePlanRequest,
  UpdateSubscriptionRequest,
  Wallet,
} from "@/lib/types/billing";
import type {
  AdminInviteSettings,
  AdminUserStreamPolicyPayload,
  InviteRelation,
  InviteRebateRecord,
  StreamPolicy,
  TopTrafficUser,
  TrafficUsage,
} from "@/lib/types/edition-commercial";

type LegacyBillingConfigResponse = BillingSettings & {
  epay: EPayConfig;
};

type BillingConfigResponse = BillingConfig | LegacyBillingConfigResponse;

type LegacyBillingConfigUpdateRequest = {
  epay?: Partial<EPayConfig>;
  enabled?: boolean;
  min_recharge_amount?: string;
  max_recharge_amount?: string;
  order_expire_minutes?: number;
  channels?: string[];
};

function normalizeBillingConfigResponse(payload: BillingConfigResponse): BillingConfig {
  if ("billing" in payload) {
    return payload;
  }

  return {
    epay: payload.epay,
    billing: {
      enabled: payload.enabled,
      min_recharge_amount: payload.min_recharge_amount,
      max_recharge_amount: payload.max_recharge_amount,
      order_expire_minutes: payload.order_expire_minutes,
      channels: payload.channels,
    },
  };
}

function toLegacyBillingConfigUpdateRequest(
  request: UpdateBillingConfigRequest
): LegacyBillingConfigUpdateRequest {
  const payload: LegacyBillingConfigUpdateRequest = {};

  if (request.epay) {
    payload.epay = request.epay;
  }

  if (request.billing) {
    if (request.billing.enabled !== undefined) {
      payload.enabled = request.billing.enabled;
    }
    if (request.billing.min_recharge_amount !== undefined) {
      payload.min_recharge_amount = request.billing.min_recharge_amount;
    }
    if (request.billing.max_recharge_amount !== undefined) {
      payload.max_recharge_amount = request.billing.max_recharge_amount;
    }
    if (request.billing.order_expire_minutes !== undefined) {
      payload.order_expire_minutes = request.billing.order_expire_minutes;
    }
    if (request.billing.channels !== undefined) {
      payload.channels = request.billing.channels;
    }
  }

  return payload;
}

export async function getInviteSettings(): Promise<AdminInviteSettings> {
  return runWithMock(
    () => mockAdminGetInviteSettings(),
    () => apiRequest<AdminInviteSettings>("/admin/invite/settings")
  );
}

export async function upsertInviteSettings(payload: {
  force_on_register?: boolean;
  invitee_bonus_enabled?: boolean;
  invitee_bonus_amount?: string;
  inviter_rebate_enabled?: boolean;
  inviter_rebate_rate?: string;
}): Promise<AdminInviteSettings> {
  return runWithMock(
    () => mockAdminUpsertInviteSettings(payload),
    () =>
      apiRequest<AdminInviteSettings>("/admin/invite/settings", {
        method: "POST",
        body: JSON.stringify(payload),
      })
  );
}

export async function listInviteRelations(limit = 100): Promise<InviteRelation[]> {
  return runWithMock(
    () => mockAdminListInviteRelations(limit),
    () =>
      apiRequest<InviteRelation[]>("/admin/invite/relations", {
        query: { limit },
      })
  );
}

export async function listInviteRebates(limit = 100): Promise<InviteRebateRecord[]> {
  return runWithMock(
    () => mockAdminListInviteRebates(limit),
    () =>
      apiRequest<InviteRebateRecord[]>("/admin/invite/rebates", {
        query: { limit },
      })
  );
}

export function buildAuditExportUrl(limit = 1000): string {
  const baseUrl = getApiBaseUrl();
  return `${baseUrl}/admin/audit-logs/export?limit=${limit}`;
}

export function buildMockAuditExportCsv(limit = 1000): string {
  return mockBuildAuditCsv(limit);
}

export async function getUserStreamPolicy(userId: string): Promise<StreamPolicy> {
  return runWithMock(
    () => mockGetUserStreamPolicy(userId),
    async () => {
      const payload = await apiRequest<AdminUserStreamPolicyPayload | StreamPolicy>(
        `/admin/users/${userId}/stream-policy`
      );
      if ("policy" in payload) {
        return payload.policy;
      }
      return payload;
    }
  );
}

export async function setUserStreamPolicy(
  userId: string,
  policy: Partial<Omit<StreamPolicy, "user_id" | "updated_at">>
): Promise<StreamPolicy> {
  return runWithMock(
    () => mockSetUserStreamPolicy(userId, policy),
    () =>
      apiRequest<StreamPolicy>(`/admin/users/${userId}/stream-policy`, {
        method: "POST",
        body: JSON.stringify(policy),
      })
  );
}

export async function getUserTrafficUsage(userId: string): Promise<TrafficUsage> {
  return runWithMock(
    () => mockGetUserTrafficUsage(userId),
    () => apiRequest<TrafficUsage>(`/admin/users/${userId}/traffic-usage`)
  );
}

export async function resetUserTrafficUsage(userId: string): Promise<TrafficUsage> {
  return runWithMock(
    () => mockResetUserTrafficUsage(userId),
    async () => {
      await apiRequest<{ user_id: string; deleted_rows: number }>(
        `/admin/users/${userId}/traffic-usage/reset`,
        {
          method: "POST",
        }
      );
      return await getUserTrafficUsage(userId);
    }
  );
}

export async function getTopTrafficUsers(limit = 20): Promise<TopTrafficUser[]> {
  return runWithMock(
    () => mockGetTopTrafficUsers(limit),
    async () => {
      const payload = await apiRequest<{
        limit: number;
        window_days: number;
        items: TopTrafficUser[];
      }>("/admin/users/traffic-usage/top", {
        query: { limit },
      });
      return payload.items;
    }
  );
}

export async function adminListPlans(): Promise<Plan[]> {
  return runWithMock(
    () => mockAdminListPlans(),
    () => apiRequest<Plan[]>("/admin/billing/plans")
  );
}

export async function adminListPermissionGroups(): Promise<BillingPermissionGroup[]> {
  return runWithMock(
    () => mockAdminListPermissionGroups(),
    () => apiRequest<BillingPermissionGroup[]>("/admin/billing/permission-groups")
  );
}

export async function adminCreatePermissionGroup(
  request: UpsertBillingPermissionGroupRequest
): Promise<BillingPermissionGroup> {
  return runWithMock(
    () => mockAdminCreatePermissionGroup(request),
    () =>
      apiRequest<BillingPermissionGroup>("/admin/billing/permission-groups", {
        method: "POST",
        body: JSON.stringify(request),
      })
  );
}

export async function adminUpdatePermissionGroup(
  groupId: string,
  request: UpsertBillingPermissionGroupRequest
): Promise<BillingPermissionGroup> {
  return runWithMock(
    () => mockAdminUpdatePermissionGroup(groupId, request),
    () =>
      apiRequest<BillingPermissionGroup>("/admin/billing/permission-groups", {
        method: "POST",
        body: JSON.stringify({ ...request, id: groupId }),
      })
  );
}

export async function adminCreatePlan(request: CreatePlanRequest): Promise<Plan> {
  return runWithMock(
    () => mockAdminCreatePlan(request),
    () =>
      apiRequest<Plan>("/admin/billing/plans", {
        method: "POST",
        body: JSON.stringify(request),
      })
  );
}

export async function adminUpdatePlan(planId: string, request: UpdatePlanRequest): Promise<Plan> {
  return runWithMock(
    () => mockAdminUpdatePlan(planId, request),
    () =>
      apiRequest<Plan>("/admin/billing/plans", {
        method: "POST",
        body: JSON.stringify({ id: planId, ...request }),
      })
  );
}

export async function adminListRechargeOrders(limit = 100): Promise<RechargeOrder[]> {
  return runWithMock(
    () => mockAdminListRechargeOrders(limit),
    () =>
      apiRequest<RechargeOrder[]>("/admin/billing/recharge-orders", {
        query: { limit },
      })
  );
}

export async function adminGetUserWallet(userId: string): Promise<Wallet> {
  return runWithMock(
    () => mockAdminGetUserWallet(userId),
    () => apiRequest<Wallet>(`/admin/billing/users/${userId}/wallet`)
  );
}

export async function adminGetUserLedger(userId: string, limit = 50): Promise<LedgerEntry[]> {
  return runWithMock(
    () => mockAdminGetUserLedger(userId, limit),
    () =>
      apiRequest<LedgerEntry[]>(`/admin/billing/users/${userId}/ledger`, {
        query: { limit },
      })
  );
}

export async function adminGetUserSubscriptions(userId: string): Promise<Subscription[]> {
  return runWithMock(
    () => mockAdminGetUserSubscriptions(userId),
    () => apiRequest<Subscription[]>(`/admin/billing/users/${userId}/subscriptions`)
  );
}

export async function adminAdjustBalance(
  userId: string,
  request: AdjustBalanceRequest
): Promise<AdjustBalanceResult> {
  return runWithMock(
    () => mockAdminAdjustBalance(userId, request),
    () =>
      apiRequest<AdjustBalanceResult>(`/admin/billing/users/${userId}/adjust-balance`, {
        method: "POST",
        body: JSON.stringify(request),
      })
  );
}

export async function adminGetBillingConfig(): Promise<BillingConfig> {
  return runWithMock(
    () => mockAdminGetBillingConfig(),
    async () => {
      const payload = await apiRequest<BillingConfigResponse>("/admin/billing/config");
      return normalizeBillingConfigResponse(payload);
    }
  );
}

export async function adminUpdateBillingConfig(
  request: UpdateBillingConfigRequest
): Promise<BillingConfig> {
  return runWithMock(
    () => mockAdminUpdateBillingConfig(request),
    async () => {
      const payload = await apiRequest<BillingConfigResponse>("/admin/billing/config", {
        method: "PATCH",
        body: JSON.stringify(toLegacyBillingConfigUpdateRequest(request)),
      });
      return normalizeBillingConfigResponse(payload);
    }
  );
}

export async function adminAssignSubscription(
  userId: string,
  request: AssignSubscriptionRequest
): Promise<Subscription> {
  return runWithMock(
    () => mockAdminAssignSubscription(userId, request),
    () =>
      apiRequest<Subscription>(`/admin/billing/users/${userId}/subscriptions`, {
        method: "POST",
        body: JSON.stringify({
          plan_id: request.plan_id,
          duration_days: request.duration_days,
        }),
      })
  );
}

export async function adminUpdateSubscription(
  userId: string,
  subscriptionId: string,
  request: UpdateSubscriptionRequest
): Promise<Subscription> {
  return runWithMock(
    () => mockAdminUpdateSubscription(subscriptionId, request),
    () =>
      apiRequest<Subscription>(`/admin/billing/users/${userId}/subscriptions/${subscriptionId}`, {
        method: "PATCH",
        body: JSON.stringify(request),
      })
  );
}

export async function adminCancelSubscription(
  userId: string,
  subscriptionId: string
): Promise<Subscription> {
  return runWithMock(
    () => mockAdminCancelSubscription(subscriptionId),
    () =>
      apiRequest<Subscription>(`/admin/billing/users/${userId}/subscriptions/${subscriptionId}`, {
        method: "DELETE",
      })
  );
}
