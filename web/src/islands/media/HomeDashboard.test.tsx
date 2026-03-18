/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { HomeDashboard } from "./HomeDashboard";

const getRootItemsMock = vi.fn();
const getResumeItemsMock = vi.fn();
const getItemCountsMock = vi.fn();
const getTopPlayedItemsMock = vi.fn();
const getUserItemsMock = vi.fn();
const scrollBySpy = vi.fn();

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

vi.mock("@/lib/hooks/use-image-glow", () => ({
  useImageGlow: () => ({ glowColor: null }),
}));

vi.mock("@/components/ui/button", () => ({
  buttonVariants: () => "btn",
}));

vi.mock("@/components/domain/DataState", () => ({
  EmptyState: ({ title }: { title?: string }) => React.createElement("div", null, title || ""),
  ErrorState: ({ title }: { title?: string }) => React.createElement("div", null, title || ""),
  LoadingState: ({ title }: { title?: string }) => React.createElement("div", null, title || ""),
}));

vi.mock("@/components/domain/PosterItemCard", () => ({
  PosterItemCard: ({ item, href }: { item: { Id: string; Name: string }; href: string }) =>
    React.createElement("a", { href, "data-testid": `poster-${item.Id}` }, item.Name),
}));

vi.mock("@/lib/api/items", () => ({
  getRootItemsShared: (...args: unknown[]) => getRootItemsMock(...args),
  getResumeItems: (...args: unknown[]) => getResumeItemsMock(...args),
  getItemCounts: (...args: unknown[]) => getItemCountsMock(...args),
  getTopPlayedItems: (...args: unknown[]) => getTopPlayedItemsMock(...args),
  getUserItems: (...args: unknown[]) => getUserItemsMock(...args),
  buildItemImageUrl: (itemId: string) => `https://image.example.com/${itemId}`,
  buildItemBackdropUrl: (itemId: string) => `https://backdrop.example.com/${itemId}`,
}));

async function flushEffects() {
  for (let i = 0; i < 8; i += 1) {
    await Promise.resolve();
  }
}

describe("HomeDashboard", () => {
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

    scrollBySpy.mockReset();
    Object.defineProperty(HTMLElement.prototype, "scrollBy", {
      configurable: true,
      writable: true,
      value: scrollBySpy,
    });

    getRootItemsMock.mockResolvedValue({
      Items: [
        {
          Id: "lib-movie",
          Name: "动画电影",
          Type: "CollectionFolder",
          Path: "/media/movies",
          ImagePrimaryUrl: "https://admin.example.com/library-movie-cover.jpg",
        },
        { Id: "lib-drama", Name: "国产剧集", Type: "CollectionFolder", Path: "/media/drama" },
      ],
      TotalRecordCount: 2,
      StartIndex: 0,
    });

    getResumeItemsMock.mockResolvedValue({
      Items: [],
      TotalRecordCount: 0,
      StartIndex: 0,
    });

    getItemCountsMock.mockResolvedValue({
      MovieCount: 20,
      SeriesCount: 10,
      EpisodeCount: 100,
      SongCount: 0,
      AlbumCount: 0,
      ArtistCount: 0,
      ProgramCount: 0,
      TrailerCount: 0,
    });

    getTopPlayedItemsMock.mockResolvedValue({
      StatDate: "2026-03-01",
      WindowDays: 1,
      Items: [],
    });

    getUserItemsMock.mockImplementation(async (_userId: string, query?: { parentId?: string }) => {
      if (query?.parentId === "lib-movie") {
        return {
          Items: [{ Id: "movie-1", Name: "天空之城", Type: "Movie", Path: "/media/movies/a.mkv" }],
          TotalRecordCount: 1,
          StartIndex: 0,
        };
      }

      if (query?.parentId === "lib-drama") {
        return {
          Items: [{ Id: "series-1", Name: "狂飙", Type: "Series", Path: "/media/drama/a" }],
          TotalRecordCount: 1,
          StartIndex: 0,
        };
      }

      return { Items: [], TotalRecordCount: 0, StartIndex: 0 };
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

  it("renders media library as a horizontal cover shelf", async () => {
    await act(async () => {
      root.render(<HomeDashboard />);
      await flushEffects();
    });

    const text = container.textContent || "";
    expect(text).toContain("媒体库");
    expect(text).toContain("所有媒体库概览");

    const shelf = container.querySelector("[aria-label='媒体库横向列表']");
    expect(shelf).not.toBeNull();
    expect(shelf?.className).toContain("overflow-x-auto");

    const libraryLink = shelf?.querySelector("a[href='/app/library/lib-movie']");
    expect(libraryLink).not.toBeNull();

    const coverImage = libraryLink?.querySelector("img");
    expect(coverImage?.getAttribute("src")).toBe(
      "https://admin.example.com/library-movie-cover.jpg"
    );
  });

  it("supports left/right horizontal scrolling for library shelf", async () => {
    await act(async () => {
      root.render(<HomeDashboard />);
      await flushEffects();
    });

    const leftButton = container.querySelector("button[aria-label='向左滑动媒体库']");
    const rightButton = container.querySelector("button[aria-label='向右滑动媒体库']");

    expect(leftButton).not.toBeNull();
    expect(rightButton).not.toBeNull();

    await act(async () => {
      rightButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(scrollBySpy).toHaveBeenCalledWith({ left: 420, behavior: "smooth" });

    await act(async () => {
      leftButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(scrollBySpy).toHaveBeenCalledWith({ left: -420, behavior: "smooth" });
  });
});
