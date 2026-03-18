export const SCRAPER_DEFAULT_ROUTE_KEYS = ["movie", "series", "image"] as const;
export const SCRAPER_LIBRARY_ROUTE_KEYS = ["movie", "series", "image"] as const;

export type ScraperDefaultRouteKey = (typeof SCRAPER_DEFAULT_ROUTE_KEYS)[number];
export type ScraperLibraryRouteKey = (typeof SCRAPER_LIBRARY_ROUTE_KEYS)[number];
export type LibraryPolicyObject = Record<string, unknown>;

const DEFAULT_ROUTE_LABELS: Record<ScraperDefaultRouteKey, string> = {
  movie: "电影",
  series: "电视剧",
  image: "图像",
};

const LIBRARY_ROUTE_LABELS: Record<ScraperLibraryRouteKey, string> = {
  movie: "电影",
  series: "电视剧",
  image: "图像获取",
};

export function parseProviderChainInput(value: string): string[] {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

export function parseLibraryPolicyInput(value: string): LibraryPolicyObject | null {
  try {
    const parsed = JSON.parse(value) as unknown;
    return isRecord(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

export function extractLibraryRouteInputs(
  policy: LibraryPolicyObject | null
): Record<ScraperLibraryRouteKey, string> {
  const next = {} as Record<ScraperLibraryRouteKey, string>;
  SCRAPER_LIBRARY_ROUTE_KEYS.forEach((routeKey) => {
    const chain = Array.isArray(policy?.[routeKey])
      ? policy[routeKey]
          .filter((value): value is string => typeof value === "string")
          .map((value) => value.trim())
          .filter(Boolean)
      : [];
    next[routeKey] = chain.join(", ");
  });
  return next;
}

export function updateLibraryPolicyRouteInput(
  rawPolicyInput: string,
  routeKey: ScraperLibraryRouteKey,
  nextChainInput: string
): string {
  const basePolicy = parseLibraryPolicyInput(rawPolicyInput) ?? {};
  const nextPolicy: LibraryPolicyObject = { ...basePolicy };
  const chain = parseProviderChainInput(nextChainInput);

  if (chain.length > 0) {
    nextPolicy[routeKey] = chain;
  } else {
    delete nextPolicy[routeKey];
  }

  return JSON.stringify(nextPolicy, null, 2);
}

export function formatLibraryPolicyInput(policy: Record<string, unknown> | undefined): string {
  return JSON.stringify(policy ?? {}, null, 2);
}

export function normalizeLibraryPolicyInput(value: string): string {
  const parsed = parseLibraryPolicyInput(value);
  return parsed ? JSON.stringify(parsed) : value.trim();
}

export function getScraperDefaultRouteLabel(routeKey: ScraperDefaultRouteKey): string {
  return DEFAULT_ROUTE_LABELS[routeKey];
}

export function getScraperLibraryRouteLabel(routeKey: ScraperLibraryRouteKey): string {
  return LIBRARY_ROUTE_LABELS[routeKey];
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
