/**
 * Extract a path segment value from `window.location.pathname`.
 *
 * Given a pathname like `/app/item/abc-123`, calling
 * `getRouteParam("item")` returns `"abc-123"`.
 *
 * It finds the segment matching `key` and returns the *next* segment
 * (decoded). Returns `""` when the key is not found or there is no
 * following segment.
 */
export function getRouteParam(key: string): string {
  if (typeof window === "undefined") {
    return "";
  }

  const segments = window.location.pathname.split("/").filter(Boolean);
  const index = segments.indexOf(key);

  if (index === -1 || index + 1 >= segments.length) {
    return "";
  }

  return decodeURIComponent(segments[index + 1]!);
}
