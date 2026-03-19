import { apiRequest, getApiBaseUrl } from "@/lib/api/client";
import { getAccessToken } from "@/lib/auth/token";
import {
  mockAdminGetAgentRequest,
  mockAdminGetAgentSettings,
  mockAdminListAgentProviders,
  mockAdminListAgentRequests,
  mockAdminReviewAgentRequest,
  mockAdminRetryAgentRequest,
  mockAdminTestMoviePilot,
  mockAdminUpdateAgentSettings,
  mockCreateMyAgentRequest,
  mockGetMyAgentRequest,
  mockListMyAgentRequests,
  mockResubmitMyAgentRequest,
} from "@/lib/mock/api";
import { runWithMock } from "@/lib/mock/mode";
import type {
  AgentCreateRequest,
  AgentProviderStatus,
  AgentReplyRequest,
  AgentRequest,
  AgentRequestDetail,
  AgentRequestRealtimeEvent,
  AgentRequestsQuery,
  AgentReviewRequest,
  AgentSettings,
} from "@/lib/types/requests";

export async function listMyRequests(query: AgentRequestsQuery = {}): Promise<AgentRequest[]> {
  return runWithMock(
    () => mockListMyAgentRequests(query),
    () =>
      apiRequest<AgentRequest[]>("/api/requests", {
        query: query as Record<string, string | number | boolean | null | undefined>,
      })
  );
}

export async function createMyRequest(payload: AgentCreateRequest): Promise<AgentRequestDetail> {
  return runWithMock(
    () => mockCreateMyAgentRequest(payload),
    () =>
      apiRequest<AgentRequestDetail>("/api/requests", {
        method: "POST",
        body: JSON.stringify(payload),
      })
  );
}

export async function getMyRequest(requestId: string): Promise<AgentRequestDetail> {
  return runWithMock(
    () => mockGetMyAgentRequest(requestId),
    () => apiRequest<AgentRequestDetail>(`/api/requests/${requestId}`)
  );
}

export async function resubmitMyRequest(requestId: string): Promise<AgentRequestDetail> {
  return runWithMock(
    () => mockResubmitMyAgentRequest(requestId),
    () =>
      apiRequest<AgentRequestDetail>(`/api/requests/${requestId}/resubmit`, {
        method: "POST",
      })
  );
}

export async function replyMyRequest(
  requestId: string,
  payload: AgentReplyRequest
): Promise<AgentRequestDetail> {
  return runWithMock(
    () => mockGetMyAgentRequest(requestId),
    () =>
      apiRequest<AgentRequestDetail>(`/api/requests/${requestId}/reply`, {
        method: "POST",
        body: JSON.stringify(payload),
      })
  );
}

export async function adminListRequests(query: AgentRequestsQuery = {}): Promise<AgentRequest[]> {
  return runWithMock(
    () => mockAdminListAgentRequests(query),
    () =>
      apiRequest<AgentRequest[]>("/admin/requests", {
        query: query as Record<string, string | number | boolean | null | undefined>,
      })
  );
}

export async function adminGetRequest(requestId: string): Promise<AgentRequestDetail> {
  return runWithMock(
    () => mockAdminGetAgentRequest(requestId),
    () => apiRequest<AgentRequestDetail>(`/admin/requests/${requestId}`)
  );
}

export async function adminReviewRequest(
  requestId: string,
  payload: AgentReviewRequest
): Promise<AgentRequestDetail> {
  return runWithMock(
    () => mockAdminReviewAgentRequest(requestId, payload),
    () =>
      apiRequest<AgentRequestDetail>(`/admin/requests/${requestId}/review`, {
        method: "POST",
        body: JSON.stringify(payload),
      })
  );
}

export async function adminRetryRequest(requestId: string): Promise<AgentRequestDetail> {
  return runWithMock(
    () => mockAdminRetryAgentRequest(requestId),
    () =>
      apiRequest<AgentRequestDetail>(`/admin/requests/${requestId}/retry`, {
        method: "POST",
      })
  );
}

export async function adminGetAgentSettings(): Promise<AgentSettings> {
  return runWithMock(
    () => mockAdminGetAgentSettings(),
    () => apiRequest<AgentSettings>("/admin/agent/settings")
  );
}

export async function adminListAgentProviders(): Promise<AgentProviderStatus[]> {
  return runWithMock(
    () => mockAdminListAgentProviders(),
    () => apiRequest<AgentProviderStatus[]>("/admin/agent/providers")
  );
}

export async function adminUpdateAgentSettings(payload: AgentSettings): Promise<AgentSettings> {
  return runWithMock(
    () => mockAdminUpdateAgentSettings(payload),
    () =>
      apiRequest<AgentSettings>("/admin/agent/settings", {
        method: "POST",
        body: JSON.stringify(payload),
      })
  );
}

export async function adminTestMoviePilot(config: AgentSettings): Promise<Record<string, unknown>> {
  return runWithMock(
    () => mockAdminTestMoviePilot(config),
    () =>
      apiRequest<Record<string, unknown>>("/admin/agent/moviepilot/test", {
        method: "POST",
        body: JSON.stringify({ config }),
      })
  );
}

export function getMyRequestsWebSocketUrl(token: string): string {
  const httpUrl = new URL(`${getApiBaseUrl()}/api/requests/ws`);
  httpUrl.searchParams.set("token", token);
  httpUrl.protocol = httpUrl.protocol === "https:" ? "wss:" : "ws:";
  return httpUrl.toString();
}

export function getAdminRequestsWebSocketUrl(token: string): string {
  const httpUrl = new URL(`${getApiBaseUrl()}/admin/requests/ws`);
  httpUrl.searchParams.set("token", token);
  httpUrl.protocol = httpUrl.protocol === "https:" ? "wss:" : "ws:";
  return httpUrl.toString();
}

export function getRequestsWebSocketToken(): string | null {
  return getAccessToken();
}

export type { AgentRequestRealtimeEvent };
