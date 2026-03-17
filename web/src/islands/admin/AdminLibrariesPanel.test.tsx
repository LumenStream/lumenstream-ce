/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  parseLibraryPolicyInput,
  updateLibraryPolicyScenarioInput,
} from "@/lib/admin/scraper-policy";

import { AdminLibrariesPanel } from "./AdminLibrariesPanel";

const listLibraryStatusMock = vi.fn();
const listLibrariesMock = vi.fn();
const patchLibraryMock = vi.fn();
const createLibraryMock = vi.fn();
const setLibraryEnabledMock = vi.fn();
const uploadLibraryCoverMock = vi.fn();
const deleteLibraryCoverMock = vi.fn();
const listScraperProvidersMock = vi.fn();

vi.mock("@/lib/auth/use-auth-session", () => ({
  useAuthSession: () => ({ ready: true, session: { token: "demo-token" } }),
}));

vi.mock("@/lib/api/items", () => ({
  buildItemImageUrl: () => "/mock-cover.jpg",
}));

vi.mock("@/lib/api/admin", () => ({
  createLibrary: (...args: unknown[]) => createLibraryMock(...args),
  deleteLibraryCover: (...args: unknown[]) => deleteLibraryCoverMock(...args),
  listLibraries: (...args: unknown[]) => listLibrariesMock(...args),
  listLibraryStatus: (...args: unknown[]) => listLibraryStatusMock(...args),
  listScraperProviders: (...args: unknown[]) => listScraperProvidersMock(...args),
  patchLibrary: (...args: unknown[]) => patchLibraryMock(...args),
  setLibraryEnabled: (...args: unknown[]) => setLibraryEnabledMock(...args),
  uploadLibraryCover: (...args: unknown[]) => uploadLibraryCoverMock(...args),
}));

vi.mock("@/components/domain/DataState", () => ({
  EmptyState: ({ title }: { title: string }) => React.createElement("div", null, title),
  ErrorState: ({ title }: { title: string }) => React.createElement("div", null, title),
  LoadingState: ({ title }: { title?: string }) => React.createElement("div", null, title || ""),
}));

vi.mock("@/lib/notifications/toast-store", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

async function flushEffects() {
  await Promise.resolve();
  await Promise.resolve();
}

describe("AdminLibrariesPanel", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    listLibraryStatusMock.mockResolvedValue({
      total: 2,
      enabled: 2,
      items: [
        {
          id: "lib-001",
          name: "Movies",
          root_path: "/media/movies",
          paths: ["/media/movies"],
          library_type: "Movie",
          enabled: true,
          scraper_policy: {},
          item_count: 12,
          last_item_updated_at: "2026-03-14T12:00:00Z",
        },
        {
          id: "lib-002",
          name: "Shows",
          root_path: "/media/shows",
          paths: ["/media/shows", "/media/anime"],
          library_type: "Series",
          enabled: true,
          scraper_policy: {
            scenario_defaults: {
              series_metadata: ["bangumi", "tvdb", "tmdb"],
              episode_metadata: ["bangumi", "tvdb", "tmdb"],
              image_fetch: ["bangumi", "tvdb", "tmdb"],
            },
          },
          item_count: 24,
          last_item_updated_at: "2026-03-14T11:00:00Z",
        },
      ],
    });

    listScraperProvidersMock.mockResolvedValue([
      { provider_id: "tmdb", display_name: "TMDB" },
      { provider_id: "tvdb", display_name: "TVDB" },
      { provider_id: "bangumi", display_name: "Bangumi" },
    ]);

    listLibrariesMock.mockResolvedValue([
      {
        id: "lib-001",
        name: "Movies",
        root_path: "/media/movies",
        paths: ["/media/movies"],
        library_type: "Movie",
        enabled: true,
        scraper_policy: {},
      },
      {
        id: "lib-002",
        name: "Shows",
        root_path: "/media/shows",
        paths: ["/media/shows", "/media/anime"],
        library_type: "Series",
        enabled: true,
        scraper_policy: {
          scenario_defaults: {
            series_metadata: ["bangumi", "tvdb", "tmdb"],
            episode_metadata: ["bangumi", "tvdb", "tmdb"],
            image_fetch: ["bangumi", "tvdb", "tmdb"],
          },
        },
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
    vi.clearAllMocks();
  });

  it("renders library cards with per-library scraper routing controls", async () => {
    await act(async () => {
      root.render(<AdminLibrariesPanel />);
      await flushEffects();
    });

    expect(container.textContent).toContain("媒体库管理");
    expect(container.textContent).toContain("Movies");
    expect(container.textContent).toContain("Shows");

    await act(async () => {
      container
        .querySelector("tbody tr")
        ?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(container.textContent).toContain("刮削链路策略");
    expect(container.textContent).toContain("TMDB");
    expect(container.textContent).toContain("Bangumi");
    expect(container.textContent).toContain("保存链路策略");
  });

  it("updates scraper policy chains with shared helper", () => {
    const nextPolicy = updateLibraryPolicyScenarioInput(
      JSON.stringify(
        {
          scenario_defaults: {
            series_metadata: ["bangumi", "tvdb", "tmdb"],
            episode_metadata: ["bangumi", "tvdb", "tmdb"],
          },
        },
        null,
        2
      ),
      "series_metadata",
      "tvdb, tmdb"
    );

    expect(parseLibraryPolicyInput(nextPolicy)).toEqual({
      scenario_defaults: {
        series_metadata: ["tvdb", "tmdb"],
        episode_metadata: ["bangumi", "tvdb", "tmdb"],
      },
    });
  });
});
