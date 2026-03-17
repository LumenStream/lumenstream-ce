import React, { useState } from "react";

import { canAccessAdmin, getCurrentUser } from "@/lib/auth/token";

interface DockItem {
  id: string;
  label: string;
  href: string;
  icon: React.ReactNode;
}

const dockItems: DockItem[] = [
  {
    id: "home",
    label: "首页",
    href: "/app/home",
    icon: (
      <svg
        className="h-5 w-5"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        strokeWidth={1.5}
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6"
        />
      </svg>
    ),
  },
  {
    id: "search",
    label: "搜索",
    href: "/app/search",
    icon: (
      <svg
        className="h-5 w-5"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        strokeWidth={1.5}
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
        />
      </svg>
    ),
  },
  {
    id: "profile",
    label: "账户",
    href: "/app/profile",
    icon: (
      <svg
        className="h-5 w-5"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        strokeWidth={1.5}
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"
        />
      </svg>
    ),
  },
  {
    id: "playlists",
    label: "收藏",
    href: "/app/playlists",
    icon: (
      <svg
        className="h-5 w-5"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        strokeWidth={1.5}
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01"
        />
      </svg>
    ),
  },
  {
    id: "requests",
    label: "求片",
    href: "/app/requests",
    icon: (
      <svg
        className="h-5 w-5"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        strokeWidth={1.5}
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M8.625 9.75A2.625 2.625 0 1111.25 12.375V13.5m0 3h.008v.008H11.25V16.5z"
        />
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M9 3.75h6A5.25 5.25 0 0120.25 9v6A5.25 5.25 0 0115 20.25H9A5.25 5.25 0 013.75 15V9A5.25 5.25 0 019 3.75z"
        />
      </svg>
    ),
  },
];

interface FloatingDockProps {
  active?: "home" | "search" | "playlists" | "requests" | "profile" | "admin";
}

export function FloatingDock({ active = "home" }: FloatingDockProps) {
  const [showAdmin] = useState(() => canAccessAdmin(getCurrentUser()));

  return (
    <div className="fixed bottom-[calc(env(safe-area-inset-bottom,0px)+0.75rem)] left-1/2 z-[100] -translate-x-1/2 sm:bottom-5">
      <nav className="light:border-black/10 light:bg-white/75 light:shadow-[0_8px_32px_-8px_rgba(0,0,0,0.18)] flex items-center gap-0.5 rounded-full border border-white/10 bg-black/55 px-2.5 py-1.5 shadow-[0_16px_44px_-10px_rgba(0,0,0,0.62)] backdrop-blur-xl sm:px-3 sm:py-2">
        {dockItems.map((item) => {
          const isActive = active === item.id;

          return (
            <a
              key={item.id}
              href={item.href}
              className={`group relative flex h-11 w-12 flex-col items-center justify-center rounded-full transition-all duration-300 sm:w-14 ${
                isActive
                  ? "light:bg-black/[0.10] bg-white/20"
                  : "light:hover:bg-black/[0.04] hover:bg-white/[0.06]"
              }`}
            >
              <div
                className={`flex h-5 w-5 items-center justify-center transition-colors duration-300 ${
                  isActive ? "text-foreground" : "text-muted-foreground group-hover:text-foreground"
                }`}
              >
                {item.icon}
              </div>
              <span
                className={`mt-0.5 text-[10px] font-medium transition-colors duration-300 ${
                  isActive
                    ? "text-foreground"
                    : "text-muted-foreground group-hover:text-foreground/80"
                }`}
              >
                {item.label}
              </span>
            </a>
          );
        })}

        {showAdmin && (
          <>
            {/* Divider */}
            <div className="light:bg-black/10 mx-0.5 h-7 w-px bg-white/12" />

            {/* Admin link */}
            <a
              href="/admin/overview"
              className={`group relative flex h-11 w-12 flex-col items-center justify-center rounded-full transition-all duration-300 sm:w-14 ${
                active === "admin"
                  ? "light:bg-black/[0.10] bg-white/20"
                  : "light:hover:bg-black/[0.04] hover:bg-white/[0.06]"
              }`}
            >
              <div
                className={`flex h-5 w-5 items-center justify-center transition-colors duration-300 ${
                  active === "admin"
                    ? "text-foreground"
                    : "text-muted-foreground group-hover:text-foreground"
                }`}
              >
                <svg
                  className="h-5 w-5"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  strokeWidth={1.5}
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
                  />
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                  />
                </svg>
              </div>
              <span
                className={`mt-0.5 text-[10px] font-medium transition-colors duration-300 ${
                  active === "admin"
                    ? "text-foreground"
                    : "text-muted-foreground group-hover:text-foreground/80"
                }`}
              >
                管理
              </span>
            </a>
          </>
        )}
      </nav>
    </div>
  );
}
