import type { AdminSystemCapabilities } from "@/lib/types/admin";

export type AdminNavItemId =
  | "overview"
  | "users"
  | "billing"
  | "libraries"
  | "jobs"
  | "sessions"
  | "traffic"
  | "settings"
  | "scraper"
  | "tmdb"
  | "playback"
  | "requests"
  | "api-keys"
  | "audit-logs";

export interface EditionNavItem {
  id: AdminNavItemId;
  href: string;
  label: string;
}

export type ProfileSection = "billing" | "playback" | "social" | "traffic";

const ADMIN_NAV_ITEMS: readonly EditionNavItem[] = [
  { id: "overview", href: "/admin/overview", label: "总览" },
  { id: "users", href: "/admin/users", label: "用户" },
  { id: "billing", href: "/admin/billing", label: "账单" },
  { id: "libraries", href: "/admin/libraries", label: "媒体库" },
  { id: "jobs", href: "/admin/jobs", label: "任务中心" },
  { id: "sessions", href: "/admin/sessions", label: "会话" },
  { id: "traffic", href: "/admin/traffic", label: "流量" },
  { id: "playback", href: "/admin/playback", label: "推流与域名" },
  { id: "requests", href: "/admin/requests", label: "求片 Agent" },
  { id: "settings", href: "/admin/settings", label: "系统设置" },
  { id: "scraper", href: "/admin/scraper", label: "刮削系统" },
  { id: "api-keys", href: "/admin/api-keys", label: "API Keys" },
  { id: "audit-logs", href: "/admin/audit-logs", label: "审计日志" },
] as const;

export function getAdminNavItems(
  capabilities?: Pick<
    AdminSystemCapabilities,
    "billing_enabled" | "advanced_traffic_controls_enabled"
  > | null
): EditionNavItem[] {
  return ADMIN_NAV_ITEMS.filter((item) => {
    if (item.id === "billing") {
      return capabilities?.billing_enabled ?? false;
    }
    if (item.id === "traffic") {
      return capabilities?.advanced_traffic_controls_enabled ?? false;
    }
    return true;
  });
}

export function getProfileSections(
  capabilities?: Pick<
    AdminSystemCapabilities,
    "billing_enabled" | "advanced_traffic_controls_enabled"
  > | null
): ProfileSection[] {
  const sections: ProfileSection[] = [];
  if (capabilities?.billing_enabled) {
    sections.push("billing");
  }
  sections.push("playback", "social");
  if (capabilities?.advanced_traffic_controls_enabled) {
    sections.push("traffic");
  }
  return sections;
}

export function getDefaultProfileSection(
  capabilities?: Pick<
    AdminSystemCapabilities,
    "billing_enabled" | "advanced_traffic_controls_enabled"
  > | null
): ProfileSection {
  return getProfileSections(capabilities)[0] ?? "playback";
}
