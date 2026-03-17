import { beforeEach, describe, expect, it, vi } from "vitest";

import { apiRequest } from "@/lib/api/client";
import { runWithMock } from "@/lib/mock/mode";

import {
  adminListAgentProviders,
  adminListRequests,
  adminReviewRequest,
  createMyRequest,
  listMyRequests,
} from "./requests";

vi.mock("@/lib/api/client", () => ({
  apiRequest: vi.fn(),
}));

vi.mock("@/lib/mock/mode", () => ({
  runWithMock: vi.fn((mockFn: () => unknown, realFn: () => unknown) => realFn()),
}));

describe("requests api", () => {
  const mockApiRequest = vi.mocked(apiRequest);
  const mockRunWithMock = vi.mocked(runWithMock);

  beforeEach(() => {
    mockApiRequest.mockReset();
    mockRunWithMock.mockClear();
  });

  it("requests my agent requests with query params", async () => {
    mockApiRequest.mockResolvedValueOnce([]);
    await listMyRequests({ limit: 20, request_type: "feedback" });
    expect(mockApiRequest).toHaveBeenCalledWith("/api/requests", {
      query: { limit: 20, request_type: "feedback" },
    });
  });

  it("posts create request payload", async () => {
    mockApiRequest.mockResolvedValueOnce({
      request: { id: "req-1" },
      events: [],
    });
    await createMyRequest({
      request_type: "media_request",
      title: "沙丘",
      content: "4K",
      tmdb_id: 1,
    });
    expect(mockApiRequest).toHaveBeenCalledWith("/api/requests", {
      method: "POST",
      body: JSON.stringify({
        request_type: "media_request",
        title: "沙丘",
        content: "4K",
        tmdb_id: 1,
      }),
    });
  });

  it("requests admin list using admin endpoint", async () => {
    mockApiRequest.mockResolvedValueOnce([]);
    await adminListRequests({ status_admin: "review_required" });
    expect(mockApiRequest).toHaveBeenCalledWith("/admin/requests", {
      query: { status_admin: "review_required" },
    });
  });

  it("posts admin review action", async () => {
    mockApiRequest.mockResolvedValueOnce({ request: { id: "req-1" }, events: [] });
    await adminReviewRequest("req-1", { action: "approve", note: "ok" });
    expect(mockApiRequest).toHaveBeenCalledWith("/admin/requests/req-1/review", {
      method: "POST",
      body: JSON.stringify({ action: "approve", note: "ok" }),
    });
  });

  it("requests agent provider health from admin endpoint", async () => {
    mockApiRequest.mockResolvedValueOnce([]);
    await adminListAgentProviders();
    expect(mockApiRequest).toHaveBeenCalledWith("/admin/agent/providers");
  });
});
