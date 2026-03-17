import { apiRequest } from "@/lib/api/client";
import { mockGetMyInviteSummary, mockResetMyInviteCode } from "@/lib/mock/api";
import { runWithMock } from "@/lib/mock/mode";
import type { InviteSummary } from "@/lib/types/admin";

export async function getMyInviteSummary(userIdForMock: string): Promise<InviteSummary> {
  return runWithMock(
    () => mockGetMyInviteSummary(userIdForMock),
    () => apiRequest<InviteSummary>("/api/invite/me")
  );
}

export async function resetMyInviteCode(userIdForMock: string): Promise<InviteSummary> {
  return runWithMock(
    () => mockResetMyInviteCode(userIdForMock),
    () =>
      apiRequest<InviteSummary>("/api/invite/me/reset", {
        method: "POST",
      })
  );
}
