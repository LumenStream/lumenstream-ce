import { describe, expect, it } from "vitest";

import { detectPlatform, getPlayersForPlatform } from "@/lib/player/deeplink";

describe("deeplink platform detection", () => {
  it("detects iOS", () => {
    expect(detectPlatform("Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)")).toBe("ios");
  });

  it("detects Android", () => {
    expect(detectPlatform("Mozilla/5.0 (Linux; Android 14; Pixel 8)")).toBe("android");
  });

  it("returns unknown for unrecognized UA", () => {
    expect(detectPlatform("Custom-UA")).toBe("unknown");
  });
});

describe("getPlayersForPlatform", () => {
  it("returns Infuse as recommended for iOS", () => {
    const players = getPlayersForPlatform("ios");
    expect(players[0]?.id).toBe("infuse");
    expect(players[0]?.recommended).toBe(true);
    expect(players.map((p) => p.id)).toEqual(["infuse", "senplayer", "vlc"]);
  });

  it("returns Infuse as recommended for macOS with IINA and others", () => {
    const players = getPlayersForPlatform("mac");
    expect(players[0]?.id).toBe("infuse");
    expect(players[0]?.recommended).toBe(true);
    expect(players.map((p) => p.id)).toEqual(["infuse", "iina", "senplayer", "mpv", "vlc"]);
  });

  it("returns PotPlayer as recommended for Windows", () => {
    const players = getPlayersForPlatform("windows");
    expect(players[0]?.id).toBe("potplayer");
    expect(players[0]?.recommended).toBe(true);
    expect(players.map((p) => p.id)).toEqual(["potplayer", "mpv", "vlc"]);
  });

  it("returns VLC as recommended for Android", () => {
    const players = getPlayersForPlatform("android");
    expect(players[0]?.id).toBe("vlc");
    expect(players[0]?.recommended).toBe(true);
  });

  it("returns MPV as recommended for Linux", () => {
    const players = getPlayersForPlatform("linux");
    expect(players[0]?.id).toBe("mpv");
    expect(players[0]?.recommended).toBe(true);
  });

  it("builds correct Infuse URL with encoded params", () => {
    const players = getPlayersForPlatform("ios");
    const infuse = players.find((p) => p.id === "infuse")!;
    const url = infuse.buildUrl("https://api.example.com/stream", "My Movie");
    expect(url).toContain("infuse://x-callback-url/play?url=");
    expect(url).toContain(encodeURIComponent("https://api.example.com/stream"));
    expect(url).toContain(encodeURIComponent("My Movie"));
  });

  it("builds correct VLC URL with raw stream", () => {
    const players = getPlayersForPlatform("android");
    const vlc = players.find((p) => p.id === "vlc")!;
    const url = vlc.buildUrl("https://api.example.com/stream", "Movie");
    expect(url).toBe("vlc://https://api.example.com/stream");
  });

  it("returns a fallback list for unknown platform", () => {
    const players = getPlayersForPlatform("unknown");
    expect(players.length).toBeGreaterThan(0);
    expect(players[0]?.recommended).toBe(true);
  });
});
