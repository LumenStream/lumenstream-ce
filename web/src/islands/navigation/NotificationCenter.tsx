import { useCallback, useEffect, useRef, useState } from "react";

import { Badge } from "@/components/ui/badge";
import {
  countUnread,
  markAllAsRead,
  markAsRead,
  sortNotificationsByTime,
  type Notification,
  type NotificationCenterProps,
  type NotificationType,
} from "@/lib/notifications/types";

const NOTIFICATION_TYPE_STYLES: Record<NotificationType, string> = {
  info: "border-l-blue-500",
  success: "border-l-emerald-500",
  warning: "border-l-amber-500",
  error: "border-l-red-500",
};

function formatRelativeTime(date: Date): string {
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMinutes = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMinutes / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffMinutes < 1) return "刚刚";
  if (diffMinutes < 60) return `${diffMinutes} 分钟前`;
  if (diffHours < 24) return `${diffHours} 小时前`;
  if (diffDays < 7) return `${diffDays} 天前`;
  return date.toLocaleDateString("zh-CN");
}

export default function NotificationCenter({ initialNotifications = [] }: NotificationCenterProps) {
  const [notifications, setNotifications] = useState<Notification[]>(() =>
    sortNotificationsByTime(initialNotifications)
  );
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const unreadCount = countUnread(notifications);

  const closeMenu = useCallback(() => setIsOpen(false), []);

  useEffect(() => {
    if (!isOpen) return;

    function handleClickOutside(event: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        closeMenu();
      }
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        closeMenu();
      }
    }

    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [isOpen, closeMenu]);

  useEffect(() => {
    if (isOpen && menuRef.current) {
      const firstButton = menuRef.current.querySelector("button");
      firstButton?.focus();
    }
  }, [isOpen]);

  const handleMarkAsRead = useCallback((id: string) => {
    setNotifications((prev) => markAsRead(prev, id));
  }, []);

  const handleMarkAllAsRead = useCallback(() => {
    setNotifications((prev) => markAllAsRead(prev));
  }, []);

  const handleKeyNavigation = useCallback((event: React.KeyboardEvent, index: number) => {
    const items = menuRef.current?.querySelectorAll("[role='menuitem']");
    if (!items) return;

    if (event.key === "ArrowDown") {
      event.preventDefault();
      const next = items[index + 1] as HTMLElement | undefined;
      next?.focus();
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      const prev = items[index - 1] as HTMLElement | undefined;
      prev?.focus();
    }
  }, []);

  return (
    <div ref={containerRef} className="relative">
      <button
        type="button"
        onClick={() => setIsOpen((prev) => !prev)}
        aria-expanded={isOpen}
        aria-haspopup="menu"
        aria-label={`通知中心${unreadCount > 0 ? `，${unreadCount} 条未读` : ""}`}
        className="border-border text-muted-foreground hover:text-foreground focus-visible:ring-ring light:hover:border-black/20 relative flex cursor-pointer items-center justify-center rounded-md border p-2 transition-colors hover:border-white/30 focus-visible:ring-2 focus-visible:outline-none"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9" />
          <path d="M10.3 21a1.94 1.94 0 0 0 3.4 0" />
        </svg>
        {unreadCount > 0 && (
          <Badge
            variant="danger"
            className="absolute -top-1.5 -right-1.5 flex h-5 min-w-5 items-center justify-center px-1 text-[10px]"
          >
            {unreadCount > 99 ? "99+" : unreadCount}
          </Badge>
        )}
      </button>

      {isOpen && (
        <div
          ref={menuRef}
          role="menu"
          aria-label="通知列表"
          className="light:border-black/[0.08] light:bg-white/95 light:shadow-black/10 absolute right-0 z-10 mt-2 w-80 rounded-md border border-white/[0.08] bg-neutral-900/95 shadow-2xl backdrop-blur-xl"
        >
          <div className="light:border-black/[0.06] flex items-center justify-between border-b border-white/5 px-3 py-2">
            <p id="notification-menu-label" className="text-foreground text-sm font-medium">
              通知中心
            </p>
            {unreadCount > 0 && (
              <button
                type="button"
                onClick={handleMarkAllAsRead}
                className="text-muted-foreground hover:text-foreground text-xs transition-colors"
              >
                全部已读
              </button>
            )}
          </div>

          <nav aria-labelledby="notification-menu-label" className="max-h-80 overflow-y-auto">
            {notifications.length === 0 ? (
              <div className="text-muted-foreground flex flex-col items-center justify-center py-8 text-sm">
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="32"
                  height="32"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  className="mb-2 opacity-50"
                  aria-hidden="true"
                >
                  <path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9" />
                  <path d="M10.3 21a1.94 1.94 0 0 0 3.4 0" />
                  <line x1="1" y1="1" x2="23" y2="23" />
                </svg>
                <p>暂无通知</p>
              </div>
            ) : (
              notifications.map((notification, index) => (
                <button
                  key={notification.id}
                  type="button"
                  role="menuitem"
                  tabIndex={0}
                  onClick={() => handleMarkAsRead(notification.id)}
                  onKeyDown={(e) => handleKeyNavigation(e, index)}
                  className={`hover:bg-background focus-visible:ring-ring w-full border-l-2 px-3 py-2.5 text-left transition-colors focus-visible:ring-2 focus-visible:outline-none ${
                    NOTIFICATION_TYPE_STYLES[notification.type]
                  } ${notification.read ? "opacity-60" : ""}`}
                >
                  <div className="flex items-start justify-between gap-2">
                    <p
                      className={`text-sm ${notification.read ? "text-muted-foreground" : "text-foreground font-medium"}`}
                    >
                      {notification.title}
                    </p>
                    {!notification.read && (
                      <span
                        className="mt-1.5 h-2 w-2 shrink-0 rounded-full bg-white/80"
                        aria-label="未读"
                      />
                    )}
                  </div>
                  <p className="text-muted-foreground mt-0.5 line-clamp-2 text-xs">
                    {notification.message}
                  </p>
                  <p className="text-muted-foreground mt-1 text-[10px]">
                    {formatRelativeTime(notification.timestamp)}
                  </p>
                </button>
              ))
            )}
          </nav>
        </div>
      )}
    </div>
  );
}
