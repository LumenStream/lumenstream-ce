import type {
  CreateRechargeOrderRequest,
  LedgerEntry,
  Plan,
  PurchaseResult,
  RechargeOrder,
  Subscription,
  Wallet,
} from "@/lib/types/billing";

import { apiRequest, getApiBaseUrl } from "@/lib/api/client";
import {
  mockCreateRechargeOrder,
  mockGetPlans,
  mockGetRechargeOrder,
  mockGetWallet,
  mockPurchasePlan,
} from "@/lib/mock/billing";
import { runWithMock } from "@/lib/mock/mode";

interface WalletPayload {
  wallet: Wallet | null;
  active_subscription: Subscription | null;
  recent_ledger: LedgerEntry[];
}

function extractWallet(payload: Wallet | WalletPayload): Wallet {
  if ("wallet" in payload) {
    if (!payload.wallet) {
      throw {
        status: 500,
        message: "钱包数据缺失",
      };
    }
    return payload.wallet;
  }
  return payload;
}

export async function getWallet(): Promise<Wallet> {
  return runWithMock(
    () => mockGetWallet(),
    async () => {
      const payload = await apiRequest<Wallet | WalletPayload>("/billing/wallet");
      return extractWallet(payload);
    }
  );
}

export async function getPlans(): Promise<Plan[]> {
  return runWithMock(
    () => mockGetPlans(),
    () => apiRequest<Plan[]>("/billing/plans")
  );
}

export async function createRechargeOrder(
  request: CreateRechargeOrderRequest
): Promise<RechargeOrder> {
  return runWithMock(
    () => mockCreateRechargeOrder(request),
    () =>
      apiRequest<RechargeOrder>("/billing/recharge/orders", {
        method: "POST",
        body: JSON.stringify(request),
      })
  );
}

export async function getRechargeOrder(orderId: string): Promise<RechargeOrder> {
  return runWithMock(
    () => mockGetRechargeOrder(orderId),
    () => apiRequest<RechargeOrder>(`/billing/recharge/orders/${orderId}`)
  );
}

export function getRechargeOrderWebSocketUrl(orderId: string, token: string): string {
  const base = getApiBaseUrl();
  const httpUrl = new URL(`${base}/billing/recharge/orders/${orderId}/ws`);
  httpUrl.searchParams.set("token", token);
  if (httpUrl.protocol === "https:") {
    httpUrl.protocol = "wss:";
  } else {
    httpUrl.protocol = "ws:";
  }
  return httpUrl.toString();
}

export async function purchasePlan(planId: string): Promise<PurchaseResult> {
  return runWithMock(
    () => mockPurchasePlan(planId),
    () =>
      apiRequest<PurchaseResult>(`/billing/plans/${planId}/purchase`, {
        method: "POST",
      })
  );
}
