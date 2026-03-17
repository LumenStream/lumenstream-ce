export const SCRAPER_SCENARIO_KEYS = [
  "movie_metadata",
  "series_metadata",
  "season_metadata",
  "episode_metadata",
  "person_metadata",
  "image_fetch",
  "search_by_title",
  "search_by_external_id",
] as const;

export type ScraperScenarioKey = (typeof SCRAPER_SCENARIO_KEYS)[number];
export type LibraryPolicyObject = Record<string, unknown>;

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

export function extractLibraryScenarioInputs(
  policy: LibraryPolicyObject | null
): Record<string, string> {
  const scenarioDefaults = isRecord(policy?.scenario_defaults) ? policy.scenario_defaults : {};
  const next: Record<string, string> = {};
  SCRAPER_SCENARIO_KEYS.forEach((scenarioKey) => {
    const chain = Array.isArray(scenarioDefaults[scenarioKey])
      ? scenarioDefaults[scenarioKey]
          .filter((value): value is string => typeof value === "string")
          .map((value) => value.trim())
          .filter(Boolean)
      : [];
    next[scenarioKey] = chain.join(", ");
  });
  return next;
}

export function updateLibraryPolicyScenarioInput(
  rawPolicyInput: string,
  scenarioKey: ScraperScenarioKey,
  nextChainInput: string
): string {
  const basePolicy = parseLibraryPolicyInput(rawPolicyInput) ?? {};
  const nextPolicy: LibraryPolicyObject = { ...basePolicy };
  const scenarioDefaults = isRecord(nextPolicy.scenario_defaults)
    ? { ...nextPolicy.scenario_defaults }
    : {};
  const chain = parseProviderChainInput(nextChainInput);

  if (chain.length > 0) {
    scenarioDefaults[scenarioKey] = chain;
  } else {
    delete scenarioDefaults[scenarioKey];
  }

  if (Object.keys(scenarioDefaults).length > 0) {
    nextPolicy.scenario_defaults = scenarioDefaults;
  } else {
    delete nextPolicy.scenario_defaults;
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
