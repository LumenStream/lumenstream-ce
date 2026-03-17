import { clearAuthSession, getAuthSession, setAccessToken, setCurrentUser } from "@/lib/auth/token";
import { mockAuthenticateByName } from "@/lib/mock/api";
import { isMockFeatureEnabled, isMockMode, setMockMode } from "@/lib/mock/mode";

export async function enableMockExperience(
  username = "demo-admin",
  options?: { persistent?: boolean }
): Promise<void> {
  if (!isMockFeatureEnabled()) {
    throw new Error("mock feature disabled");
  }

  setMockMode(true);
  const auth = await mockAuthenticateByName(username, "mock-password");
  setAccessToken(auth.AccessToken, options);
  setCurrentUser(auth.User, options);
}

export async function ensureMockSession(): Promise<void> {
  if (!isMockFeatureEnabled()) {
    return;
  }

  if (!isMockMode()) {
    return;
  }

  if (getAuthSession()) {
    return;
  }

  await enableMockExperience();
}

export function disableMockExperience(): void {
  setMockMode(false);
  clearAuthSession();
}
