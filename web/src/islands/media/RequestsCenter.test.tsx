/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { RequestsCenter } from "./RequestsCenter";

const listMyRequestsMock = vi.fn();
const getMyRequestMock = vi.fn();
const createMyRequestMock = vi.fn();
const resubmitMyRequestMock = vi.fn();

vi.mock("@/lib/auth/use-auth-session", () => ({
  useAuthSession: () => ({ ready: true }),
}));

vi.mock("@/lib/api/requests", () => ({
  listMyRequests: (...args: unknown[]) => listMyRequestsMock(...args),
  getMyRequest: (...args: unknown[]) => getMyRequestMock(...args),
  createMyRequest: (...args: unknown[]) => createMyRequestMock(...args),
  resubmitMyRequest: (...args: unknown[]) => resubmitMyRequestMock(...args),
}));

vi.mock("@/components/domain/DataState", () => ({
  EmptyState: ({ title }: { title: string }) => React.createElement("div", null, title),
  ErrorState: ({ title }: { title: string }) => React.createElement("div", null, title),
  LoadingState: ({ title }: { title?: string }) => React.createElement("div", null, title || ""),
}));

vi.mock("@/lib/notifications/toast-store", () => ({
  toast: {
    success: vi.fn(),
    warning: vi.fn(),
    error: vi.fn(),
  },
}));

async function flushEffects() {
  await Promise.resolve();
  await Promise.resolve();
}

describe("RequestsCenter", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    listMyRequestsMock.mockResolvedValue([
      {
        id: "req-1",
        request_type: "media_request",
        source: "user_submit",
        title: "测试求片",
        content: "",
        media_type: "series",
        tmdb_id: 1,
        season_numbers: [1],
        episode_numbers: [],
        status_user: "processing",
        status_admin: "auto_processing",
        agent_stage: "mp_search",
        priority: 0,
        auto_handled: true,
        admin_note: "",
        agent_note: "正在搜索",
        moviepilot_payload: {},
        moviepilot_result: {},
        created_at: "2026-03-12T00:00:00Z",
        updated_at: "2026-03-12T00:00:00Z",
      },
    ]);
    getMyRequestMock.mockResolvedValue({
      request: {
        id: "req-1",
        request_type: "media_request",
        source: "user_submit",
        title: "测试求片",
        content: "",
        media_type: "series",
        tmdb_id: 1,
        season_numbers: [1],
        episode_numbers: [],
        status_user: "processing",
        status_admin: "auto_processing",
        agent_stage: "mp_search",
        priority: 0,
        auto_handled: true,
        admin_note: "",
        agent_note: "正在搜索",
        moviepilot_payload: {},
        moviepilot_result: {},
        created_at: "2026-03-12T00:00:00Z",
        updated_at: "2026-03-12T00:00:00Z",
      },
      events: [],
      workflow_kind: "request_media",
      workflow_steps: [
        { step: "accepted", label: "接单", status: "completed" },
        { step: "provider_search", label: "Provider 搜索", status: "active" },
      ],
      required_capabilities: ["search", "download", "subscribe", "notify"],
      manual_actions: [],
    });
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

  it("renders workflow kind and required capabilities for selected request", async () => {
    await act(async () => {
      root.render(<RequestsCenter />);
      await flushEffects();
    });

    await act(async () => {
      container
        .querySelector("tbody tr")
        ?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(container.textContent).toContain("处理进度");
    expect(container.textContent).not.toContain("request_media");
    expect(container.textContent).not.toContain("search, download, subscribe, notify");
  });
});
