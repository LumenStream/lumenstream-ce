import { beforeEach, describe, expect, it, vi } from "vitest";

import { apiRequest, getApiBaseUrl } from "@/lib/api/client";
import { runWithMock } from "@/lib/mock/mode";
import type { StreamPolicy, TrafficUsage } from "@/lib/types/edition-commercial";
import type { BillingConfig } from "@/lib/types/billing";

import {
  adminAssignSubscription,
  adminGetBillingConfig,
  adminUpdateBillingConfig,
  buildAuditExportUrl,
  getTopTrafficUsers,
  getUserStreamPolicy,
  resetUserTrafficUsage,
} from "./admin-commercial";

vi.mock("@/lib/api/client", () => ({
  apiRequest: vi.fn(),
  getApiBaseUrl: vi.fn(() => "https://api.example.com"),
}));

vi.mock("@/lib/mock/mode", () => ({
  isMockMode: vi.fn(() => false),
  setMockMode: vi.fn(),
  runWithMock: vi.fn(
    async (_mockFn: () => unknown, realFn: () => Promise<unknown>) => await realFn()
  ),
}));

const mockApiRequest = vi.mocked(apiRequest);
const mockGetApiBaseUrl = vi.mocked(getApiBaseUrl);
const mockRunWithMock = vi.mocked(runWithMock);

describe("admin commercial api contracts", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetApiBaseUrl.mockReturnValue("https://api.example.com");
    mockRunWithMock.mockImplementation(
      async (_mockFn: () => unknown, realFn: () => Promise<unknown>) => await realFn()
    );
  });

  it("unwraps /admin/users/{id}/stream-policy envelope", async () => {
    const policy: StreamPolicy = {
      user_id: "user-1",
      expires_at: null,
      max_concurrent_streams: 2,
      traffic_quota_bytes: 1024,
      traffic_window_days: 30,
      updated_at: "2026-02-16T00:00:00.000Z",
    };
    mockApiRequest.mockResolvedValueOnce({
      user_id: "user-1",
      username: "demo",
      policy,
      defaults: {
        max_concurrent_streams: 2,
        traffic_quota_bytes: null,
        traffic_window_days: 30,
      },
    });

    const result = await getUserStreamPolicy("user-1");

    expect(result).toEqual(policy);
  });

  it("unwraps /admin/users/traffic-usage/top items array", async () => {
    const items = [
      {
        user_id: "user-1",
        username: "demo",
        used_bytes: 1024,
      },
    ];
    mockApiRequest.mockResolvedValueOnce({
      limit: 20,
      window_days: 30,
      items,
    });

    const result = await getTopTrafficUsers(20);

    expect(result).toEqual(items);
    expect(mockApiRequest).toHaveBeenCalledWith("/admin/users/traffic-usage/top", {
      query: { limit: 20 },
    });
  });

  it("sends duration_days when assigning subscription", async () => {
    mockApiRequest.mockResolvedValueOnce({
      id: "sub-1",
      user_id: "user-1",
      plan_id: "plan-1",
      plan_code: "standard",
      plan_name: "标准套餐",
      plan_price: "29.00",
      duration_days: 10,
      traffic_quota_bytes: 1024,
      traffic_window_days: 30,
      status: "active",
      started_at: "2026-02-16T00:00:00.000Z",
      expires_at: "2026-02-26T00:00:00.000Z",
      updated_at: "2026-02-16T00:00:00.000Z",
    });

    await adminAssignSubscription("user-1", {
      plan_id: "plan-1",
      duration_days: 10,
    });

    expect(mockApiRequest).toHaveBeenCalledWith("/admin/billing/users/user-1/subscriptions", {
      method: "POST",
      body: JSON.stringify({
        plan_id: "plan-1",
        duration_days: 10,
      }),
    });
  });

  it("normalizes flat /admin/billing/config response shape", async () => {
    mockApiRequest.mockResolvedValueOnce({
      enabled: true,
      min_recharge_amount: "1.00",
      max_recharge_amount: "2000.00",
      order_expire_minutes: 30,
      channels: ["alipay", "wxpay"],
      epay: {
        gateway_url: "https://pay.example.com/submit.php",
        pid: "1001",
        notify_url: "https://api.example.com/billing/notify",
        return_url: "https://app.example.com/billing/return",
        sitename: "LumenStream",
      },
    });

    const result = await adminGetBillingConfig();

    expect(result).toEqual({
      epay: {
        gateway_url: "https://pay.example.com/submit.php",
        pid: "1001",
        notify_url: "https://api.example.com/billing/notify",
        return_url: "https://app.example.com/billing/return",
        sitename: "LumenStream",
      },
      billing: {
        enabled: true,
        min_recharge_amount: "1.00",
        max_recharge_amount: "2000.00",
        order_expire_minutes: 30,
        channels: ["alipay", "wxpay"],
      },
    } satisfies BillingConfig);
    expect(mockApiRequest).toHaveBeenCalledWith("/admin/billing/config");
  });

  it("sends legacy flat payload when updating billing config", async () => {
    mockApiRequest.mockResolvedValueOnce({
      enabled: false,
      min_recharge_amount: "10.00",
      max_recharge_amount: "1000.00",
      order_expire_minutes: 60,
      channels: ["alipay"],
      epay: {
        gateway_url: "https://pay.example.com/submit.php",
        pid: "1001",
        notify_url: "https://api.example.com/billing/notify",
        return_url: "https://app.example.com/billing/return",
        sitename: "LumenStream",
      },
    });

    const result = await adminUpdateBillingConfig({
      epay: {
        pid: "1001",
      },
      billing: {
        enabled: false,
        min_recharge_amount: "10.00",
        max_recharge_amount: "1000.00",
        order_expire_minutes: 60,
        channels: ["alipay"],
      },
    });

    expect(mockApiRequest).toHaveBeenCalledWith("/admin/billing/config", {
      method: "PATCH",
      body: JSON.stringify({
        epay: {
          pid: "1001",
        },
        enabled: false,
        min_recharge_amount: "10.00",
        max_recharge_amount: "1000.00",
        order_expire_minutes: 60,
        channels: ["alipay"],
      }),
    });
    expect(result).toEqual({
      epay: {
        gateway_url: "https://pay.example.com/submit.php",
        pid: "1001",
        notify_url: "https://api.example.com/billing/notify",
        return_url: "https://app.example.com/billing/return",
        sitename: "LumenStream",
      },
      billing: {
        enabled: false,
        min_recharge_amount: "10.00",
        max_recharge_amount: "1000.00",
        order_expire_minutes: 60,
        channels: ["alipay"],
      },
    } satisfies BillingConfig);
  });

  it("re-queries traffic summary after reset", async () => {
    const usage: TrafficUsage = {
      user_id: "user-1",
      window_days: 30,
      used_bytes: 0,
      quota_bytes: null,
      remaining_bytes: null,
      daily: [],
    };
    mockApiRequest
      .mockResolvedValueOnce({
        user_id: "user-1",
        deleted_rows: 3,
      })
      .mockResolvedValueOnce(usage);

    const result = await resetUserTrafficUsage("user-1");

    expect(result).toEqual(usage);
    expect(mockApiRequest).toHaveBeenNthCalledWith(1, "/admin/users/user-1/traffic-usage/reset", {
      method: "POST",
    });
    expect(mockApiRequest).toHaveBeenNthCalledWith(2, "/admin/users/user-1/traffic-usage");
  });

  it("builds audit export urls from the api base", () => {
    expect(buildAuditExportUrl(500)).toBe(
      "https://api.example.com/admin/audit-logs/export?limit=500"
    );
  });
});
