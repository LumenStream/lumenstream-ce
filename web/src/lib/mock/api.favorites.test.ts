import { beforeEach, describe, expect, it, vi } from "vitest";

let mockApi: typeof import("./api");

beforeEach(async () => {
  vi.resetModules();
  mockApi = await import("./api");
});

describe("favorite and default playlist sync", () => {
  it("keeps default playlist items aligned with favorite toggles", async () => {
    const before = await mockApi.mockListMyPlaylists();
    const defaultPlaylist = before.find((entry) => entry.is_default);
    expect(defaultPlaylist).toBeTruthy();

    await mockApi.mockAddFavoriteItem("user-admin-001", "movie-003");

    const afterAdd = await mockApi.mockListMyPlaylists();
    const defaultAfterAdd = afterAdd.find((entry) => entry.is_default);
    expect(defaultAfterAdd).toBeTruthy();
    const itemsAfterAdd = await mockApi.mockListPlaylistItems(defaultAfterAdd!.id);
    expect(itemsAfterAdd.items.some((item) => item.Id === "movie-003")).toBe(true);

    await mockApi.mockRemoveFavoriteItem("user-admin-001", "movie-003");

    const itemsAfterRemove = await mockApi.mockListPlaylistItems(defaultAfterAdd!.id);
    expect(itemsAfterRemove.items.some((item) => item.Id === "movie-003")).toBe(false);
  });
});
