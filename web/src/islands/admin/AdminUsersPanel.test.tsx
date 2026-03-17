/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { deleteUser, getAdminUserProfile, listUserSummaries } from "@/lib/api/admin";
import { adminGetUserLedger, adminGetUserWallet, adminListPlans } from "@/lib/api/admin-commercial";
import { getPublicSystemCapabilities } from "@/lib/api/system";

import { AdminUsersPanel } from "./AdminUsersPanel";

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

vi.mock("@/lib/api/admin", () => ({
  listUserSummaries: vi.fn(),
  getAdminUserProfile: vi.fn(),
  createUser: vi.fn(),
  batchSetUserEnabled: vi.fn(),
  patchUserProfile: vi.fn(),
  deleteUser: vi.fn(),
}));

vi.mock("@/lib/api/admin-commercial", () => ({
  adminListPlans: vi.fn(),
  setUserStreamPolicy: vi.fn(),
  resetUserTrafficUsage: vi.fn(),
  adminAdjustBalance: vi.fn(),
  adminAssignSubscription: vi.fn(),
  adminCancelSubscription: vi.fn(),
  adminUpdateSubscription: vi.fn(),
  adminGetUserWallet: vi.fn(),
  adminGetUserLedger: vi.fn(),
}));

vi.mock("@/lib/api/system", () => ({
  getPublicSystemCapabilities: vi.fn(),
}));

const mockAdminListPlans = vi.mocked(adminListPlans);
const mockListUserSummaries = vi.mocked(listUserSummaries);
const mockGetAdminUserProfile = vi.mocked(getAdminUserProfile);
const mockAdminGetUserWallet = vi.mocked(adminGetUserWallet);
const mockAdminGetUserLedger = vi.mocked(adminGetUserLedger);
const mockDeleteUser = vi.mocked(deleteUser);
const mockGetPublicSystemCapabilities = vi.mocked(getPublicSystemCapabilities);

async function flushEffects() {
  for (let i = 0; i < 5; i++) await Promise.resolve();
}

describe("AdminUsersPanel", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    mockAdminListPlans.mockResolvedValue([
      {
        id: "plan-standard",
        code: "standard",
        name: "标准套餐",
        price: "29.00",
        duration_days: 30,
        traffic_quota_bytes: 10 * 1024 * 1024 * 1024,
        traffic_window_days: 30,
        enabled: true,
        updated_at: "2026-02-16T00:00:00Z",
      },
    ]);
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
    mockListUserSummaries.mockResolvedValue({
      page: 1,
      page_size: 20,
      total: 1,
      items: [
        {
          id: "user-viewer-001",
          username: "demo-viewer",
          email: "viewer@lumenstream.local",
          display_name: "普通用户",
          role: "Viewer",
          is_admin: false,
          is_disabled: true,
          active_auth_sessions: 0,
          active_playback_sessions: 0,
          subscription_name: null,
          used_bytes: 1_073_741_824,
          created_at: "2026-02-16T00:00:00Z",
        },
      ],
    });
    mockGetAdminUserProfile.mockResolvedValue({
      user: {
        Id: "user-viewer-001",
        Name: "demo-viewer",
        HasPassword: true,
        ServerId: "lumenstream-test",
        Policy: {
          IsAdministrator: false,
          IsDisabled: true,
          Role: "Viewer",
        },
      },
      profile: {
        user_id: "user-viewer-001",
        email: "viewer@lumenstream.local",
        display_name: "普通用户",
        remark: null,
        created_at: "2026-02-16T00:00:00Z",
        updated_at: "2026-02-16T00:00:00Z",
      },
      stream_policy: {
        user_id: "user-viewer-001",
        expires_at: null,
        max_concurrent_streams: 1,
        traffic_quota_bytes: 10 * 1024 * 1024 * 1024,
        traffic_window_days: 30,
        updated_at: "2026-02-16T00:00:00Z",
      },
      traffic_usage: {
        user_id: "user-viewer-001",
        window_days: 30,
        used_bytes: 1_073_741_824,
        quota_bytes: 10 * 1024 * 1024 * 1024,
        remaining_bytes: 8_926_258_176,
        daily: [],
      },
      wallet: {
        user_id: "user-viewer-001",
        balance: "0.00",
        total_recharged: "0.00",
        total_spent: "0.00",
        updated_at: "2026-02-16T00:00:00Z",
      },
      subscriptions: [],
      sessions_summary: {
        active_auth_sessions: 0,
        active_playback_sessions: 0,
        last_auth_seen_at: null,
        last_playback_seen_at: null,
      },
    });
    mockAdminGetUserWallet.mockResolvedValue({
      user_id: "user-viewer-001",
      balance: "0.00",
      total_recharged: "0.00",
      total_spent: "0.00",
      updated_at: "2026-02-16T00:00:00Z",
    });
    mockAdminGetUserLedger.mockResolvedValue([]);
    mockDeleteUser.mockResolvedValue(undefined);
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

  it("opens the user management modal when clicking details action", async () => {
    await act(async () => {
      root.render(<AdminUsersPanel />);
      await flushEffects();
    });

    const row = container.querySelector("tbody tr");
    expect(row).not.toBeNull();

    await act(async () => {
      row?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(mockGetAdminUserProfile).toHaveBeenCalledWith("user-viewer-001");
    expect(document.body.textContent).toContain("demo-viewer");
    expect(document.body.textContent).toContain("资料");
  });

  it("allows delete confirmation with trimmed username input", async () => {
    await act(async () => {
      root.render(<AdminUsersPanel />);
      await flushEffects();
    });

    const row = container.querySelector("tbody tr");
    expect(row).not.toBeNull();

    await act(async () => {
      row?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    const dangerTab = Array.from(document.body.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "危险"
    );
    expect(dangerTab).not.toBeUndefined();

    await act(async () => {
      dangerTab?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    const deleteButton = Array.from(document.body.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "删除用户"
    );
    expect(deleteButton).not.toBeUndefined();

    await act(async () => {
      deleteButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    const confirmInput = Array.from(document.body.querySelectorAll("input")).find((input) =>
      (input as HTMLInputElement).placeholder.includes("demo-viewer")
    ) as HTMLInputElement | undefined;
    const confirmDeleteButton = Array.from(document.body.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "确认删除"
    ) as HTMLButtonElement | undefined;

    expect(confirmInput).not.toBeUndefined();
    expect(confirmDeleteButton).not.toBeUndefined();
    expect(confirmDeleteButton?.disabled).toBe(true);

    await act(async () => {
      if (confirmInput) {
        const nativeSetter = Object.getOwnPropertyDescriptor(
          HTMLInputElement.prototype,
          "value"
        )!.set!;
        nativeSetter.call(confirmInput, "  demo-viewer  ");
        confirmInput.dispatchEvent(new Event("input", { bubbles: true }));
      }
      await flushEffects();
    });

    expect(confirmDeleteButton?.disabled).toBe(false);

    await act(async () => {
      confirmDeleteButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(mockDeleteUser).toHaveBeenCalledWith("user-viewer-001");
  });
});
