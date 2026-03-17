import { useEffect, useState } from "react";

import { getPublicSystemCapabilities } from "@/lib/api/system";
import { getAdminNavItems, type AdminNavItemId } from "@/lib/edition/capabilities";
import type { AdminSystemCapabilities } from "@/lib/types/admin";

interface Props {
  active?: AdminNavItemId;
}

export function AdminNavigation({ active = "overview" }: Props) {
  const [capabilities, setCapabilities] = useState<AdminSystemCapabilities | null>(null);

  useEffect(() => {
    let cancelled = false;

    getPublicSystemCapabilities()
      .then((payload) => {
        if (!cancelled) {
          setCapabilities(payload);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setCapabilities(null);
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  const navItems = getAdminNavItems(capabilities);

  return (
    <>
      <nav aria-label="管理后台导航" className="mb-4 flex gap-2 overflow-x-auto pb-1 lg:hidden">
        {navItems.map((item) => (
          <a
            key={item.id}
            href={item.href}
            aria-current={active === item.id ? "page" : undefined}
            className={
              active === item.id
                ? "focus-visible:ring-ring light:border-black/[0.08] light:bg-black/[0.04] light:text-foreground shrink-0 rounded-lg border border-white/[0.08] bg-white/[0.06] px-3 py-2 text-sm font-medium whitespace-nowrap text-white transition-all duration-300 focus-visible:ring-2 focus-visible:outline-none"
                : "text-muted-foreground focus-visible:ring-ring hover:border-border hover:bg-muted/50 hover:text-foreground shrink-0 rounded-lg border border-transparent px-3 py-2 text-sm font-medium whitespace-nowrap transition-all duration-300 focus-visible:ring-2 focus-visible:outline-none"
            }
          >
            {item.label}
          </a>
        ))}
      </nav>

      <aside className="light:border-black/[0.06] light:bg-black/[0.02] hidden rounded-2xl border border-white/[0.03] bg-white/[0.02] p-3 backdrop-blur-xl lg:block">
        <nav aria-label="管理后台导航" className="flex flex-col gap-1.5">
          {navItems.map((item) => (
            <a
              key={item.id}
              href={item.href}
              aria-current={active === item.id ? "page" : undefined}
              className={
                active === item.id
                  ? "focus-visible:ring-ring light:border-black/[0.08] light:bg-black/[0.04] light:text-foreground rounded-lg border border-white/[0.08] bg-white/[0.06] px-3 py-2.5 text-sm font-medium text-white transition-all duration-300 focus-visible:ring-2 focus-visible:outline-none"
                  : "text-muted-foreground focus-visible:ring-ring hover:border-border hover:bg-muted/50 hover:text-foreground rounded-lg border border-transparent px-3 py-2.5 text-sm font-medium transition-all duration-300 focus-visible:ring-2 focus-visible:outline-none"
              }
            >
              {item.label}
            </a>
          ))}
        </nav>
      </aside>
    </>
  );
}
