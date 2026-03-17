import { beforeEach, describe, expect, it, vi } from "vitest";

import { apiRequest } from "@/lib/api/client";
import { mockGetMyTrafficUsageByMedia } from "@/lib/mock/api";
import { runWithMock } from "@/lib/mock/mode";
import type { MyTrafficUsageMediaSummary } from "@/lib/types/edition-commercial";

import { getMyTrafficUsageByMedia } from "./traffic";

vi.mock("@/lib/api/client", () => ({
  apiRequest: vi.fn(),
}));

vi.mock("@/lib/mock/api", () => ({
  mockGetMyTrafficUsageByMedia: vi.fn(),
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
const mockMockGetMyTrafficUsageByMedia = vi.mocked(mockGetMyTrafficUsageByMedia);

const sampleResponse: MyTrafficUsageMediaSummary = {
  user_id: "user-1",
  window_days: 30,
  used_bytes: 1024,
  real_used_bytes: 768,
  quota_bytes: 2048,
  remaining_bytes: 1024,
  unclassified_bytes: 0,
  unclassified_real_bytes: 0,
  items: [
    {
      media_item_id: "item-1",
      item_name: "Interstellar",
      item_type: "Movie",
      bytes_served: 1024,
      real_bytes_served: 768,
      usage_days: 1,
      last_usage_date: "2026-02-24",
    },
  ],
};

describe("traffic api contracts", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockRunWithMock.mockImplementation(
      async (_mockFn: () => unknown, realFn: () => Promise<unknown>) => await realFn()
    );
  });

  it("requests /api/traffic/me/items with limit query", async () => {
    mockApiRequest.mockResolvedValueOnce(sampleResponse);

    const result = await getMyTrafficUsageByMedia(120);

    expect(result).toEqual(sampleResponse);
    expect(mockApiRequest).toHaveBeenCalledWith("/api/traffic/me/items", {
      query: { limit: 120 },
    });
  });

  it("uses mock path when runWithMock chooses mock implementation", async () => {
    mockRunWithMock.mockImplementationOnce(async (mockFn: () => unknown) => await mockFn());
    mockMockGetMyTrafficUsageByMedia.mockResolvedValueOnce(sampleResponse);

    const result = await getMyTrafficUsageByMedia(80);

    expect(result).toEqual(sampleResponse);
    expect(mockMockGetMyTrafficUsageByMedia).toHaveBeenCalledWith(80);
    expect(mockApiRequest).not.toHaveBeenCalled();
  });
});
