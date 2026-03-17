export interface AppSidebarItem {
  id: string;
  label: string;
  href: string;
  icon?: "home" | "search" | "user" | "settings" | "admin";
}

export const appSidebarItems: readonly AppSidebarItem[] = [
  {
    id: "home",
    label: "首页",
    href: "/app/home",
    icon: "home",
  },
  {
    id: "search",
    label: "搜索",
    href: "/app/search",
    icon: "search",
  },
  {
    id: "profile",
    label: "账户",
    href: "/app/profile",
    icon: "user",
  },
] as const;

export const appSidebarQuickLinks: readonly AppSidebarItem[] = [
  {
    id: "admin",
    label: "管理后台",
    href: "/admin/overview",
    icon: "admin",
  },
  {
    id: "landing",
    label: "返回首页",
    href: "/",
  },
] as const;

export function getAppSidebarItems(): readonly AppSidebarItem[] {
  return appSidebarItems;
}

export function getAppSidebarQuickLinks(): readonly AppSidebarItem[] {
  return appSidebarQuickLinks;
}
