import { apiRequest } from "@/lib/api/client";
import { mockGetSystemCapabilities } from "@/lib/mock/api";
import { runWithMock } from "@/lib/mock/mode";
import type { AdminSystemCapabilities } from "@/lib/types/admin";

export async function getPublicSystemCapabilities(): Promise<AdminSystemCapabilities> {
  return runWithMock(
    () => mockGetSystemCapabilities(),
    () =>
      apiRequest<AdminSystemCapabilities>("/api/system/capabilities", {
        auth: false,
      })
  );
}
