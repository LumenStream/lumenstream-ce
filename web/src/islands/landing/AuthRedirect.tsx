import { useEffect } from "react";

import { getAuthSession } from "@/lib/auth/token";
import { isBrowser } from "@/lib/utils";

export function AuthRedirect() {
  useEffect(() => {
    if (!isBrowser()) {
      return;
    }

    const session = getAuthSession();
    if (session) {
      window.location.replace("/app/home");
    }
  }, []);

  return null;
}
