/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AdminRequestsPanel } from "./AdminRequestsPanel";

const adminListRequestsMock = vi.fn();
const adminGetRequestMock = vi.fn();
const adminGetAgentSettingsMock = vi.fn();
const adminListAgentProvidersMock = vi.fn();
const adminTestMoviePilotMock = vi.fn();

vi.mock("@/lib/auth/use-auth-session", () => ({
  useAuthSession: () => ({ ready: true }),
}));

vi.mock("@/lib/api/requests", () => ({
  adminListRequests: (...args: unknown[]) => adminListRequestsMock(...args),
  adminGetRequest: (...args: unknown[]) => adminGetRequestMock(...args),
  adminGetAgentSettings: (...args: unknown[]) => adminGetAgentSettingsMock(...args),
  adminListAgentProviders: (...args: unknown[]) => adminListAgentProvidersMock(...args),
  adminRetryRequest: vi.fn(),
  adminReviewRequest: vi.fn(),
  adminTestMoviePilot: (...args: unknown[]) => adminTestMoviePilotMock(...args),
  adminUpdateAgentSettings: vi.fn(),
}));

vi.mock("@/components/domain/DataState", () => ({
  EmptyState: ({ title }: { title: string }) => React.createElement("div", null, title),
  ErrorState: ({ title }: { title: string }) => React.createElement("div", null, title),
  LoadingState: ({ title }: { title?: string }) => React.createElement("div", null, title || ""),
}));

vi.mock("@/lib/notifications/toast-store", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

async function flushEffects() {
  await Promise.resolve();
  await Promise.resolve();
}

function findButtonByText(container: HTMLElement, label: string): HTMLButtonElement | undefined {
  return Array.from(container.querySelectorAll("button")).find((button) =>
    button.textContent?.includes(label)
  ) as HTMLButtonElement | undefined;
}

function findLabelByText(container: HTMLElement, label: string): HTMLLabelElement | undefined {
  return Array.from(container.querySelectorAll("label")).find((node) =>
    node.textContent?.includes(label)
  ) as HTMLLabelElement | undefined;
}

describe("AdminRequestsPanel", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    adminListRequestsMock.mockResolvedValue([
      {
        id: "req-1",
        request_type: "missing_episode",
        source: "auto_detected",
        title: "基地 缺集",
        content: "S02 缺 E05",
        media_type: "series",
        tmdb_id: 1,
        season_numbers: [2],
        episode_numbers: [5],
        status_user: "action_required",
        status_admin: "review_required",
        agent_stage: "manual_review",
        priority: 10,
        auto_handled: false,
        admin_note: "",
        agent_note: "等待人工处理",
        moviepilot_payload: {},
        moviepilot_result: {},
        created_at: "2026-03-12T00:00:00Z",
        updated_at: "2026-03-12T00:00:00Z",
      },
    ]);
    adminGetRequestMock.mockResolvedValue({
      request: {
        id: "req-1",
        request_type: "missing_episode",
        source: "auto_detected",
        title: "基地 缺集",
        content: "S02 缺 E05",
        media_type: "series",
        tmdb_id: 1,
        season_numbers: [2],
        episode_numbers: [5],
        status_user: "action_required",
        status_admin: "review_required",
        agent_stage: "manual_review",
        priority: 10,
        auto_handled: false,
        admin_note: "",
        agent_note: "等待人工处理",
        moviepilot_payload: {},
        moviepilot_result: {},
        created_at: "2026-03-12T00:00:00Z",
        updated_at: "2026-03-12T00:00:00Z",
      },
      events: [],
      workflow_kind: "missing_episode_repair",
      workflow_steps: [{ step: "manual_review", label: "人工接管", status: "blocked" }],
      required_capabilities: ["metadata", "search", "download", "subscribe", "notify"],
      manual_actions: [
        { action: "approve", label: "批准并重试", description: "重新进入自动处理。" },
      ],
    });
    adminGetAgentSettingsMock.mockResolvedValue({
      enabled: true,
      auto_mode: "automatic",
      missing_scan_enabled: true,
      missing_scan_cron: "0 */30 * * * *",
      auto_close_on_library_hit: true,
      review_required_on_parse_ambiguity: true,
      feedback_auto_route: true,
      llm: {
        enabled: false,
        base_url: "https://api.openai.com/v1",
        api_key: "",
        model: "gpt-4o-mini",
      },
      moviepilot: {
        enabled: true,
        base_url: "https://moviepilot.example.com",
        username: "admin",
        password: "***",
        timeout_seconds: 20,
        search_download_enabled: true,
        subscribe_fallback_enabled: true,
        filter: {
          min_seeders: 5,
          max_movie_size_gb: 30,
          max_episode_size_gb: 5,
          preferred_resource_pix: [],
          preferred_video_encode: [],
          preferred_resource_type: [],
          preferred_labels: [],
          excluded_keywords: [],
        },
      },
    });
    adminListAgentProvidersMock.mockResolvedValue([
      {
        provider_id: "moviepilot",
        display_name: "MoviePilot",
        provider_kind: "subscription_download",
        enabled: true,
        configured: true,
        healthy: true,
        capabilities: ["search", "download", "subscribe"],
        message: "authentication succeeded",
        checked_at: "2026-03-12T00:00:00Z",
      },
    ]);
    adminTestMoviePilotMock.mockResolvedValue({ base_url: "https://moviepilot.example.com" });
  });

  afterEach(() => {
    act(() => {
      root.unmount();
    });
    container.remove();
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = undefined;
    vi.clearAllMocks();
  });

  it("renders provider health and workflow metadata", async () => {
    await act(async () => {
      root.render(<AdminRequestsPanel />);
      await flushEffects();
    });

    await act(async () => {
      findButtonByText(container, "系统设置")?.dispatchEvent(
        new MouseEvent("click", { bubbles: true })
      );
      await flushEffects();
    });

    expect(container.textContent).toContain("Provider 健康");
    expect(container.textContent).toContain("MoviePilot");

    await act(async () => {
      findButtonByText(container, "请求工单")?.dispatchEvent(
        new MouseEvent("click", { bubbles: true })
      );
      await flushEffects();
    });

    await act(async () => {
      container
        .querySelector("tbody tr")
        ?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(container.textContent).toContain("missing_episode_repair");
    expect(container.textContent).toContain("批准并重试");
  });

  it("toggles MoviePilot enabled state before connection tests", async () => {
    await act(async () => {
      root.render(<AdminRequestsPanel />);
      await flushEffects();
    });

    await act(async () => {
      findButtonByText(container, "系统设置")?.dispatchEvent(
        new MouseEvent("click", { bubbles: true })
      );
      await flushEffects();
    });

    expect(container.textContent).toContain("启用 MoviePilot Provider");

    await act(async () => {
      findLabelByText(container, "启用 MoviePilot Provider")?.dispatchEvent(
        new MouseEvent("click", { bubbles: true })
      );
      await flushEffects();
    });

    await act(async () => {
      findButtonByText(container, "测试连接")?.dispatchEvent(
        new MouseEvent("click", { bubbles: true })
      );
      await flushEffects();
    });

    expect(adminTestMoviePilotMock).toHaveBeenCalledWith(
      expect.objectContaining({
        moviepilot: expect.objectContaining({ enabled: false }),
      })
    );
  });
});
