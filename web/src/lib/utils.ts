import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}

export function formatDate(value?: string | null): string {
  if (!value) {
    return "-";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

export function isBrowser(): boolean {
  return typeof window !== "undefined";
}

export function formatRelativeTime(dateStr: string): string {
  const diffMins = Math.floor((Date.now() - new Date(dateStr).getTime()) / 60000);

  if (diffMins < 1) return "刚刚";
  if (diffMins < 60) return `${diffMins}分钟前`;
  if (diffMins < 1440) return `${Math.floor(diffMins / 60)}小时前`;
  return `${Math.floor(diffMins / 1440)}天前`;
}

export function formatDuration(startStr: string): string {
  const diffMins = Math.floor((Date.now() - new Date(startStr).getTime()) / 60000);
  if (diffMins < 1) return "不到1分钟";
  if (diffMins < 60) return `${diffMins}分钟`;
  if (diffMins < 1440) return `${Math.floor(diffMins / 60)}小时`;
  return `${Math.floor(diffMins / 1440)}天`;
}
