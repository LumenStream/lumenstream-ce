import { apiRequest, getApiBaseUrl, type ApiError } from "./client";

/**
 * Notification from the backend API.
 */
export interface ApiNotification {
  id: string;
  user_id: string;
  title: string;
  message: string;
  notification_type: "info" | "success" | "warning" | "error";
  is_read: boolean;
  meta: Record<string, unknown>;
  created_at: string;
  read_at: string | null;
}

/**
 * Paginated response for notifications list.
 */
export interface NotificationsListResponse {
  items: ApiNotification[];
  total: number;
  limit: number;
  offset: number;
}

/**
 * Query parameters for listing notifications.
 */
export interface ListNotificationsParams {
  limit?: number;
  offset?: number;
}

/**
 * Fetch paginated notifications for the current user.
 */
export async function listNotifications(
  params: ListNotificationsParams = {}
): Promise<NotificationsListResponse> {
  return apiRequest<NotificationsListResponse>("/api/notifications", {
    method: "GET",
    query: {
      limit: params.limit,
      offset: params.offset,
    },
  });
}

/**
 * Response from marking a notification as read.
 */
export interface MarkReadResponse {
  updated: boolean;
}

/**
 * Mark a single notification as read.
 */
export async function markNotificationRead(notificationId: string): Promise<MarkReadResponse> {
  return apiRequest<MarkReadResponse>(`/api/notifications/${notificationId}/read`, {
    method: "PATCH",
  });
}

/**
 * Response from marking all notifications as read.
 */
export interface MarkAllReadResponse {
  updated_count: number;
}

/**
 * Mark all notifications as read for the current user.
 */
export async function markAllNotificationsRead(): Promise<MarkAllReadResponse> {
  return apiRequest<MarkAllReadResponse>("/api/notifications/read-all", {
    method: "PATCH",
  });
}

/**
 * Get the WebSocket URL for notifications.
 */
export function getNotificationsWebSocketUrl(token: string): string {
  const base = getApiBaseUrl();
  const httpUrl = new URL(`${base}/api/notifications/ws`);
  httpUrl.searchParams.set("token", token);
  if (httpUrl.protocol === "https:") {
    httpUrl.protocol = "wss:";
  } else {
    httpUrl.protocol = "ws:";
  }
  return httpUrl.toString();
}

export type { ApiError };
