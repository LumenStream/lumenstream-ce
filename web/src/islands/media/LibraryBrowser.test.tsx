/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { LibraryBrowser } from "./LibraryBrowser";

const getUserItemMock = vi.fn();
const getUserItemsMock = vi.fn();
const getShowSeasonsMock = vi.fn();
const getShowEpisodesMock = vi.fn();
const addFavoriteItemMock = vi.fn();
const removeFavoriteItemMock = vi.fn();
const toastSuccessMock = vi.fn();
const toastInfoMock = vi.fn();
const toastErrorMock = vi.fn();

const mockAuthState = {
  ready: true,
  session: {
    token: "token-1",
    user: { Id: "user-1", Name: "tester" },
  },
};

vi.mock("@/lib/auth/use-auth-session", () => ({
  useAuthSession: () => mockAuthState,
}));

vi.mock("@/lib/api/items", () => ({
  getUserItem: (...args: unknown[]) => getUserItemMock(...args),
  getUserItems: (...args: unknown[]) => getUserItemsMock(...args),
  getShowSeasons: (...args: unknown[]) => getShowSeasonsMock(...args),
  getShowEpisodes: (...args: unknown[]) => getShowEpisodesMock(...args),
  addFavoriteItem: (...args: unknown[]) => addFavoriteItemMock(...args),
  removeFavoriteItem: (...args: unknown[]) => removeFavoriteItemMock(...args),
  buildItemImageUrl: (itemId: string) => `https://image.example.com/${itemId}`,
  buildItemBackdropUrl: (itemId: string) => `https://backdrop.example.com/${itemId}`,
}));

vi.mock("@/components/domain/PosterItemCard", () => ({
  PosterItemCard: ({ item, href }: { item: { Id: string; Name?: string }; href: string }) =>
    React.createElement("a", { href, "data-testid": `poster-${item.Id}` }, item.Name || item.Id),
}));

vi.mock("@/components/domain/DataState", () => ({
  EmptyState: ({ title }: { title: string }) => React.createElement("div", null, title),
  ErrorState: ({ title }: { title: string }) => React.createElement("div", null, title),
  LoadingState: ({ title }: { title?: string }) => React.createElement("div", null, title || ""),
}));

vi.mock("@/components/domain/AddToPlaylistModal", () => ({
  AddToPlaylistModal: ({ open, item }: { open: boolean; item: { Id: string } }) =>
    open
      ? React.createElement(
          "div",
          { "data-testid": "add-to-playlist-modal" },
          `playlist-${item.Id}`
        )
      : null,
}));

vi.mock("@/lib/notifications/toast-store", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    info: (...args: unknown[]) => toastInfoMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

async function flushEffects() {
  for (let i = 0; i < 8; i += 1) {
    await Promise.resolve();
  }
}

describe("LibraryBrowser", () => {
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
      Id: "root-high-score",
      Name: "高分佳作",
      Type: "CollectionFolder",
      Path: "/media/movies/high-score",
    });
    getUserItemsMock.mockResolvedValue({ Items: [], TotalRecordCount: 0, StartIndex: 0 });
    getShowSeasonsMock.mockResolvedValue({ Items: [], TotalRecordCount: 0, StartIndex: 0 });
    getShowEpisodesMock.mockResolvedValue({ Items: [], TotalRecordCount: 0, StartIndex: 0 });
    addFavoriteItemMock.mockResolvedValue({
      PlaybackPositionTicks: 0,
      PlayCount: 0,
      IsFavorite: true,
      Played: false,
      ItemId: "series-001",
    });
    removeFavoriteItemMock.mockResolvedValue({
      PlaybackPositionTicks: 0,
      PlayCount: 0,
      IsFavorite: false,
      Played: false,
      ItemId: "series-001",
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

  it("renders two-level directory for series and navigates episodes to item detail", async () => {
    getUserItemMock.mockResolvedValue({
      Id: "series-001",
      Name: "The Last of Us",
      Type: "Series",
      Path: "/media/shows/the-last-of-us",
    });

    getShowSeasonsMock.mockResolvedValue({
      Items: [
        { Id: "season-001", Name: "第 1 季", SeriesId: "series-001", IndexNumber: 1 },
        { Id: "season-002", Name: "第 2 季", SeriesId: "series-001", IndexNumber: 2 },
      ],
      TotalRecordCount: 2,
      StartIndex: 0,
    });

    getShowEpisodesMock.mockImplementation(
      async (
        _showId: string,
        query?: {
          seasonId?: string;
        }
      ) => {
        if (query?.seasonId === "season-001") {
          return {
            Items: [
              {
                Id: "episode-001",
                Name: "第一集",
                Type: "Episode",
                Path: "/media/shows/the-last-of-us/s01e01.strm",
                IndexNumber: 1,
                RunTimeTicks: 3_600_000_000,
                UserData: { Played: false, PlaybackPositionTicks: 0 },
              },
              {
                Id: "episode-002",
                Name: "第二集",
                Type: "Episode",
                Path: "/media/shows/the-last-of-us/s01e02.strm",
                IndexNumber: 2,
                RunTimeTicks: 3_500_000_000,
                UserData: { Played: false, PlaybackPositionTicks: 0 },
              },
            ],
            TotalRecordCount: 2,
            StartIndex: 0,
          };
        }

        if (query?.seasonId === "season-002") {
          return {
            Items: [
              {
                Id: "episode-003",
                Name: "第三集",
                Type: "Episode",
                Path: "/media/shows/the-last-of-us/s02e01.strm",
                IndexNumber: 1,
                RunTimeTicks: 3_300_000_000,
                UserData: { Played: true, PlaybackPositionTicks: 0 },
              },
            ],
            TotalRecordCount: 1,
            StartIndex: 0,
          };
        }

        return { Items: [], TotalRecordCount: 0, StartIndex: 0 };
      }
    );

    await act(async () => {
      root.render(<LibraryBrowser parentId="series-001" />);
      await flushEffects();
    });

    expect(container.textContent || "").toContain("季目录");
    expect(container.textContent || "").toContain("第 1 季");
    expect(container.textContent || "").toContain("第 2 季");
    expect(getShowSeasonsMock).toHaveBeenCalledWith("series-001");

    const episodeLinkBeforeSort = container.querySelector(
      "a[href='/app/item/episode-001']"
    ) as HTMLAnchorElement | null;
    expect(episodeLinkBeforeSort).not.toBeNull();

    const favoriteButton = Array.from(container.querySelectorAll("button")).find((button) =>
      button.textContent?.includes("喜欢")
    );
    expect(favoriteButton).not.toBeNull();

    await act(async () => {
      favoriteButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(addFavoriteItemMock).toHaveBeenCalledWith("user-1", "series-001");
    expect(toastSuccessMock).toHaveBeenCalledWith("已加入喜欢");

    const addToPlaylistButton = Array.from(container.querySelectorAll("button")).find((button) =>
      button.textContent?.includes("加入收藏列表")
    );
    expect(addToPlaylistButton).not.toBeNull();

    await act(async () => {
      addToPlaylistButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(container.querySelector("[data-testid='add-to-playlist-modal']")).not.toBeNull();

    const descButton = Array.from(container.querySelectorAll("button")).find((button) =>
      button.textContent?.includes("倒序")
    );
    expect(descButton).not.toBeNull();

    await act(async () => {
      descButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    const orderedLinksAfterSort = Array.from(
      container.querySelectorAll("a[href^='/app/item/episode-']")
    );
    expect(orderedLinksAfterSort[0]?.getAttribute("href")).toBe("/app/item/episode-002");

    const seasonTwoButton = Array.from(container.querySelectorAll("button")).find((button) =>
      button.textContent?.includes("第 2 季")
    );
    expect(seasonTwoButton).not.toBeNull();

    await act(async () => {
      seasonTwoButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    const seasonTwoEpisodeLink = container.querySelector(
      "a[href='/app/item/episode-003']"
    ) as HTMLAnchorElement | null;
    expect(seasonTwoEpisodeLink).not.toBeNull();
    expect(getShowEpisodesMock).toHaveBeenCalledWith("series-001", { seasonId: "season-002" });
  });

  it("keeps collection folder browsing and routes series children to library pages", async () => {
    getUserItemMock.mockResolvedValue({
      Id: "root-series",
      Name: "剧集推荐",
      Type: "CollectionFolder",
      Path: "/media/shows",
    });

    getUserItemsMock.mockResolvedValue({
      Items: [
        {
          Id: "series-009",
          Name: "示例剧",
          Type: "Series",
          Path: "/media/shows/demo-series",
        },
        {
          Id: "movie-009",
          Name: "示例电影",
          Type: "Movie",
          Path: "/media/movies/demo-movie.strm",
        },
      ],
      TotalRecordCount: 2,
      StartIndex: 0,
    });

    await act(async () => {
      root.render(<LibraryBrowser parentId="root-series" />);
      await flushEffects();
    });

    expect(getUserItemsMock).toHaveBeenCalledWith(
      "user-1",
      expect.objectContaining({ parentId: "root-series", limit: 24, startIndex: 0 })
    );

    const seriesLink = container.querySelector(
      "a[data-testid='poster-series-009']"
    ) as HTMLAnchorElement | null;
    const movieLink = container.querySelector(
      "a[data-testid='poster-movie-009']"
    ) as HTMLAnchorElement | null;

    expect(seriesLink?.getAttribute("href")).toBe("/app/library/series-009");
    expect(movieLink?.getAttribute("href")).toBe("/app/item/movie-009");
  });
});
