import React from "react";

import { cn } from "@/lib/utils";

interface BrandLockupProps {
  className?: string;
  title?: string;
  subtitle?: string;
}

export function BrandLockup({ className, title = "LumenStream Web", subtitle }: BrandLockupProps) {
  return (
    <a
      href="/app/home"
      aria-label="返回 LumenStream 首页"
      className={cn(
        "group light:hover:bg-black/4 inline-flex min-w-0 items-center gap-3 rounded-xl px-1 py-1 transition-colors hover:bg-white/6",
        className
      )}
    >
      <span className="light:bg-black/8 light:border-black/6 relative flex h-9 w-9 flex-shrink-0 items-center justify-center overflow-hidden rounded-xl border border-white/10 bg-white/12 shadow-[0_10px_24px_rgba(14,165,233,0.18)]">
        <span className="absolute inset-0 bg-[radial-gradient(circle_at_top,_rgba(34,211,238,0.28),_rgba(15,23,42,0.02)_70%)] opacity-90" />
        <img
          src="/brand/logo.svg"
          alt="LumenStream Logo"
          className="relative h-6 w-6 transition-transform duration-300 group-hover:scale-105"
        />
      </span>

      <span className="flex min-w-0 flex-col">
        <span className="light:text-foreground truncate text-xs font-semibold tracking-[0.12em] text-white/92 uppercase sm:text-sm">
          {title}
        </span>
        {subtitle ? (
          <span className="light:text-black/45 text-[11px] text-white/58 sm:text-xs">
            {subtitle}
          </span>
        ) : null}
      </span>
    </a>
  );
}
