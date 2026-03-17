import { useSyncExternalStore, useEffect, useCallback } from "react";
import {
  subscribe,
  getSnapshot,
  getUnreadCount,
  fetchNotifications,
  markRead,
  markAllRead,
  initializeNotifications,
} from "./notification-store";
import type { ApiNotification } from "@/lib/api/notifications";
import type { ConnectionStatus } from "./ws-client";

/**
 * Return type for the useNotifications hook.
 */
export interface UseNotificationsResult {
  /** List of notifications */
  notifications: ApiNotification[];
  /** Total count of notifications */
  total: number;
  /** Count of unread notifications */
  unreadCount: number;
  /** Whether notifications are being loaded */
  loading: boolean;
  /** Error message if any */
  error: string | null;
  /** WebSocket connection status */
  connectionStatus: ConnectionStatus;
  /** Mark a single notification as read */
  markRead: (notificationId: string) => Promise<void>;
  /** Mark all notifications as read */
  markAllRead: () => Promise<void>;
  /** Refresh notifications from the API */
  refresh: () => Promise<void>;
}

/**
 * Hook to access and manage notifications.
 * Automatically initializes WebSocket connection and fetches initial data.
 */
export function useNotifications(): UseNotificationsResult {
  const state = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);

  useEffect(() => {
    const cleanup = initializeNotifications();
    return cleanup;
  }, []);

  const refresh = useCallback(() => fetchNotifications(), []);

  return {
    notifications: state.notifications,
    total: state.total,
    unreadCount: getUnreadCount(),
    loading: state.loading,
    error: state.error,
    connectionStatus: state.connectionStatus,
    markRead,
    markAllRead,
    refresh,
  };
}

/**
 * Hook to get just the unread count (lightweight subscription).
 */
export function useUnreadCount(): number {
  const state = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
  return state.notifications.filter((n) => !n.is_read).length;
}

/**
 * Hook to get just the connection status.
 */
export function useConnectionStatus(): ConnectionStatus {
  const state = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
  return state.connectionStatus;
}
