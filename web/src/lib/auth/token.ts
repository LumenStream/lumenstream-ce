import type { User } from "@/lib/types/jellyfin";
import { isBrowser } from "@/lib/utils";

const ACCESS_TOKEN_KEY = "ls.access_token";
const USER_KEY = "ls.user";

export interface AuthSession {
  token: string;
  user: User;
}

export interface AuthStorageOptions {
  persistent?: boolean;
}

function pickReadStorage(key: string): Storage | null {
  if (!isBrowser()) {
    return null;
  }

  if (window.sessionStorage.getItem(key)) {
    return window.sessionStorage;
  }

  if (window.localStorage.getItem(key)) {
    return window.localStorage;
  }

  return null;
}

function primaryWriteStorage(options?: AuthStorageOptions): Storage {
  return options?.persistent ? window.localStorage : window.sessionStorage;
}

function secondaryWriteStorage(options?: AuthStorageOptions): Storage {
  return options?.persistent ? window.sessionStorage : window.localStorage;
}

function parseStoredUser(raw: string | null): User | null {
  if (!raw) {
    return null;
  }

  try {
    return JSON.parse(raw) as User;
  } catch {
    return null;
  }
}

export function getAccessToken(): string | null {
  return getAuthSession()?.token || null;
}

export function setAccessToken(token: string, options?: AuthStorageOptions): void {
  if (!isBrowser()) {
    return;
  }

  const primary = primaryWriteStorage(options);
  const secondary = secondaryWriteStorage(options);
  primary.setItem(ACCESS_TOKEN_KEY, token);
  secondary.removeItem(ACCESS_TOKEN_KEY);
}

export function clearAccessToken(): void {
  if (!isBrowser()) {
    return;
  }

  window.sessionStorage.removeItem(ACCESS_TOKEN_KEY);
  window.localStorage.removeItem(ACCESS_TOKEN_KEY);
}

export function setCurrentUser(user: User, options?: AuthStorageOptions): void {
  if (!isBrowser()) {
    return;
  }

  const serialized = JSON.stringify(user);
  const primary = primaryWriteStorage(options);
  const secondary = secondaryWriteStorage(options);
  primary.setItem(USER_KEY, serialized);
  secondary.removeItem(USER_KEY);
}

export function getCurrentUser(): User | null {
  if (!isBrowser()) {
    return null;
  }

  const session = getAuthSession();
  if (session) {
    return session.user;
  }

  const storage = pickReadStorage(USER_KEY);
  if (!storage) {
    return null;
  }

  const user = parseStoredUser(storage.getItem(USER_KEY));
  if (!user) {
    storage.removeItem(USER_KEY);
    return null;
  }
  return user;
}

export function clearCurrentUser(): void {
  if (!isBrowser()) {
    return;
  }

  window.sessionStorage.removeItem(USER_KEY);
  window.localStorage.removeItem(USER_KEY);
}

function readSessionFromStorage(storage: Storage): AuthSession | null {
  const token = storage.getItem(ACCESS_TOKEN_KEY);
  if (!token) {
    return null;
  }

  const user = parseStoredUser(storage.getItem(USER_KEY));
  if (!user) {
    return null;
  }

  return { token, user };
}

export function getAuthSession(): AuthSession | null {
  if (!isBrowser()) {
    return null;
  }

  return (
    readSessionFromStorage(window.sessionStorage) || readSessionFromStorage(window.localStorage)
  );
}

export function clearAuthSession(): void {
  clearAccessToken();
  clearCurrentUser();
}

export function getUserRole(user: User | null): string {
  return user?.Policy?.Role || (user?.Policy?.IsAdministrator ? "Admin" : "Viewer");
}

export function canAccessAdmin(user: User | null): boolean {
  const role = getUserRole(user);
  return role === "Admin";
}

export function isSuperAdmin(user: User | null): boolean {
  return getUserRole(user) === "Admin";
}
