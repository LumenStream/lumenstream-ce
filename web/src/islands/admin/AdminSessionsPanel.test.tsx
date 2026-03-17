/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { listAuthSessions, listPlaybackSessions } from "@/lib/api/admin";
import type { PlaybackSession } from "@/lib/types/admin";

import { AdminSessionsPanel } from "./AdminSessionsPanel";

const mockAuthState = {
  ready: true,
};

vi.mock("@/lib/auth/use-auth-session", () => ({
  useAuthSession: () => mockAuthState,
}));

vi.mock("@/components/domain/DataState", () => ({
  LoadingState: ({ title }: { title: string }) => React.createElement("div", null, title),
  ErrorState: ({ title, description }: { title: string; description?: string }) =>
    React.createElement("div", null, `${title}${description ? `: ${description}` : ""}`),
}));

vi.mock("@/lib/api/admin", () => ({
  listPlaybackSessions: vi.fn(),
  listAuthSessions: vi.fn(),
}));

const mockListPlaybackSessions = vi.mocked(listPlaybackSessions);
const mockListAuthSessions = vi.mocked(listAuthSessions);

async function flushEffects() {
  for (let i = 0; i < 5; i += 1) {
    await Promise.resolve();
  }
}

describe("AdminSessionsPanel", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    (globalThis as typeof globalThis & { React?: typeof React }).React = React;
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);
    mockListPlaybackSessions.mockResolvedValue([]);
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

  it("loads only active playback media sessions and does not render auth sessions", async () => {
    const sessions: PlaybackSession[] = [
      {
        id: "play-1",
        play_session_id: "ps-1",
        user_id: "u-1",
        user_name: "demo",
        media_item_id: "movie-1",
        media_item_name: "正在播放电影",
        device_name: "VidHub",
        client_name: "VidHub",
        play_method: "DirectPlay",
        position_ticks: 1200,
        is_active: true,
        last_heartbeat_at: "2026-02-24T20:00:00Z",
        updated_at: "2026-02-24T20:00:00Z",
      },
      {
        id: "play-2",
        play_session_id: "ps-2",
        user_id: "u-1",
        user_name: "demo",
        media_item_id: "movie-2",
        media_item_name: "已停止电影",
        device_name: "Infuse",
        client_name: "Infuse-Direct",
        play_method: "DirectStream",
        position_ticks: 2400,
        is_active: false,
        last_heartbeat_at: "2026-02-24T20:00:00Z",
        updated_at: "2026-02-24T20:00:00Z",
      },
      {
        id: "play-3",
        play_session_id: "ps-3",
        user_id: "u-1",
        user_name: "demo",
        media_item_id: null,
        media_item_name: null,
        device_name: "SenPlayer",
        client_name: "SenPlayer",
        play_method: "DirectPlay",
        position_ticks: 3600,
        is_active: true,
        last_heartbeat_at: "2026-02-24T20:00:00Z",
        updated_at: "2026-02-24T20:00:00Z",
      },
    ];
    mockListPlaybackSessions.mockResolvedValue(sessions);

    await act(async () => {
      root.render(<AdminSessionsPanel />);
      await flushEffects();
    });

    expect(mockListPlaybackSessions).toHaveBeenCalledWith({ limit: 80, active_only: true });
    expect(mockListAuthSessions).not.toHaveBeenCalled();

    expect(container.textContent).toContain("播放会话");
    expect(container.textContent).not.toContain("鉴权会话");
    expect(container.textContent).toContain("正在播放电影");
    expect(container.textContent).toContain("VidHub");
    expect(container.textContent).not.toContain("已停止电影");
    expect(container.textContent).not.toContain("SenPlayer");
  });

  it("shows empty state when there are no currently playing sessions", async () => {
    mockListPlaybackSessions.mockResolvedValue([]);

    await act(async () => {
      root.render(<AdminSessionsPanel />);
      await flushEffects();
    });

    expect(container.textContent).toContain("暂无正在播放会话");
  });
});
