import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { canAccessAdmin, clearAuthSession, getCurrentUser } from "@/lib/auth/token";
import {
  buildHeaderAccountQuickLinks,
  buildHeaderAvatarDisplay,
  type HeaderAccountQuickLink,
} from "@/lib/navigation/header-account-entry";
import type { User } from "@/lib/types/jellyfin";

interface Props {
  extraLink?: HeaderAccountQuickLink | null;
}

export default function HeaderAccountEntry({ extraLink = null }: Props) {
  const [user, setUser] = useState<User | null>(null);
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setUser(getCurrentUser());
  }, []);

  const avatar = useMemo(() => buildHeaderAvatarDisplay(user), [user]);
  const isAdmin = useMemo(() => canAccessAdmin(user), [user]);
  const resolvedExtra = isAdmin
    ? extraLink
    : extraLink?.href?.startsWith("/admin")
      ? null
      : extraLink;
  const quickLinks = useMemo(() => buildHeaderAccountQuickLinks(resolvedExtra), [resolvedExtra]);

  const closeMenu = useCallback(() => setIsOpen(false), []);
  const handleQuickLinkClick = useCallback(
    (href: string) => {
      if (href.startsWith("/login")) {
        clearAuthSession();
      }
      closeMenu();
    },
    [closeMenu]
  );

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
      const firstLink = menuRef.current.querySelector("a");
      firstLink?.focus();
    }
  }, [isOpen]);

  function handleKeyNavigation(event: React.KeyboardEvent, index: number) {
    const links = menuRef.current?.querySelectorAll("a");
    if (!links) return;

    if (event.key === "ArrowDown") {
      event.preventDefault();
      const next = links[index + 1] || links[0];
      (next as HTMLElement).focus();
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      const prev = links[index - 1] || links[links.length - 1];
      (prev as HTMLElement).focus();
    }
  }

  return (
    <div ref={containerRef} className="relative">
      <button
        type="button"
        onClick={() => setIsOpen((prev) => !prev)}
        aria-expanded={isOpen}
        aria-haspopup="menu"
        aria-label={`${avatar.displayName} 账户菜单`}
        className="border-border text-muted-foreground hover:text-foreground focus-visible:ring-ring light:hover:border-black/20 flex cursor-pointer items-center gap-2 rounded-md border px-3 py-2 text-sm transition-colors hover:border-white/30 focus-visible:ring-2 focus-visible:outline-none"
      >
        {avatar.imageUrl ? (
          <img
            src={avatar.imageUrl}
            alt=""
            aria-hidden="true"
            className="border-border h-7 w-7 rounded-full border object-cover"
            loading="lazy"
          />
        ) : (
          <span
            aria-hidden="true"
            className="border-border bg-background text-foreground flex h-7 w-7 items-center justify-center rounded-full border text-xs font-semibold"
          >
            {avatar.fallbackInitial}
          </span>
        )}
        <span className="hidden text-sm sm:inline">{avatar.displayName}</span>
        <svg
          aria-hidden="true"
          className={`h-4 w-4 transition-transform ${isOpen ? "rotate-180" : ""}`}
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      {isOpen && (
        <div
          ref={menuRef}
          role="menu"
          aria-label="账户快捷入口"
          className="light:border-black/[0.08] light:bg-white/95 light:shadow-black/10 absolute right-0 z-10 mt-2 w-44 rounded-md border border-white/[0.08] bg-neutral-900/95 p-2 shadow-2xl backdrop-blur-xl"
        >
          <p id="account-menu-label" className="text-muted-foreground px-2 py-1 text-xs">
            账户快捷入口
          </p>
          <nav aria-labelledby="account-menu-label" className="mt-1 flex flex-col gap-1">
            {quickLinks.map((link, index) => (
              <a
                key={link.href}
                href={link.href}
                role="menuitem"
                tabIndex={0}
                onClick={() => handleQuickLinkClick(link.href)}
                onKeyDown={(e) => handleKeyNavigation(e, index)}
                className="text-foreground hover:border-border hover:bg-background focus-visible:ring-ring rounded-md border border-transparent px-2 py-1.5 text-sm transition-colors focus-visible:ring-2 focus-visible:outline-none"
              >
                {link.label}
              </a>
            ))}
          </nav>
        </div>
      )}
    </div>
  );
}
