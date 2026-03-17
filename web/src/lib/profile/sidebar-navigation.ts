export interface ProfileSidebarNavigationItem {
  id: "admin-overview" | "landing";
  label: string;
  href: "/admin/overview" | "/";
}

export const profileSidebarNavigationItems = [
  {
    id: "admin-overview",
    label: "进入管理端",
    href: "/admin/overview",
  },
  {
    id: "landing",
    label: "返回首页",
    href: "/",
  },
] as const satisfies readonly ProfileSidebarNavigationItem[];

export function getProfileSidebarNavigationItems(): readonly ProfileSidebarNavigationItem[] {
  return profileSidebarNavigationItems;
}
