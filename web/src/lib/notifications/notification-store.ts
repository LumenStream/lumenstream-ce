import {
  listNotifications,
  markNotificationRead,
  markAllNotificationsRead,
  type ApiNotification,
  type ListNotificationsParams,
} from "@/lib/api/notifications";
import { getWebSocketClient, type ConnectionStatus } from "./ws-client";
import { toast } from "./toast-store";

type Listener = () => void;

/**
 * Internal state for the notification store.
 */
interface NotificationState {
  notifications: ApiNotification[];
  total: number;
  loading: boolean;
  error: string | null;
  connectionStatus: ConnectionStatus;
}

const listeners = new Set<Listener>();
let state: NotificationState = {
  notifications: [],
  total: 0,
  loading: false,
  error: null,
  connectionStatus: "disconnected",
};

function emitChange(): void {
  for (const listener of listeners) {
    listener();
  }
}

function setState(partial: Partial<NotificationState>): void {
  state = { ...state, ...partial };
  emitChange();
}

/**
 * Subscribe to state changes.
 */
export function subscribe(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/**
 * Get the current state snapshot.
 */
export function getSnapshot(): NotificationState {
  return state;
}

/**
 * Get the count of unread notifications.
 */
export function getUnreadCount(): number {
  return state.notifications.filter((n) => !n.is_read).length;
}

/**
 * Fetch notifications from the API.
 */
export async function fetchNotifications(params: ListNotificationsParams = {}): Promise<void> {
  setState({ loading: true, error: null });

  try {
    const response = await listNotifications(params);
    setState({
      notifications: response.items,
      total: response.total,
      loading: false,
    });
  } catch (err) {
    const message = err instanceof Error ? err.message : "获取通知失败";
    setState({ loading: false, error: message });
  }
}

/**
 * Mark a single notification as read with optimistic update.
 */
export async function markRead(notificationId: string): Promise<void> {
  // Optimistic update
  const previousNotifications = state.notifications;
  setState({
    notifications: state.notifications.map((n) =>
      n.id === notificationId ? { ...n, is_read: true, read_at: new Date().toISOString() } : n
    ),
  });

  try {
    await markNotificationRead(notificationId);
  } catch {
    // Rollback on failure
    setState({ notifications: previousNotifications });
  }
}

/**
 * Mark all notifications as read with optimistic update.
 */
export async function markAllRead(): Promise<void> {
  // Optimistic update
  const previousNotifications = state.notifications;
  const now = new Date().toISOString();
  setState({
    notifications: state.notifications.map((n) => ({
      ...n,
      is_read: true,
      read_at: n.read_at ?? now,
    })),
  });

  try {
    await markAllNotificationsRead();
  } catch {
    // Rollback on failure
    setState({ notifications: previousNotifications });
  }
}

/**
 * Add a new notification to the store (used by WebSocket events).
 */
export function addNotification(notification: ApiNotification): void {
  // Avoid duplicates
  if (state.notifications.some((n) => n.id === notification.id)) {
    return;
  }

  setState({
    notifications: [notification, ...state.notifications],
    total: state.total + 1,
  });

  // Show toast for new notification
  const variant = notification.notification_type;
  const showToast = toast[variant] ?? toast.info;
  showToast(notification.title);
}

/**
 * Update connection status.
 */
export function setConnectionStatus(status: ConnectionStatus): void {
  setState({ connectionStatus: status });
}

/**
 * Initialize the notification store and WebSocket connection.
 */
export function initializeNotifications(): () => void {
  const wsClient = getWebSocketClient();

  // Subscribe to WebSocket events
  const unsubNotification = wsClient.onNotification(addNotification);
  const unsubStatus = wsClient.onStatusChange(setConnectionStatus);

  // Connect to WebSocket
  wsClient.connect();

  // Fetch initial notifications
  fetchNotifications();

  // Return cleanup function
  return () => {
    unsubNotification();
    unsubStatus();
    wsClient.disconnect();
  };
}

/**
 * Clear all notifications from the store (useful for logout).
 */
export function clearNotifications(): void {
  setState({
    notifications: [],
    total: 0,
    loading: false,
    error: null,
  });
}
