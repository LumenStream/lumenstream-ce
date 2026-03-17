import { useEffect, useState, type ReactElement } from "react";

import { canAccessAdmin, getCurrentUser } from "@/lib/auth/token";
import {
  getAppSidebarItems,
  getAppSidebarQuickLinks,
  type AppSidebarItem,
} from "@/lib/navigation/app-sidebar";

interface Props {
  active?: "home" | "search" | "profile";
}

const iconMap: Record<string, ReactElement> = {
  home: (
    <svg
      className="h-4 w-4"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6"
      />
    </svg>
  ),
  search: (
    <svg
      className="h-4 w-4"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
      />
    </svg>
  ),
  user: (
    <svg
      className="h-4 w-4"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"
      />
    </svg>
  ),
  admin: (
    <svg
      className="h-4 w-4"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
      />
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
      />
    </svg>
  ),
};

function NavIcon({ icon }: { icon?: string }) {
  if (!icon || !iconMap[icon]) {
    return <span className="h-4 w-4" />;
  }
  return iconMap[icon];
}

function NavItem({ item, isActive }: { item: AppSidebarItem; isActive: boolean }) {
  return (
    <a
      href={item.href}
      aria-current={isActive ? "page" : undefined}
      className={`focus-visible:ring-ring flex items-center gap-3 rounded-md border px-3 py-2 text-sm font-medium transition-colors focus-visible:ring-2 focus-visible:outline-none ${
        isActive
          ? "border-rose-700 bg-rose-900/30 text-rose-100"
          : "text-muted-foreground hover:border-border hover:bg-background/60 hover:text-foreground border-transparent"
      }`}
    >
      <NavIcon icon={item.icon} />
      {item.label}
    </a>
  );
}

export function AppSidebar({ active }: Props) {
  const mainItems = getAppSidebarItems();
  const allQuickLinks = getAppSidebarQuickLinks();
  const [isAdmin, setIsAdmin] = useState(false);

  useEffect(() => {
    setIsAdmin(canAccessAdmin(getCurrentUser()));
  }, []);

  const quickLinks = isAdmin
    ? allQuickLinks
    : allQuickLinks.filter((item) => !item.href.startsWith("/admin"));

  return (
    <aside className="flex w-56 shrink-0 flex-col gap-4">
      <nav aria-label="主导航" className="border-border bg-card shadow-card rounded-xl border p-3">
        <p className="text-muted-foreground mb-2 px-3 text-xs font-medium tracking-wide uppercase">
          导航
        </p>
        <div className="flex flex-col gap-1">
          {mainItems.map((item) => (
            <NavItem key={item.id} item={item} isActive={active === item.id} />
          ))}
        </div>
      </nav>

      <nav
        aria-label="快捷入口"
        className="border-border bg-card shadow-card rounded-xl border p-3"
      >
        <p className="text-muted-foreground mb-2 px-3 text-xs font-medium tracking-wide uppercase">
          快捷入口
        </p>
        <div className="flex flex-col gap-1">
          {quickLinks.map((item) => (
            <NavItem key={item.id} item={item} isActive={false} />
          ))}
        </div>
      </nav>
    </aside>
  );
}
