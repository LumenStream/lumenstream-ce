import { beforeEach, describe, expect, it, vi } from "vitest";

import { apiRequest } from "@/lib/api/client";
import { mockGetPlans, mockGetWallet } from "@/lib/mock/billing";
import { runWithMock } from "@/lib/mock/mode";
import type { Plan, Wallet } from "@/lib/types/billing";

import { getPlans, getWallet } from "./billing";

vi.mock("@/lib/api/client", () => ({
  apiRequest: vi.fn(),
  getApiBaseUrl: vi.fn(() => "https://api.example.com"),
}));

vi.mock("@/lib/mock/billing", () => ({
  mockGetWallet: vi.fn(),
  mockGetPlans: vi.fn(),
  mockCreateRechargeOrder: vi.fn(),
  mockGetRechargeOrder: vi.fn(),
  mockPurchasePlan: vi.fn(),
}));

vi.mock("@/lib/mock/mode", () => ({
  isMockMode: vi.fn(() => false),
  setMockMode: vi.fn(),
  runWithMock: vi.fn(
    async (_mockFn: () => unknown, realFn: () => Promise<unknown>) => await realFn()
  ),
}));

const mockApiRequest = vi.mocked(apiRequest);
const mockRunWithMock = vi.mocked(runWithMock);
const mockMockGetPlans = vi.mocked(mockGetPlans);
const mockMockGetWallet = vi.mocked(mockGetWallet);

const sampleWallet: Wallet = {
  user_id: "user-1",
  balance: "20.50",
  total_recharged: "50.00",
  total_spent: "29.50",
  updated_at: "2026-02-21T00:00:00.000Z",
};

const samplePlan: Plan = {
  id: "plan-basic",
  code: "basic",
  name: "Basic",
  price: "15.00",
  duration_days: 30,
  traffic_quota_bytes: 107374182400,
  traffic_window_days: 30,
  enabled: true,
  updated_at: "2026-02-24T00:00:00.000Z",
};

describe("billing api contracts", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockRunWithMock.mockImplementation(
      async (_mockFn: () => unknown, realFn: () => Promise<unknown>) => await realFn()
    );
  });

  it("unwraps /billing/wallet envelope payload", async () => {
    mockApiRequest.mockResolvedValueOnce({
      wallet: sampleWallet,
      active_subscription: null,
      recent_ledger: [],
    });

    const result = await getWallet();

    expect(result).toEqual(sampleWallet);
    expect(mockApiRequest).toHaveBeenCalledWith("/billing/wallet");
  });

  it("keeps compatibility when backend returns wallet object directly", async () => {
    mockApiRequest.mockResolvedValueOnce(sampleWallet);

    await expect(getWallet()).resolves.toEqual(sampleWallet);
  });

  it("returns mock wallet when runWithMock chooses mock path", async () => {
    mockRunWithMock.mockImplementationOnce(async (mockFn: () => unknown) => await mockFn());
    mockMockGetWallet.mockResolvedValueOnce(sampleWallet);

    const result = await getWallet();

    expect(result).toEqual(sampleWallet);
    expect(mockMockGetWallet).toHaveBeenCalledTimes(1);
    expect(mockApiRequest).not.toHaveBeenCalled();
  });

  it("requests /billing/plans with auth enabled by default", async () => {
    mockApiRequest.mockResolvedValueOnce([samplePlan]);

    const result = await getPlans();

    expect(result).toEqual([samplePlan]);
    expect(mockApiRequest).toHaveBeenCalledWith("/billing/plans");
  });

  it("returns mock plans when runWithMock chooses mock path", async () => {
    mockRunWithMock.mockImplementationOnce(async (mockFn: () => unknown) => await mockFn());
    mockMockGetPlans.mockResolvedValueOnce([samplePlan]);

    const result = await getPlans();

    expect(result).toEqual([samplePlan]);
    expect(mockMockGetPlans).toHaveBeenCalledTimes(1);
    expect(mockApiRequest).not.toHaveBeenCalled();
  });
});
