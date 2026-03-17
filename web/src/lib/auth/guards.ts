import { canAccessAdmin, getAuthSession } from "@/lib/auth/token";

export function ensureSignedIn(redirectTo = "/login"): void {
  const session = getAuthSession();
  if (!session) {
    window.location.replace(redirectTo);
  }
}

export function ensureAdminAccess(redirectTo = "/app/home"): void {
  const session = getAuthSession();
  if (!session) {
    window.location.replace("/login");
    return;
  }

  if (!canAccessAdmin(session.user)) {
    window.location.replace(redirectTo);
  }
}
