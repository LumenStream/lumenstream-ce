/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { buildAuditExportUrl, listAuditLogs } from "@/lib/api/admin";
import { getPublicSystemCapabilities } from "@/lib/api/system";
import { toast } from "@/lib/notifications/toast-store";

import { AdminAuditLogsPanel } from "./AdminAuditLogsPanel";

const mockAuthState = {
  ready: true,
};

vi.mock("@/lib/auth/use-auth-session", () => ({
  useAuthSession: () => mockAuthState,
}));

vi.mock("@/components/domain/DataState", () => ({
  LoadingState: () => React.createElement("div", null, "loading"),
  ErrorState: () => React.createElement("div", null, "error"),
}));

vi.mock("@/lib/auth/token", () => ({
  getAccessToken: () => "demo-token",
}));

vi.mock("@/lib/mock/mode", () => ({
  isMockFeatureEnabled: () => false,
  isMockMode: () => false,
}));

vi.mock("@/lib/api/admin", () => ({
  listAuditLogs: vi.fn(),
  buildAuditExportUrl: vi.fn(),
  buildMockAuditExportCsv: vi.fn(),
}));

vi.mock("@/lib/api/system", () => ({
  getPublicSystemCapabilities: vi.fn(),
}));

vi.mock("@/lib/notifications/toast-store", () => ({
  toast: {
    success: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
    error: vi.fn(),
  },
}));

const mockListAuditLogs = vi.mocked(listAuditLogs);
const mockBuildAuditExportUrl = vi.mocked(buildAuditExportUrl);
const mockGetPublicSystemCapabilities = vi.mocked(getPublicSystemCapabilities);
const mockToast = vi.mocked(toast);

async function flushEffects() {
  await Promise.resolve();
  await Promise.resolve();
}

describe("AdminAuditLogsPanel", () => {
  let container: HTMLDivElement;
  let root: Root;
  let openSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    openSpy = vi.spyOn(window, "open").mockReturnValue({} as Window);
    mockGetPublicSystemCapabilities.mockResolvedValue({
      edition: "ee",
      strm_only_streaming: true,
      transcoding_enabled: false,
      billing_enabled: true,
      advanced_traffic_controls_enabled: true,
      invite_rewards_enabled: true,
      audit_log_export_enabled: true,
      request_agent_enabled: true,
      playback_routing_enabled: true,
      supported_stream_features: ["strm-direct-play"],
    });
    mockBuildAuditExportUrl.mockReturnValue("https://admin.local/audit/export?limit=200");
    mockListAuditLogs.mockResolvedValue([
      {
        id: "audit-001",
        actor_user_id: "admin",
        actor_username: "admin",
        action: "user.update",
        target_type: "user",
        target_id: "user-001",
        detail: { role: "Viewer" },
        created_at: "2026-02-16T00:00:00Z",
      },
    ]);
  });

  afterEach(() => {
    act(() => {
      root.unmount();
    });
    container.remove();
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = undefined;
    openSpy.mockRestore();
    vi.clearAllMocks();
  });

  it("shows user feedback after refresh and export actions", async () => {
    await act(async () => {
      root.render(<AdminAuditLogsPanel />);
      await flushEffects();
    });

    const refreshButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "刷新"
    );
    expect(refreshButton).not.toBeUndefined();

    await act(async () => {
      refreshButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(mockListAuditLogs).toHaveBeenCalledTimes(2);
    expect(mockToast.info).toHaveBeenCalledWith("已刷新审计日志（limit=200）");

    const exportButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "导出 CSV"
    );
    expect(exportButton).not.toBeUndefined();

    await act(async () => {
      exportButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(mockBuildAuditExportUrl).toHaveBeenCalledWith(200);
    expect(openSpy).toHaveBeenCalledWith(
      "https://admin.local/audit/export?limit=200&api_key=demo-token",
      "_blank",
      "noopener,noreferrer"
    );
    expect(mockToast.success).toHaveBeenCalledWith("已打开导出链接。");
  });
});
