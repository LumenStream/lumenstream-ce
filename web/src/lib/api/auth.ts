import type { AuthResult, User } from "@/lib/types/jellyfin";

import { apiRequest } from "@/lib/api/client";
import {
  mockAuthenticateByName,
  mockGetUserById,
  mockLogout,
  mockRegisterWithInvite,
} from "@/lib/mock/api";
import { runWithMock } from "@/lib/mock/mode";

interface AuthenticateByNamePayload {
  Username: string;
  Pw: string;
}

export async function authenticateByName(username: string, password: string): Promise<AuthResult> {
  const payload: AuthenticateByNamePayload = {
    Username: username,
    Pw: password,
  };

  return runWithMock(
    () => mockAuthenticateByName(username, password),
    () =>
      apiRequest<AuthResult>("/Users/AuthenticateByName", {
        method: "POST",
        auth: false,
        headers: {
          "X-Emby-Client": "ls-web",
          "X-Emby-Device-Name": "ls-web-browser",
          "X-Emby-Device-Id": "ls-web",
        },
        body: JSON.stringify(payload),
      })
  );
}

export async function registerWithInvite(payload: {
  username: string;
  password: string;
  invite_code?: string;
}): Promise<AuthResult> {
  return runWithMock(
    () => mockRegisterWithInvite(payload),
    () =>
      apiRequest<AuthResult>("/api/auth/register", {
        method: "POST",
        auth: false,
        headers: {
          "X-Emby-Client": "ls-web",
          "X-Emby-Device-Name": "ls-web-browser",
          "X-Emby-Device-Id": "ls-web-register",
        },
        body: JSON.stringify(payload),
      })
  );
}

export async function logoutSession(): Promise<void> {
  return runWithMock(
    () => mockLogout(),
    () =>
      apiRequest<void>("/Sessions/Logout", {
        method: "POST",
      })
  );
}

export async function getUserById(userId: string): Promise<User> {
  return runWithMock(
    () => mockGetUserById(userId),
    () => apiRequest<User>(`/Users/${userId}`)
  );
}
