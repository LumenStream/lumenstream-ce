import { describe, expect, it } from "vitest";

import { resolveMediaItemHref } from "./item-href";

describe("resolveMediaItemHref", () => {
  it("routes series items to the library page", () => {
    expect(resolveMediaItemHref({ Id: "series-1", Type: "Series", Name: "Test" })).toBe(
      "/app/library/series-1"
    );
  });

  it("routes collection folders to the library page", () => {
    expect(resolveMediaItemHref({ Id: "root-1", Type: "CollectionFolder", Name: "Test" })).toBe(
      "/app/library/root-1"
    );
  });

  it("routes playable items to the item detail page", () => {
    expect(resolveMediaItemHref({ Id: "movie-1", Type: "Movie", Name: "Test" })).toBe(
      "/app/item/movie-1"
    );
    expect(resolveMediaItemHref({ Id: "episode-1", Type: "Episode", Name: "Test" })).toBe(
      "/app/item/episode-1"
    );
  });

  it("routes person items to person detail page", () => {
    const href = resolveMediaItemHref({ Id: "person-1", Type: "Person", Name: "张三" });
    expect(href).toBe("/app/person/person-1");
  });
});
