/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { PlaylistsCenter } from "./PlaylistsCenter";

const listMyPlaylistsMock = vi.fn();
const listPlaylistItemsMock = vi.fn();
const createPlaylistMock = vi.fn();
const updatePlaylistMock = vi.fn();
const deletePlaylistMock = vi.fn();
const mockSession = {
  token: "token-1",
  user: { Id: "user-1", Name: "tester" },
};
const mockAuthState = {
  ready: true,
  session: mockSession,
};

vi.mock("@/lib/auth/use-auth-session", () => ({
  useAuthSession: () => mockAuthState,
}));

vi.mock("@/lib/api/playlists", () => ({
  listMyPlaylists: (...args: unknown[]) => listMyPlaylistsMock(...args),
  listPlaylistItems: (...args: unknown[]) => listPlaylistItemsMock(...args),
  createPlaylist: (...args: unknown[]) => createPlaylistMock(...args),
  updatePlaylist: (...args: unknown[]) => updatePlaylistMock(...args),
  deletePlaylist: (...args: unknown[]) => deletePlaylistMock(...args),
}));

vi.mock("@/lib/notifications/toast-store", () => ({
  toast: {
    success: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
    error: vi.fn(),
  },
}));

vi.mock("@/components/domain/DataState", () => ({
  LoadingState: () => React.createElement("div", null, "loading"),
  ErrorState: () => React.createElement("div", null, "error"),
  EmptyState: () => React.createElement("div", null, "empty"),
}));

const playlist = {
  id: "playlist-1",
  owner_user_id: "user-1",
  name: "旧收藏夹",
  description: "旧描述",
  is_public: false,
  is_default: false,
  item_count: 0,
  created_at: "2026-02-15T00:00:00Z",
  updated_at: "2026-02-15T00:00:00Z",
};

function queryButtonByText(container: HTMLDivElement, text: string): HTMLButtonElement | null {
  return (
    Array.from(container.querySelectorAll("button")).find((button) =>
      button.textContent?.includes(text)
    ) || null
  );
}

async function flushEffects() {
  await Promise.resolve();
  await Promise.resolve();
}

function setNativeValue(element: HTMLInputElement | HTMLTextAreaElement, value: string) {
  const prototype =
    element instanceof HTMLTextAreaElement
      ? HTMLTextAreaElement.prototype
      : HTMLInputElement.prototype;
  const setter = Object.getOwnPropertyDescriptor(prototype, "value")?.set;
  setter?.call(element, value);
}

describe("PlaylistsCenter", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    listMyPlaylistsMock.mockResolvedValue([playlist]);
    listPlaylistItemsMock.mockResolvedValue({ items: [], total: 0 });
    updatePlaylistMock.mockResolvedValue({
      ...playlist,
      name: "新收藏夹",
      description: "新描述",
      updated_at: "2026-02-15T01:00:00Z",
    });
    createPlaylistMock.mockResolvedValue({
      id: "playlist-2",
      owner_user_id: "user-1",
      name: "旅行片单",
      description: "",
      is_public: false,
      is_default: false,
      item_count: 0,
      created_at: "2026-02-15T02:00:00Z",
      updated_at: "2026-02-15T02:00:00Z",
    });
    deletePlaylistMock.mockResolvedValue({ deleted: true });
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

  it("shows a visible delete action in playlist editor", async () => {
    await act(async () => {
      root.render(<PlaylistsCenter />);
      await flushEffects();
    });

    const deleteButton = queryButtonByText(container, "删除");
    expect(deleteButton).not.toBeNull();
  });

  it("supports renaming playlist from playlist center", async () => {
    await act(async () => {
      root.render(<PlaylistsCenter />);
      await flushEffects();
    });

    // Click "重命名" to enter editing mode
    const renameButton = queryButtonByText(container, "重命名");
    expect(renameButton).not.toBeNull();

    await act(async () => {
      renameButton!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    // Now the inline edit input should be visible
    const nameInput =
      (Array.from(container.querySelectorAll("input")).find(
        (candidate) => (candidate as HTMLInputElement).value === "旧收藏夹"
      ) as HTMLInputElement | undefined) || null;
    const saveButton = queryButtonByText(container, "保存");

    expect(nameInput).not.toBeNull();
    expect(saveButton).not.toBeNull();

    await act(async () => {
      if (!nameInput || !saveButton) {
        return;
      }
      setNativeValue(nameInput, "新收藏夹");
      nameInput.dispatchEvent(new Event("input", { bubbles: true }));
      saveButton.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(updatePlaylistMock).toHaveBeenCalledWith("playlist-1", {
      name: "新收藏夹",
    });
  });

  it("creates playlist from playlist center", async () => {
    await act(async () => {
      root.render(<PlaylistsCenter />);
      await flushEffects();
    });

    const createInput = container.querySelector(
      "input[placeholder='新建收藏夹名称']"
    ) as HTMLInputElement | null;
    const createButton = queryButtonByText(container, "新建收藏夹");

    expect(createInput).not.toBeNull();
    expect(createButton).not.toBeNull();

    await act(async () => {
      if (!createInput || !createButton) {
        return;
      }
      setNativeValue(createInput, "旅行片单");
      createInput.dispatchEvent(new Event("input", { bubbles: true }));
      createButton.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(createPlaylistMock).toHaveBeenCalledWith({ name: "旅行片单" });
    expect(container.textContent).toContain("旅行片单");
  });
});
