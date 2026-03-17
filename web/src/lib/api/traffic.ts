import { apiRequest } from "@/lib/api/client";
import { mockGetMyTrafficUsageByMedia } from "@/lib/mock/api";
import { runWithMock } from "@/lib/mock/mode";
import type { MyTrafficUsageMediaSummary } from "@/lib/types/edition-commercial";

export async function getMyTrafficUsageByMedia(limit = 200): Promise<MyTrafficUsageMediaSummary> {
  return runWithMock(
    () => mockGetMyTrafficUsageByMedia(limit),
    () =>
      apiRequest<MyTrafficUsageMediaSummary>("/api/traffic/me/items", {
        query: { limit },
      })
  );
}
