import { beforeEach, describe, expect, it, vi } from "vitest";

import { apiRequest } from "@/lib/api/client";
import { runWithMock } from "@/lib/mock/mode";

import {
  batchSetUserEnabled,
  cancelTaskRun,
  createLibrary,
  patchLibrary,
  deleteLibraryCover,
  getTaskRunsWebSocketUrl,
  getScraperSettings,
  testScraperProvider,
  uploadLibraryCover,
} from "./admin";

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
const mockRunWithMock = vi.mocked(runWithMock);

describe("admin api contracts", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockRunWithMock.mockImplementation(
      async (_mockFn: () => unknown, realFn: () => Promise<unknown>) => await realFn()
    );
  });

  it("sends disabled field for batch user status", async () => {
    mockApiRequest.mockResolvedValueOnce({
      updated: 1,
      users: [],
    });

    await batchSetUserEnabled(["user-1"], true);

    expect(mockApiRequest).toHaveBeenCalledWith("/admin/users/batch-status", {
      method: "POST",
      body: JSON.stringify({
        user_ids: ["user-1"],
        disabled: false,
      }),
    });
  });

  it("sends library_type and paths when creating library", async () => {
    mockApiRequest.mockResolvedValueOnce({
      id: "lib-001",
      name: "Movies",
      root_path: "/media/movies",
      library_type: "Movie",
      enabled: true,
      created_at: "2026-02-16T00:00:00.000Z",
    });

    await createLibrary({
      name: "Movies",
      paths: ["/media/movies"],
      library_type: "Movie",
    });

    expect(mockApiRequest).toHaveBeenCalledWith("/admin/libraries", {
      method: "POST",
      body: JSON.stringify({
        name: "Movies",
        paths: ["/media/movies"],
        library_type: "Movie",
      }),
    });
  });

  it("patches library_type for existing library", async () => {
    mockApiRequest.mockResolvedValueOnce({
      id: "lib-001",
      name: "Shows",
      root_path: "/media/shows",
      library_type: "Series",
      enabled: true,
      created_at: "2026-02-16T00:00:00.000Z",
    });

    await patchLibrary("lib-001", { library_type: "Series" });

    expect(mockApiRequest).toHaveBeenCalledWith("/admin/libraries/lib-001", {
      method: "PATCH",
      body: JSON.stringify({
        library_type: "Series",
      }),
    });
  });

  it("loads scraper settings from the generic scraper endpoint", async () => {
    mockApiRequest.mockResolvedValueOnce({
      settings: { scraper: { enabled: true } },
      libraries: [],
    });

    await getScraperSettings();

    expect(mockApiRequest).toHaveBeenCalledWith("/admin/scraper/settings");
  });

  it("posts provider test requests to the scraper endpoint", async () => {
    mockApiRequest.mockResolvedValueOnce({
      provider_id: "tmdb",
      display_name: "TMDB",
      provider_kind: "metadata",
      enabled: true,
      configured: true,
      healthy: true,
      capabilities: [],
      scenarios: [],
      message: "ready",
    });

    await testScraperProvider("tmdb");

    expect(mockApiRequest).toHaveBeenCalledWith("/admin/scraper/providers/tmdb/test", {
      method: "POST",
    });
  });

  it("builds task-center websocket url with token", () => {
    expect(getTaskRunsWebSocketUrl("demo-token")).toBe(
      "wss://api.example.com/admin/task-center/ws?token=demo-token"
    );
  });

  it("posts cancel request for task run", async () => {
    mockApiRequest.mockResolvedValueOnce({
      id: "run-001",
      kind: "scan_library",
      status: "cancelled",
      payload: {},
      attempts: 1,
      max_attempts: 3,
      dead_letter: false,
      created_at: "2026-02-16T00:00:00.000Z",
    });

    await cancelTaskRun("run-001");

    expect(mockApiRequest).toHaveBeenCalledWith("/admin/task-center/runs/run-001/cancel", {
      method: "POST",
    });
  });

  it("uploads library cover with binary body and content type", async () => {
    mockApiRequest.mockResolvedValueOnce(undefined);
    const file = new File(["cover"], "cover.png", { type: "image/png" });

    await uploadLibraryCover("lib-001", file);

    expect(mockApiRequest).toHaveBeenCalledWith("/Items/lib-001/Images/Primary", {
      method: "POST",
      headers: {
        "Content-Type": "image/png",
      },
      body: file,
    });
  });

  it("deletes library cover through item images path", async () => {
    mockApiRequest.mockResolvedValueOnce(undefined);

    await deleteLibraryCover("lib-001");

    expect(mockApiRequest).toHaveBeenCalledWith("/Items/lib-001/Images/Primary", {
      method: "DELETE",
    });
  });
});
