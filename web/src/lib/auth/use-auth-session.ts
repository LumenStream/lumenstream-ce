import { useEffect, useMemo, useState } from "react";

import { clearAuthSession, getAuthSession, type AuthSession } from "@/lib/auth/token";
import { ensureMockSession } from "@/lib/mock/session";

export function useAuthSession(options?: { requireAdmin?: boolean }) {
  const [session, setSession] = useState<AuthSession | null>(null);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    let cancelled = false;

    async function resolveSession() {
      await ensureMockSession();

      const current = getAuthSession();
      if (!current) {
        window.location.replace("/login");
        return;
      }

      if (options?.requireAdmin) {
        const role =
          current.user.Policy?.Role || (current.user.Policy?.IsAdministrator ? "Admin" : "Viewer");
        if (role !== "Admin") {
          clearAuthSession();
          window.location.replace("/app/home");
          return;
        }
      }

      if (!cancelled) {
        setSession(current);
        setReady(true);
      }
    }

    void resolveSession();

    return () => {
      cancelled = true;
    };
  }, [options?.requireAdmin]);

  return useMemo(
    () => ({
      session,
      ready,
    }),
    [ready, session]
  );
}
