/**
 * @vitest-environment jsdom
 */

import { beforeEach, describe, expect, it } from "vitest";

import {
  clearAuthSession,
  getAccessToken,
  getAuthSession,
  getCurrentUser,
  setAccessToken,
  setCurrentUser,
} from "@/lib/auth/token";

const DEMO_USER = {
  Id: "user-001",
  Name: "demo-user",
  HasPassword: true,
  ServerId: "server-demo",
  Policy: {
    Role: "Viewer",
    IsAdministrator: false,
    IsDisabled: false,
  },
};

describe("auth token storage", () => {
  beforeEach(() => {
    window.sessionStorage.clear();
    window.localStorage.clear();
  });

  it("stores auth in session storage by default", () => {
    setAccessToken("session-token");
    setCurrentUser(DEMO_USER);

    expect(window.sessionStorage.getItem("ls.access_token")).toBe("session-token");
    expect(window.localStorage.getItem("ls.access_token")).toBeNull();
    expect(getAccessToken()).toBe("session-token");
    expect(getCurrentUser()).toEqual(DEMO_USER);
  });

  it("stores auth in local storage when persistent is true", () => {
    setAccessToken("persistent-token", { persistent: true });
    setCurrentUser(DEMO_USER, { persistent: true });

    expect(window.localStorage.getItem("ls.access_token")).toBe("persistent-token");
    expect(window.sessionStorage.getItem("ls.access_token")).toBeNull();
    expect(getAccessToken()).toBe("persistent-token");
    expect(getAuthSession()).toEqual({ token: "persistent-token", user: DEMO_USER });
  });

  it("prefers session auth when both stores have data", () => {
    window.sessionStorage.setItem("ls.access_token", "session-token");
    window.sessionStorage.setItem("ls.user", JSON.stringify(DEMO_USER));
    window.localStorage.setItem("ls.access_token", "persistent-token");
    window.localStorage.setItem("ls.user", JSON.stringify({ ...DEMO_USER, Name: "fallback" }));

    expect(getAuthSession()).toEqual({ token: "session-token", user: DEMO_USER });
  });

  it("uses token from the storage that has a complete auth session", () => {
    window.sessionStorage.setItem("ls.access_token", "stale-session-token");
    window.localStorage.setItem("ls.access_token", "persistent-token");
    window.localStorage.setItem("ls.user", JSON.stringify(DEMO_USER));

    expect(getAuthSession()).toEqual({ token: "persistent-token", user: DEMO_USER });
    expect(getAccessToken()).toBe("persistent-token");
    expect(getCurrentUser()).toEqual(DEMO_USER);
  });

  it("clears auth from both storages", () => {
    window.sessionStorage.setItem("ls.access_token", "session-token");
    window.sessionStorage.setItem("ls.user", JSON.stringify(DEMO_USER));
    window.localStorage.setItem("ls.access_token", "persistent-token");
    window.localStorage.setItem("ls.user", JSON.stringify(DEMO_USER));

    clearAuthSession();

    expect(window.sessionStorage.getItem("ls.access_token")).toBeNull();
    expect(window.sessionStorage.getItem("ls.user")).toBeNull();
    expect(window.localStorage.getItem("ls.access_token")).toBeNull();
    expect(window.localStorage.getItem("ls.user")).toBeNull();
    expect(getAuthSession()).toBeNull();
  });
});
