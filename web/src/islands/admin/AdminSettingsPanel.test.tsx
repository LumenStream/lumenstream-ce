/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  clearStorageCache,
  getSettings,
  invalidateStorageCache,
  upsertSettings,
} from "@/lib/api/admin";
import { toast } from "@/lib/notifications/toast-store";

import { AdminSettingsPanel } from "./AdminSettingsPanel";

const mockAuthState = {
  ready: true,
};

const sampleSettings = {
  server: {
    host: "0.0.0.0",
    port: 3000,
    base_url: "http://localhost:3000",
    cors_allow_origins: ["*"],
  },
  auth: {
    token_ttl_hours: 24,
    bootstrap_admin_user: "admin",
    bootstrap_admin_password: "admin",
    admin_api_key_prefix: "ls_",
    max_failed_attempts: 5,
    risk_window_seconds: 300,
    risk_block_seconds: 600,
    invite: {
      force_on_register: false,
      invitee_bonus_enabled: false,
      invitee_bonus_amount: "0.00",
      inviter_rebate_enabled: false,
      inviter_rebate_rate: "0.0000",
    },
  },
  scan: {},
  storage: {},
  tmdb: {
    enabled: false,
    api_key: "",
    language: "zh-CN",
    timeout_seconds: 10,
    request_interval_ms: 350,
    cache_ttl_seconds: 86400,
    retry_attempts: 3,
    retry_backoff_ms: 2000,
  },
  scraper: {
    enabled: false,
    default_strategy: "primary_with_fallback",
    providers: ["tmdb", "tvdb", "bangumi"],
    default_routes: {
      movie: ["tmdb", "tvdb"],
      series: ["tmdb", "tvdb"],
      image: ["tmdb", "tvdb"],
    },
    tvdb: {
      enabled: true,
      base_url: "https://api4.thetvdb.com/v4",
      api_key: "",
      pin: "",
      timeout_seconds: 15,
    },
    bangumi: {
      enabled: false,
      base_url: "https://api.bgm.tv",
      access_token: "",
      timeout_seconds: 15,
      user_agent: "lumenstream/0.1",
    },
  },
  security: {
    admin_allow_ips: [],
    trust_x_forwarded_for: false,
    redact_sensitive_logs: true,
  },
  observability: {},
  jobs: {},
  agent: {
    enabled: false,
    auto_mode: "automatic",
    missing_scan_enabled: false,
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
        preferred_resource_pix: ["2160P", "4K"],
        preferred_video_encode: ["X265"],
        preferred_resource_type: ["WEB-DL"],
        preferred_labels: ["中字"],
        excluded_keywords: ["CAM"],
      },
    },
  },
};

vi.mock("@/lib/auth/use-auth-session", () => ({
  useAuthSession: () => mockAuthState,
}));

vi.mock("@/components/domain/DataState", () => ({
  LoadingState: () => React.createElement("div", null, "loading"),
  ErrorState: () => React.createElement("div", null, "error"),
}));

vi.mock("@/lib/api/admin", () => ({
  getSettings: vi.fn(),
  upsertSettings: vi.fn(),
  clearStorageCache: vi.fn(),
  invalidateStorageCache: vi.fn(),
}));

vi.mock("@/lib/notifications/toast-store", () => ({
  toast: {
    success: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
    error: vi.fn(),
  },
}));

const mockGetSettings = vi.mocked(getSettings);
const mockUpsertSettings = vi.mocked(upsertSettings);
const mockClearStorageCache = vi.mocked(clearStorageCache);
const mockInvalidateStorageCache = vi.mocked(invalidateStorageCache);
const mockToast = vi.mocked(toast);

async function flushEffects() {
  await Promise.resolve();
  await Promise.resolve();
}

function setTextareaValue(element: HTMLTextAreaElement, value: string) {
  const descriptor = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value");
  descriptor?.set?.call(element, value);
  element.dispatchEvent(new Event("input", { bubbles: true }));
}

describe("AdminSettingsPanel", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    mockGetSettings.mockResolvedValue(sampleSettings);
    mockUpsertSettings.mockResolvedValue({
      settings: sampleSettings,
      restart_required: false,
    });
    mockClearStorageCache.mockResolvedValue({
      success: true,
      message: "缓存已清除",
    });
    mockInvalidateStorageCache.mockResolvedValue({
      success: true,
      message: "缓存已失效",
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

  it("blocks save and reports toast error when settings JSON is invalid", async () => {
    await act(async () => {
      root.render(<AdminSettingsPanel />);
      await flushEffects();
    });

    const textarea = container.querySelector("textarea") as HTMLTextAreaElement | null;
    expect(textarea).not.toBeNull();

    await act(async () => {
      if (textarea) {
        setTextareaValue(textarea, "{");
      }
      await flushEffects();
    });

    const saveButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "保存设置"
    );
    expect(saveButton).not.toBeUndefined();

    await act(async () => {
      saveButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(mockUpsertSettings).not.toHaveBeenCalled();
    expect(mockToast.error).toHaveBeenCalledWith(expect.stringContaining("JSON 解析失败"));
  });

  it("does not execute cache cleanup until confirmation and supports cancel", async () => {
    await act(async () => {
      root.render(<AdminSettingsPanel />);
      await flushEffects();
    });

    const cleanupButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "清除缓存"
    );
    expect(cleanupButton).not.toBeUndefined();

    await act(async () => {
      cleanupButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(document.body.textContent).toContain("确认清除缓存");

    const cancelButton = Array.from(document.body.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "取消"
    );
    expect(cancelButton).not.toBeUndefined();

    await act(async () => {
      cancelButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(mockClearStorageCache).not.toHaveBeenCalled();
    expect(document.body.textContent).not.toContain("确认要清除全部存储缓存吗？");
  });
});
