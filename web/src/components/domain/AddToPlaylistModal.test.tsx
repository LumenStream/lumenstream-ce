/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { addItemToPlaylist, createPlaylist, listMyPlaylists } from "@/lib/api/playlists";
import { toast } from "@/lib/notifications/toast-store";
import type { BaseItem } from "@/lib/types/jellyfin";
import type { Playlist } from "@/lib/types/playlist";

import { AddToPlaylistModal } from "./AddToPlaylistModal";

vi.mock("@/lib/api/playlists", () => ({
  listMyPlaylists: vi.fn(),
  createPlaylist: vi.fn(),
  addItemToPlaylist: vi.fn(),
}));

vi.mock("@/lib/notifications/toast-store", () => ({
  toast: {
    success: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
    error: vi.fn(),
  },
}));

const mockListMyPlaylists = vi.mocked(listMyPlaylists);
const mockCreatePlaylist = vi.mocked(createPlaylist);
const mockAddItemToPlaylist = vi.mocked(addItemToPlaylist);
const mockToast = vi.mocked(toast);

const testItem: BaseItem = {
  Id: "item-1",
  Name: "Test Movie",
  Type: "Movie",
  Path: "/media/test/movie.mkv",
};

const testPlaylists: Playlist[] = [
  {
    id: "playlist-1",
    owner_user_id: "user-1",
    name: "稍后再看",
    description: "",
    is_public: false,
    is_default: true,
    item_count: 12,
    created_at: "2026-02-15T12:00:00Z",
    updated_at: "2026-02-15T12:00:00Z",
  },
  {
    id: "playlist-2",
    owner_user_id: "user-1",
    name: "经典收藏",
    description: "",
    is_public: false,
    is_default: false,
    item_count: 8,
    created_at: "2026-02-15T12:00:00Z",
    updated_at: "2026-02-15T12:00:00Z",
  },
];

function getButtonByText(text: string): HTMLButtonElement {
  const button = Array.from(document.body.querySelectorAll("button")).find(
    (candidate) => candidate.textContent?.trim() === text
  ) as HTMLButtonElement | undefined;
  if (!button) {
    throw new Error(`Expected button with text: ${text}`);
  }
  return button;
}

function setNativeValue(element: HTMLInputElement | HTMLTextAreaElement, value: string) {
  const prototype =
    element instanceof HTMLTextAreaElement
      ? HTMLTextAreaElement.prototype
      : HTMLInputElement.prototype;
  const setter = Object.getOwnPropertyDescriptor(prototype, "value")?.set;
  setter?.call(element, value);
}

describe("AddToPlaylistModal", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);
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

  it("renders playlist checklist and create controls", async () => {
    mockListMyPlaylists.mockResolvedValueOnce(testPlaylists);

    await act(async () => {
      root.render(<AddToPlaylistModal open item={testItem} onClose={() => undefined} />);
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(document.body.textContent).toContain("添加到收藏夹");
    expect(document.body.textContent).toContain("选择要添加的收藏夹。");
    expect(document.body.textContent).not.toContain(testItem.Name);
    expect(document.body.textContent).toContain("稍后再看");
    expect(document.body.textContent).toContain("经典收藏");
    expect(document.body.textContent).toContain("12 项");
    expect(document.body.textContent).toContain("新建收藏夹并添加当前条目");
    expect(document.body.textContent).toContain("新建并添加");

    const overlay = document.body.querySelector("div.fixed.inset-0") as HTMLDivElement | null;
    expect(overlay?.className).toContain("bg-black/50");

    const modalCard = document.body.querySelector("[role='dialog'] > div") as HTMLDivElement | null;
    expect(modalCard?.className).toContain("h-[30rem]");
    expect(modalCard?.className).toContain("w-[30rem]");

    const truncatedName = document.body.querySelector(
      "span.truncate.text-sm.text-white\\/95"
    ) as HTMLSpanElement | null;
    expect(truncatedName).not.toBeNull();

    const checkboxes = document.body.querySelectorAll("input[type='checkbox']");
    expect(checkboxes.length).toBe(2);
  });

  it("creates a new playlist and adds current item", async () => {
    const onClose = vi.fn();
    mockListMyPlaylists.mockResolvedValueOnce(testPlaylists);
    mockCreatePlaylist.mockResolvedValueOnce({
      id: "playlist-3",
      owner_user_id: "user-1",
      name: "旅行片单",
      description: "",
      is_public: false,
      is_default: false,
      item_count: 0,
      created_at: "2026-02-16T12:00:00Z",
      updated_at: "2026-02-16T12:00:00Z",
    });
    mockAddItemToPlaylist.mockResolvedValueOnce({
      playlist_id: "playlist-3",
      media_item_id: testItem.Id,
      added_at: "2026-02-16T12:00:01Z",
    });

    await act(async () => {
      root.render(<AddToPlaylistModal open item={testItem} onClose={onClose} />);
      await Promise.resolve();
      await Promise.resolve();
    });

    const createInput = document.body.querySelector(
      "input[placeholder='新建收藏夹名称']"
    ) as HTMLInputElement | null;
    const createButton = getButtonByText("新建并添加");
    expect(createInput).not.toBeNull();
    expect(createButton.disabled).toBe(true);

    await act(async () => {
      if (!createInput) return;
      setNativeValue(createInput, "旅行片单");
      createInput.dispatchEvent(new Event("input", { bubbles: true }));
    });

    expect(createButton.disabled).toBe(false);

    await act(async () => {
      createButton.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(mockCreatePlaylist).toHaveBeenCalledWith({ name: "旅行片单" });
    expect(mockAddItemToPlaylist).toHaveBeenCalledWith("playlist-3", "item-1");
    expect(mockToast.success).toHaveBeenCalledWith("已新建收藏夹并添加");
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("adds the item to all selected playlists and closes on success", async () => {
    const onClose = vi.fn();
    mockListMyPlaylists.mockResolvedValueOnce(testPlaylists);
    mockAddItemToPlaylist.mockResolvedValue({
      playlist_id: "playlist-1",
      media_item_id: testItem.Id,
      added_at: "2026-02-15T12:00:00Z",
    });

    await act(async () => {
      root.render(<AddToPlaylistModal open item={testItem} onClose={onClose} />);
      await Promise.resolve();
      await Promise.resolve();
    });

    const checkboxes = Array.from(
      document.body.querySelectorAll("input[type='checkbox']")
    ) as HTMLInputElement[];
    const confirmButton = getButtonByText("确定");
    expect(confirmButton.disabled).toBe(true);

    await act(async () => {
      checkboxes[0]?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      checkboxes[1]?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(confirmButton.disabled).toBe(false);

    await act(async () => {
      confirmButton.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(mockAddItemToPlaylist).toHaveBeenCalledTimes(2);
    expect(mockAddItemToPlaylist).toHaveBeenNthCalledWith(1, "playlist-1", "item-1");
    expect(mockAddItemToPlaylist).toHaveBeenNthCalledWith(2, "playlist-2", "item-1");
    expect(mockToast.success).toHaveBeenCalledWith("已添加到 2 个收藏夹");
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
