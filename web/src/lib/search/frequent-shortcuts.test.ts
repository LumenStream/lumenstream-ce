import { describe, expect, it } from "vitest";

import {
  frequentSearchShortcuts,
  getFrequentSearchShortcuts,
} from "@/lib/search/frequent-shortcuts";

describe("frequent search shortcuts model", () => {
  it("includes high-frequency link entries with expected routes", () => {
    const linkEntries = new Map<string, string>();

    for (const item of frequentSearchShortcuts) {
      if (item.action.type === "link") {
        linkEntries.set(item.label, item.action.href);
      }
    }

    expect(linkEntries.get("返回首页")).toBe("/app/home");
    expect(linkEntries.get("账户中心")).toBe("/app/profile");
    expect(linkEntries.get("进入管理端")).toBe("/admin/overview");
  });

  it("includes quick-fill search entries", () => {
    const movieShortcut = frequentSearchShortcuts.find(
      (item) => item.action.type === "search" && item.label === "找电影"
    );
    const seriesShortcut = frequentSearchShortcuts.find(
      (item) => item.action.type === "search" && item.label === "找剧集"
    );

    expect(movieShortcut).toBeTruthy();
    expect(seriesShortcut).toBeTruthy();

    if (!movieShortcut || movieShortcut.action.type !== "search") {
      throw new Error("missing movie shortcut");
    }

    if (!seriesShortcut || seriesShortcut.action.type !== "search") {
      throw new Error("missing series shortcut");
    }

    expect(movieShortcut.action.searchTerm).toBe("电影");
    expect(movieShortcut.action.includeItemTypes).toBe("Movie");
    expect(seriesShortcut.action.searchTerm).toBe("剧集");
    expect(seriesShortcut.action.includeItemTypes).toBe("Series");
  });

  it("returns the centralized shortcut collection", () => {
    expect(getFrequentSearchShortcuts()).toBe(frequentSearchShortcuts);
  });
});
