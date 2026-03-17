/**
 * Notification type categories for the notification center.
 */
export type NotificationType = "info" | "success" | "warning" | "error";

/**
 * A single notification item displayed in the notification center.
 */
export interface Notification {
  id: string;
  type: NotificationType;
  title: string;
  message: string;
  timestamp: Date;
  read: boolean;
}

/**
 * Props for the NotificationCenter component.
 */
export interface NotificationCenterProps {
  /** Initial notifications to display (optional, for SSR/testing) */
  initialNotifications?: Notification[];
}

/**
 * Creates a new notification with default values.
 */
export function createNotification(
  partial: Omit<Notification, "id" | "timestamp" | "read"> & {
    id?: string;
    timestamp?: Date;
    read?: boolean;
  }
): Notification {
  return {
    id: partial.id ?? crypto.randomUUID(),
    type: partial.type,
    title: partial.title,
    message: partial.message,
    timestamp: partial.timestamp ?? new Date(),
    read: partial.read ?? false,
  };
}

/**
 * Sorts notifications by timestamp in descending order (newest first).
 */
export function sortNotificationsByTime(notifications: Notification[]): Notification[] {
  return [...notifications].sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime());
}

/**
 * Counts unread notifications.
 */
export function countUnread(notifications: Notification[]): number {
  return notifications.filter((n) => !n.read).length;
}

/**
 * Marks a single notification as read.
 */
export function markAsRead(notifications: Notification[], id: string): Notification[] {
  return notifications.map((n) => (n.id === id ? { ...n, read: true } : n));
}

/**
 * Marks all notifications as read.
 */
export function markAllAsRead(notifications: Notification[]): Notification[] {
  return notifications.map((n) => ({ ...n, read: true }));
}
