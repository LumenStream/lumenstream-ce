import type { BaseItem } from "@/lib/types/jellyfin";

/** Format season + episode into "S01E03" style code. */
export function formatEpisodeCode(
  season: number | null | undefined,
  episode: number | null | undefined
): string {
  const s = String(season ?? 1).padStart(2, "0");
  const e = String(episode ?? 0).padStart(2, "0");
  return `S${s}E${e}`;
}

/**
 * Build a display label for an episode item.
 *
 * - `seriesContext: true`  → "S01E03"        (already inside a series page)
 * - `seriesContext: false` → "Silo S02E10"   (mixed list like favorites)
 * - Non-Episode items      → `item.Name`
 */
export function formatEpisodeLabel(item: BaseItem, opts?: { seriesContext?: boolean }): string {
  if (item.Type !== "Episode") {
    return item.Name ?? "";
  }

  const code = formatEpisodeCode(item.ParentIndexNumber, item.IndexNumber);

  if (opts?.seriesContext) {
    return code;
  }

  const series = item.SeriesName?.trim();
  return series ? `${series} ${code}` : code;
}
