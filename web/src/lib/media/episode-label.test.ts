import { describe, expect, test } from "vitest";

import type { BaseItem } from "@/lib/types/jellyfin";

import { formatEpisodeCode, formatEpisodeLabel } from "./episode-label";

describe("formatEpisodeCode", () => {
  test("pads season and episode to two digits", () => {
    expect(formatEpisodeCode(1, 3)).toBe("S01E03");
    expect(formatEpisodeCode(12, 24)).toBe("S12E24");
  });

  test("defaults season to 1 when null/undefined", () => {
    expect(formatEpisodeCode(null, 5)).toBe("S01E05");
    expect(formatEpisodeCode(undefined, 5)).toBe("S01E05");
  });

  test("defaults episode to 0 when null/undefined", () => {
    expect(formatEpisodeCode(2, null)).toBe("S02E00");
    expect(formatEpisodeCode(2, undefined)).toBe("S02E00");
  });
});

describe("formatEpisodeLabel", () => {
  const episode = {
    Id: "1",
    Name: "末日地堡",
    Type: "Episode",
    Path: "/media/silo/s02e10.mkv",
    SeriesName: "Silo",
    ParentIndexNumber: 2,
    IndexNumber: 10,
  } satisfies BaseItem;

  test("returns series + code in mixed context (default)", () => {
    expect(formatEpisodeLabel(episode)).toBe("Silo S02E10");
  });

  test("returns code only with seriesContext: true", () => {
    expect(formatEpisodeLabel(episode, { seriesContext: true })).toBe("S02E10");
  });

  test("falls back to code when SeriesName is missing", () => {
    const noSeries = { ...episode, SeriesName: undefined };
    expect(formatEpisodeLabel(noSeries)).toBe("S02E10");
  });

  test("returns item.Name for non-Episode types", () => {
    const movie = { ...episode, Id: "2", Name: "Dune", Type: "Movie" };
    expect(formatEpisodeLabel(movie)).toBe("Dune");
  });

  test("returns empty string when non-Episode has no Name", () => {
    const noName = { ...episode, Id: "3", Name: "", Type: "Series" };
    expect(formatEpisodeLabel(noName)).toBe("");
  });
});
