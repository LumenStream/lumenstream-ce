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

function fillTextarea(container: HTMLElement, value: string) {
  const textarea = container.querySelector("textarea") as HTMLTextAreaElement | null;
  if (!textarea) throw new Error("textarea not found");
  const setter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set;
  setter?.call(textarea, value);
  textarea.dispatchEvent(new Event("input", { bubbles: true }));
  textarea.dispatchEvent(new Event("change", { bubbles: true }));
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
        content: "逐玉的资源能换奈飞的资源么，爱奇艺的有广告。",
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
        provider_payload: {},
        provider_result: {},
        created_at: "2026-03-12T00:00:00Z",
        updated_at: "2026-03-12T00:00:00Z",
      },
    ]);
    getMyRequestMock.mockResolvedValue({
      request: {
        id: "req-1",
        request_type: "media_request",
        source: "user_submit",
        title: "逐玉",
        content: "逐玉的资源能换奈飞的资源么，爱奇艺的有广告。",
        media_type: "series",
        tmdb_id: 279388,
        season_numbers: [1],
        episode_numbers: [],
        status_user: "processing",
        status_admin: "auto_processing",
        agent_stage: "mp_search",
        priority: 0,
        auto_handled: true,
        admin_note: "",
        agent_note: "正在搜索",
        provider_payload: {},
        provider_result: {
          recognized_intent: {
            request_type: "media_request",
            title: "逐玉",
            avoid_sources: ["iQIYI"],
            preferred_sources: ["Netflix"],
          },
          exact_query: {
            title: "逐玉",
            mtype: "电视剧",
          },
          agent_plan: {
            action: "manual_review",
            reason: "暂无满足偏好的资源",
          },
        },
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
    createMyRequestMock.mockResolvedValue({
      request: {
        id: "req-new",
        request_type: "intake",
        source: "user_submit",
        title: "基地第二季缺第5集",
        content: "基地第二季缺第5集",
        media_type: "unknown",
        tmdb_id: null,
        season_numbers: [],
        episode_numbers: [],
        status_user: "processing",
        status_admin: "new",
        agent_stage: "queued",
        priority: 0,
        auto_handled: false,
        admin_note: "",
        agent_note: "请求已入队",
        provider_payload: {},
        provider_result: {},
        created_at: "2026-03-12T00:00:00Z",
        updated_at: "2026-03-12T00:00:00Z",
      },
      events: [],
      workflow_kind: "unknown",
      workflow_steps: [],
      required_capabilities: [],
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

  it("renders audit insight blocks for selected request", async () => {
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

    expect(container.textContent).toContain("Agent 审计视图");
    expect(container.textContent).toContain("意图识别");
    expect(container.textContent).toContain("精确搜索参数");
    expect(container.textContent).toContain("执行计划");
  });

  it("submits unified intake request with natural language", async () => {
    await act(async () => {
      root.render(<RequestsCenter />);
      await flushEffects();
    });

    await act(async () => {
      Array.from(container.querySelectorAll("button"))
        .find((button) => button.textContent?.includes("新建请求"))
        ?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    await act(async () => {
      fillTextarea(container, "基地第二季缺第5集");
      Array.from(container.querySelectorAll("button"))
        .find((button) => button.textContent?.includes("提交请求"))
        ?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(createMyRequestMock).toHaveBeenCalledWith(
      expect.objectContaining({
        request_type: "intake",
        title: "基地第二季缺第5集",
        content: "基地第二季缺第5集",
      })
    );
  });
});
