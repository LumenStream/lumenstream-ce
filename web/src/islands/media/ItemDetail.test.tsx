/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ItemDetail } from "./ItemDetail";

const getUserItemMock = vi.fn();
const getItemSubtitlesMock = vi.fn();
const getPlaybackInfoMock = vi.fn();
const getShowSeasonsMock = vi.fn();
const getShowEpisodesMock = vi.fn();
const addFavoriteItemMock = vi.fn();
const removeFavoriteItemMock = vi.fn();
const updateItemMetadataMock = vi.fn();
const refreshItemMetadataMock = vi.fn();
const deleteItemMock = vi.fn();

const mockAuthState = {
  ready: true,
  session: {
    token: "token-1",
    user: {
      Id: "user-1",
      Name: "tester",
      Policy: { IsAdministrator: false, IsDisabled: false, Role: "Viewer" },
    },
  },
};

vi.mock("@/lib/auth/use-auth-session", () => ({
  useAuthSession: () => mockAuthState,
}));

vi.mock("@/lib/api/items", () => ({
  getUserItem: (...args: unknown[]) => getUserItemMock(...args),
  getItemSubtitles: (...args: unknown[]) => getItemSubtitlesMock(...args),
  getPlaybackInfo: (...args: unknown[]) => getPlaybackInfoMock(...args),
  getShowSeasons: (...args: unknown[]) => getShowSeasonsMock(...args),
  getShowEpisodes: (...args: unknown[]) => getShowEpisodesMock(...args),
  addFavoriteItem: (...args: unknown[]) => addFavoriteItemMock(...args),
  removeFavoriteItem: (...args: unknown[]) => removeFavoriteItemMock(...args),
  updateItemMetadata: (...args: unknown[]) => updateItemMetadataMock(...args),
  refreshItemMetadata: (...args: unknown[]) => refreshItemMetadataMock(...args),
  deleteItem: (...args: unknown[]) => deleteItemMock(...args),
  buildStreamUrl: (itemId: string) => `https://stream.example.com/${itemId}`,
  buildItemImageUrl: (itemId: string) => `https://image.example.com/${itemId}`,
  buildItemBackdropUrl: (itemId: string) => `https://backdrop.example.com/${itemId}`,
  buildPersonImageUrl: (personId: string) => `https://person.example.com/${personId}`,
}));

vi.mock("@/lib/player/deeplink", () => ({
  detectPlatform: vi.fn(() => "unknown"),
  getPlayersForPlatform: vi.fn(() => [
    { id: "vlc", name: "VLC", recommended: true, buildUrl: (s: string) => `vlc://${s}` },
    { id: "mpv", name: "MPV", recommended: false, buildUrl: (s: string) => `mpv://${s}` },
  ]),
}));

vi.mock("@/components/domain/Modal", () => ({
  Modal: ({
    open,
    title,
    children,
  }: {
    open: boolean;
    title: string;
    children: React.ReactNode;
  }) =>
    open
      ? React.createElement("div", { "data-testid": "modal", "data-title": title }, children)
      : null,
}));

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

function setFieldValue(target: HTMLInputElement | HTMLTextAreaElement, value: string) {
  const prototype = Object.getPrototypeOf(target);
  const descriptor = Object.getOwnPropertyDescriptor(prototype, "value");
  descriptor?.set?.call(target, value);
  target.dispatchEvent(new Event("input", { bubbles: true }));
  target.dispatchEvent(new Event("change", { bubbles: true }));
}

describe("ItemDetail", () => {
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

    getUserItemMock.mockResolvedValue({
      Id: "movie-1",
      Name: "教父2",
      Type: "Movie",
      Path: "/media/godfather2.strm",
      ProductionYear: 1974,
      CommunityRating: 8.6,
      RunTimeTicks: 10_800_000_000,
      Bitrate: 8_200_000,
      Overview: "家族史诗",
      UserData: {
        Played: false,
        PlaybackPositionTicks: 0,
        IsFavorite: false,
      },
      People: [
        {
          Name: "Al Pacino",
          Id: "person-actor-1",
          Role: "Michael Corleone",
          Type: "Actor",
        },
        {
          Name: "Francis Ford Coppola",
          Id: "person-director-1",
          Role: "Director",
          Type: "Director",
        },
      ],
    });

    getItemSubtitlesMock.mockResolvedValue([
      {
        Index: 1,
        Codec: "ass",
        Language: "zho",
        DisplayTitle: "ZHO (ASS)",
        IsExternal: true,
        IsDefault: true,
      },
    ]);

    getPlaybackInfoMock.mockResolvedValue({
      MediaSources: [
        {
          Id: "ms-1",
          Path: "/media/godfather2.strm",
          Protocol: "File",
          Container: "mkv",
          RunTimeTicks: 10_800_000_000,
          Bitrate: 8_200_000,
          SupportsDirectPlay: true,
          SupportsDirectStream: true,
          SupportsTranscoding: false,
          MediaStreams: [
            {
              Index: 0,
              Type: "Video",
              Language: null,
              IsExternal: false,
              Path: null,
              Codec: "h264",
            },
            {
              Index: 1,
              Type: "Audio",
              Language: "eng",
              IsExternal: false,
              Path: null,
              Codec: "aac",
              Channels: 2,
              BitRate: 192000,
              IsDefault: true,
            },
            {
              Index: 2,
              Type: "Subtitle",
              Language: "zho",
              IsExternal: true,
              Path: "/media/sub.zh.ass",
              Codec: "ass",
              DisplayTitle: "ZHO (ASS)",
              IsDefault: true,
            },
          ],
        },
      ],
      PlaySessionId: "session-1",
    });

    getShowSeasonsMock.mockResolvedValue({ Items: [], TotalRecordCount: 0, StartIndex: 0 });
    getShowEpisodesMock.mockResolvedValue({ Items: [], TotalRecordCount: 0, StartIndex: 0 });
    addFavoriteItemMock.mockResolvedValue({
      PlaybackPositionTicks: 0,
      PlayCount: 0,
      IsFavorite: true,
      Played: false,
      ItemId: "movie-1",
    });
    removeFavoriteItemMock.mockResolvedValue({
      PlaybackPositionTicks: 0,
      PlayCount: 0,
      IsFavorite: false,
      Played: false,
      ItemId: "movie-1",
    });
    updateItemMetadataMock.mockResolvedValue(undefined);
    refreshItemMetadataMock.mockResolvedValue(undefined);
    deleteItemMock.mockResolvedValue(undefined);
    mockAuthState.session.user.Policy = {
      IsAdministrator: false,
      IsDisabled: false,
      Role: "Viewer",
    };
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

  it("renders hero, cast and technical stream sections", async () => {
    await act(async () => {
      root.render(<ItemDetail itemId="movie-1" />);
      await flushEffects();
    });

    const text = container.textContent || "";
    expect(text).toContain("教父2");
    expect(text).toContain("媒体技术信息");
    expect(text).toContain("演职员");
    expect(text).toContain("Al Pacino");
    expect(text).toContain("Francis Ford Coppola");
    expect(text).toContain("AAC");

    // Expand collapsible tech section to verify audio/subtitle details
    const techToggle = queryButtonByText(container, "媒体技术信息");
    expect(techToggle).not.toBeNull();
    await act(async () => {
      techToggle?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    const expandedText = container.textContent || "";
    expect(expandedText).toContain("音频轨");
    expect(expandedText).toContain("ZHO (ASS)");

    const actorLink = container.querySelector("a[href='/app/person/person-actor-1']");
    const directorLink = container.querySelector("a[href='/app/person/person-director-1']");
    expect(actorLink).not.toBeNull();
    expect(directorLink).not.toBeNull();
  });

  it("toggles favorite state via API", async () => {
    await act(async () => {
      root.render(<ItemDetail itemId="movie-1" />);
      await flushEffects();
    });

    const favoriteButton = queryButtonByText(container, "收藏");
    expect(favoriteButton).not.toBeNull();

    await act(async () => {
      favoriteButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(addFavoriteItemMock).toHaveBeenCalledWith("user-1", "movie-1");
  });

  it("opens player picker modal when play button is clicked", async () => {
    await act(async () => {
      root.render(<ItemDetail itemId="movie-1" />);
      await flushEffects();
    });

    // No modal initially
    expect(document.querySelector("[data-title='选择播放器']")).toBeNull();

    const playButton = queryButtonByText(container, "播放");
    expect(playButton).not.toBeNull();

    await act(async () => {
      playButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    // Modal should now be open
    const modal = document.querySelector("[data-title='选择播放器']");
    expect(modal).not.toBeNull();
  });

  it("shows admin modal and submits metadata update for admin users", async () => {
    mockAuthState.session.user.Policy = {
      IsAdministrator: true,
      IsDisabled: false,
      Role: "Admin",
    };

    await act(async () => {
      root.render(<ItemDetail itemId="movie-1" />);
      await flushEffects();
    });

    const adminButton = container.querySelector("button[aria-label='管理员编辑']");
    expect(adminButton).not.toBeNull();

    await act(async () => {
      adminButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    const modal = document.querySelector("[data-title='管理员媒体操作']");
    expect(modal).not.toBeNull();

    const inputs = Array.from(modal?.querySelectorAll("input") || []);
    expect(inputs.length).toBeGreaterThanOrEqual(4);
    const titleInput = inputs[0]!;
    const yearInput = inputs[1]!;
    const tmdbInput = inputs[2]!;
    const imdbInput = inputs[3]!;
    const overviewInput = modal?.querySelector("textarea") as HTMLTextAreaElement | null;
    expect(overviewInput).not.toBeNull();

    await act(async () => {
      setFieldValue(titleInput, "星际穿越");
      setFieldValue(yearInput, "2014");
      setFieldValue(tmdbInput, "157336");
      setFieldValue(imdbInput, "tt0816692");
      if (overviewInput) {
        setFieldValue(overviewInput, "一场太空远航");
      }
      await flushEffects();
    });

    const saveButton = queryButtonByText(container, "保存元数据");
    expect(saveButton).not.toBeNull();
    await act(async () => {
      saveButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(updateItemMetadataMock).toHaveBeenCalledWith(
      "movie-1",
      expect.objectContaining({
        Name: "星际穿越",
        ProductionYear: 2014,
        TmdbId: "157336",
        ImdbId: "tt0816692",
      })
    );
  });

  it("triggers rescrape and delete actions from admin modal", async () => {
    mockAuthState.session.user.Policy = {
      IsAdministrator: true,
      IsDisabled: false,
      Role: "Admin",
    };

    await act(async () => {
      root.render(<ItemDetail itemId="movie-1" />);
      await flushEffects();
    });

    const adminButton = container.querySelector("button[aria-label='管理员编辑']");
    expect(adminButton).not.toBeNull();
    await act(async () => {
      adminButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    const rescrapeButton = queryButtonByText(container, "重新刮削");
    expect(rescrapeButton).not.toBeNull();
    await act(async () => {
      rescrapeButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });
    expect(refreshItemMetadataMock).toHaveBeenCalledWith(
      "movie-1",
      expect.objectContaining({ replaceAllMetadata: true })
    );

    const modal = document.querySelector("[data-title='管理员媒体操作']");
    const inputs = Array.from(modal?.querySelectorAll("input") || []);
    const deleteConfirmInput = inputs.at(-1) as HTMLInputElement | undefined;
    expect(deleteConfirmInput).toBeDefined();
    await act(async () => {
      if (deleteConfirmInput) {
        setFieldValue(deleteConfirmInput, "教父2");
      }
      await flushEffects();
    });

    const deleteButton = queryButtonByText(container, "删除媒体");
    expect(deleteButton).not.toBeNull();
    await act(async () => {
      deleteButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });
    expect(deleteItemMock).toHaveBeenCalledWith("movie-1");
  });
});
