import { beforeEach, describe, expect, it, vi } from "vitest";

import { apiRequest, getApiBaseUrl } from "@/lib/api/client";
import { isMockMode, runWithMock } from "@/lib/mock/mode";
import type { BaseItem, QueryResult } from "@/lib/types/jellyfin";

import {
  buildItemBackdropUrl,
  buildItemImageUrl,
  buildPersonImageUrl,
  clearRootItemsSharedCache,
  deleteItem,
  getPerson,
  getPersonItems,
  getItems,
  getRootItemsShared,
  getUserItems,
  refreshItemMetadata,
  updateItemMetadata,
} from "./items";

vi.mock("@/lib/api/client", () => ({
  apiRequest: vi.fn(),
  getApiBaseUrl: vi.fn(() => "https://api.example.com"),
}));

vi.mock("@/lib/mock/mode", () => ({
  isMockMode: vi.fn(() => false),
  runWithMock: vi.fn(),
}));

const mockGetApiBaseUrl = vi.mocked(getApiBaseUrl);
const mockIsMockMode = vi.mocked(isMockMode);
const mockApiRequest = vi.mocked(apiRequest);
const mockRunWithMock = vi.mocked(runWithMock);

beforeEach(() => {
  clearRootItemsSharedCache();
  mockRunWithMock.mockImplementation((_mockHandler, liveHandler) => liveHandler());
  mockApiRequest.mockResolvedValue({
    Items: [],
    TotalRecordCount: 0,
    StartIndex: 0,
  });
});

describe("buildItemImageUrl", () => {
  it("returns encoded data-url poster in mock mode", () => {
    mockIsMockMode.mockReturnValue(true);

    const imageUrl = buildItemImageUrl("demo-item-1", "token-ignored");

    expect(imageUrl.startsWith("data:image/svg+xml;charset=utf-8,")).toBe(true);
    expect(decodeURIComponent(imageUrl)).toContain("LumenStream DEMO-ITE");
  });

  it("returns API image URL with token in normal mode", () => {
    mockIsMockMode.mockReturnValue(false);
    mockGetApiBaseUrl.mockReturnValue("https://lumenstream.example.com");

    const imageUrl = buildItemImageUrl("movie-1", "abc123");

    expect(imageUrl).toBe(
      "https://lumenstream.example.com/Items/movie-1/Images/Primary?api_key=abc123"
    );
  });
});

describe("buildItemBackdropUrl", () => {
  it("returns encoded data-url backdrop in mock mode", () => {
    mockIsMockMode.mockReturnValue(true);

    const imageUrl = buildItemBackdropUrl("demo-item-2", "token-ignored");

    expect(imageUrl.startsWith("data:image/svg+xml;charset=utf-8,")).toBe(true);
    expect(decodeURIComponent(imageUrl)).toContain("LumenStream BACKDROP");
  });

  it("returns API backdrop URL with token in normal mode", () => {
    mockIsMockMode.mockReturnValue(false);
    mockGetApiBaseUrl.mockReturnValue("https://lumenstream.example.com");

    const imageUrl = buildItemBackdropUrl("movie-1", "abc123", 1);

    expect(imageUrl).toBe(
      "https://lumenstream.example.com/Items/movie-1/Images/Backdrop/1?api_key=abc123"
    );
  });
});

describe("buildPersonImageUrl", () => {
  it("returns encoded data-url avatar in mock mode", () => {
    mockIsMockMode.mockReturnValue(true);

    const imageUrl = buildPersonImageUrl("person-1", "token-ignored");

    expect(imageUrl.startsWith("data:image/svg+xml;charset=utf-8,")).toBe(true);
    expect(decodeURIComponent(imageUrl)).toContain("LumenStream PERSON-1");
  });

  it("returns API person image URL with token in normal mode", () => {
    mockIsMockMode.mockReturnValue(false);
    mockGetApiBaseUrl.mockReturnValue("https://lumenstream.example.com");

    const imageUrl = buildPersonImageUrl("person-1", "abc123");

    expect(imageUrl).toBe(
      "https://lumenstream.example.com/Persons/person-1/Images/Primary?api_key=abc123"
    );
  });
});

describe("items query mapping", () => {
  it("dedupes concurrent /Items/Root requests for same user", async () => {
    let resolveFetch!: (value: QueryResult<BaseItem>) => void;
    const pending = new Promise<QueryResult<BaseItem>>((resolve) => {
      resolveFetch = resolve;
    });
    mockApiRequest.mockReturnValueOnce(pending);

    const first = getRootItemsShared("user-1");
    const second = getRootItemsShared("user-1");

    expect(mockApiRequest).toHaveBeenCalledTimes(1);
    resolveFetch({
      Items: [{ Id: "lib-1", Name: "影视库", Type: "CollectionFolder", Path: "/media/lib-1" }],
      TotalRecordCount: 1,
      StartIndex: 0,
    });

    const [firstResult, secondResult] = await Promise.all([first, second]);
    expect(firstResult).toEqual(secondResult);
  });

  it("reuses recent root items cache until explicitly cleared", async () => {
    mockApiRequest.mockClear();

    await getRootItemsShared("user-1");
    await getRootItemsShared("user-1");
    expect(mockApiRequest).toHaveBeenCalledTimes(1);

    clearRootItemsSharedCache("user-1");
    await getRootItemsShared("user-1");
    expect(mockApiRequest).toHaveBeenCalledTimes(2);
  });

  it("maps personIds on /Items query", async () => {
    await getItems({
      searchTerm: "pacino",
      includeItemTypes: "Movie,Series",
      personIds: "person-1,person-2",
      limit: 20,
      startIndex: 0,
    });

    expect(mockApiRequest).toHaveBeenCalledWith(
      "/Items",
      expect.objectContaining({
        query: expect.objectContaining({
          SearchTerm: "pacino",
          IncludeItemTypes: "Movie,Series",
          PersonIds: "person-1,person-2",
          Limit: 20,
          StartIndex: 0,
        }),
      })
    );
  });

  it("maps personIds on /Users/{id}/Items query", async () => {
    await getUserItems("user-1", {
      personIds: "person-9",
    });

    expect(mockApiRequest).toHaveBeenCalledWith(
      "/Users/user-1/Items",
      expect.objectContaining({
        query: expect.objectContaining({
          PersonIds: "person-9",
        }),
      })
    );
  });

  it("queries /Persons/{id} for person detail", async () => {
    await getPerson("person-1");

    expect(mockApiRequest).toHaveBeenCalledWith("/Persons/person-1");
  });

  it("maps person filter when querying person credits", async () => {
    await getPersonItems("person-7", {
      includeItemTypes: "Movie,Series",
      limit: 12,
    });

    expect(mockApiRequest).toHaveBeenCalledWith(
      "/Items",
      expect.objectContaining({
        query: expect.objectContaining({
          PersonIds: "person-7",
          IncludeItemTypes: "Movie,Series",
          Limit: 12,
        }),
      })
    );
  });

  it("posts metadata patch payload to /Items/{id}", async () => {
    await updateItemMetadata("movie-1", {
      Name: "Interstellar",
      Overview: "Space epic",
      ProductionYear: 2014,
      TmdbId: "157336",
      ImdbId: "tt0816692",
      ProviderIds: {
        Tmdb: "157336",
      },
    });

    expect(mockApiRequest).toHaveBeenCalledWith(
      "/Items/movie-1",
      expect.objectContaining({
        method: "POST",
      })
    );
  });

  it("calls item refresh endpoint with mapped query keys", async () => {
    await refreshItemMetadata("movie-1", {
      replaceAllMetadata: true,
      metadataRefreshMode: "FullRefresh",
    });

    expect(mockApiRequest).toHaveBeenCalledWith(
      "/Items/movie-1/Refresh",
      expect.objectContaining({
        method: "POST",
        query: expect.objectContaining({
          ReplaceAllMetadata: true,
          MetadataRefreshMode: "FullRefresh",
        }),
      })
    );
  });

  it("calls item refresh endpoint with image refresh query keys", async () => {
    await refreshItemMetadata("movie-1", {
      replaceAllImages: true,
      imageRefreshMode: "FullRefresh",
    });

    expect(mockApiRequest).toHaveBeenCalledWith(
      "/Items/movie-1/Refresh",
      expect.objectContaining({
        method: "POST",
        query: expect.objectContaining({
          ReplaceAllImages: true,
          ImageRefreshMode: "FullRefresh",
        }),
      })
    );
  });

  it("calls delete endpoint for item removal", async () => {
    await deleteItem("movie-1");

    expect(mockApiRequest).toHaveBeenCalledWith(
      "/Items/movie-1",
      expect.objectContaining({
        method: "DELETE",
      })
    );
  });
});
