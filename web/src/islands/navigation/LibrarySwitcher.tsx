import { useEffect, useRef, useState } from "react";

import { getRootItemsShared } from "@/lib/api/items";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import type { BaseItem } from "@/lib/types/jellyfin";

export function LibrarySwitcher() {
  const { session, ready } = useAuthSession();
  const [libraries, setLibraries] = useState<BaseItem[]>([]);
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  const currentId =
    typeof window !== "undefined"
      ? window.location.pathname.match(/\/app\/library\/([^/]+)/)?.[1]
      : undefined;
  const current = libraries.find((l) => l.Id === currentId);

  useEffect(() => {
    if (!ready || !session) return;
    getRootItemsShared(session.user.Id).then((r) => setLibraries(r.Items));
  }, [ready, session]);

  useEffect(() => {
    if (!open) return;
    function onClickOutside(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    }
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") setOpen(false);
    }
    document.addEventListener("mousedown", onClickOutside);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("mousedown", onClickOutside);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open]);

  if (!libraries.length) return null;

  return (
    <div ref={ref} className="relative hidden md:block">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="border-border text-muted-foreground hover:text-foreground light:hover:border-black/20 flex cursor-pointer items-center gap-1.5 rounded-md border px-3 py-1.5 text-sm transition-colors hover:border-white/30"
      >
        <svg
          className="h-3.5 w-3.5 shrink-0"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          strokeWidth={1.5}
        >
          <path strokeLinecap="round" strokeLinejoin="round" d="M3 7h18M3 12h18M3 17h18" />
        </svg>
        <span className="max-w-[7rem] truncate">{current?.Name ?? "媒体库"}</span>
        <svg
          className={`h-3.5 w-3.5 shrink-0 transition-transform ${open ? "rotate-180" : ""}`}
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      {open && (
        <div className="light:border-black/[0.08] light:bg-white/95 light:shadow-black/10 absolute left-0 z-50 mt-2 min-w-[10rem] rounded-md border border-white/[0.08] bg-neutral-900/95 p-1.5 shadow-2xl backdrop-blur-xl">
          {libraries.map((lib) => (
            <a
              key={lib.Id}
              href={`/app/library/${lib.Id}`}
              className={`block rounded px-3 py-1.5 text-sm transition-colors ${
                lib.Id === currentId
                  ? "text-foreground light:bg-black/[0.06] bg-white/10"
                  : "text-muted-foreground hover:text-foreground light:hover:bg-black/[0.04] hover:bg-white/[0.06]"
              }`}
            >
              {lib.Name}
            </a>
          ))}
        </div>
      )}
    </div>
  );
}
