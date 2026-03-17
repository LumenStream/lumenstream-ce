export type Platform = "ios" | "android" | "mac" | "windows" | "linux" | "unknown";

export interface PlayerDef {
  id: string;
  name: string;
  recommended: boolean;
  buildUrl: (streamUrl: string, title: string) => string;
}

export function detectPlatform(userAgent: string): Platform {
  const normalized = userAgent.toLowerCase();

  if (normalized.includes("iphone") || normalized.includes("ipad")) {
    return "ios";
  }
  if (normalized.includes("android")) {
    return "android";
  }
  if (normalized.includes("mac os")) {
    return "mac";
  }
  if (normalized.includes("windows")) {
    return "windows";
  }
  if (normalized.includes("linux")) {
    return "linux";
  }

  return "unknown";
}

const infuse: PlayerDef = {
  id: "infuse",
  name: "Infuse",
  recommended: false,
  buildUrl: (streamUrl, title) =>
    `infuse://x-callback-url/play?url=${encodeURIComponent(streamUrl)}&title=${encodeURIComponent(title)}`,
};

const senPlayer: PlayerDef = {
  id: "senplayer",
  name: "SenPlayer",
  recommended: false,
  buildUrl: (streamUrl) => `senplayer://play?url=${encodeURIComponent(streamUrl)}`,
};

const iina: PlayerDef = {
  id: "iina",
  name: "IINA",
  recommended: false,
  buildUrl: (streamUrl) => `iina://weblink?url=${encodeURIComponent(streamUrl)}`,
};

const mpv: PlayerDef = {
  id: "mpv",
  name: "MPV",
  recommended: false,
  buildUrl: (streamUrl) => `mpv://${streamUrl}`,
};

const potPlayer: PlayerDef = {
  id: "potplayer",
  name: "PotPlayer",
  recommended: false,
  buildUrl: (streamUrl) => `potplayer://${streamUrl}`,
};

const vlc: PlayerDef = {
  id: "vlc",
  name: "VLC",
  recommended: false,
  buildUrl: (streamUrl) => `vlc://${streamUrl}`,
};

function withRecommended(player: PlayerDef): PlayerDef {
  return { ...player, recommended: true };
}

export function getPlayersForPlatform(platform: Platform): PlayerDef[] {
  switch (platform) {
    case "ios":
      return [withRecommended(infuse), senPlayer, vlc];
    case "mac":
      return [withRecommended(infuse), iina, senPlayer, mpv, vlc];
    case "windows":
      return [withRecommended(potPlayer), mpv, vlc];
    case "android":
      return [withRecommended(vlc), mpv];
    case "linux":
      return [withRecommended(mpv), vlc];
    default:
      return [withRecommended(vlc), mpv];
  }
}

export async function attemptDeepLink(url: string, timeoutMs = 1200): Promise<boolean> {
  if (typeof window === "undefined") {
    return false;
  }

  return new Promise((resolve) => {
    let hidden = false;
    const startedAt = Date.now();

    function onVisibilityChange() {
      if (document.visibilityState === "hidden") {
        hidden = true;
      }
    }

    document.addEventListener("visibilitychange", onVisibilityChange);

    window.location.href = url;

    window.setTimeout(() => {
      document.removeEventListener("visibilitychange", onVisibilityChange);
      // When deeplink succeeds, browser generally loses visibility quickly.
      resolve(hidden || Date.now() - startedAt > timeoutMs + 200);
    }, timeoutMs);
  });
}
