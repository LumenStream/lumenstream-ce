/**
 * @vitest-environment jsdom
 */

import { afterEach, describe, expect, it } from "vitest";

import { getApiBaseUrl } from "@/lib/api/client";

const env = import.meta.env as Record<string, string | undefined>;
const originalApiBaseUrl = env.PUBLIC_LS_API_BASE_URL;
const originalRuntimeConfig = window.__LS_CONFIG__;

afterEach(() => {
  env.PUBLIC_LS_API_BASE_URL = originalApiBaseUrl;
  window.__LS_CONFIG__ = originalRuntimeConfig;
});

describe("getApiBaseUrl", () => {
  it("prefers runtime config apiBaseUrl when provided", () => {
    env.PUBLIC_LS_API_BASE_URL = "https://build.example.com";
    window.__LS_CONFIG__ = { apiBaseUrl: "https://runtime.example.com/" };

    expect(getApiBaseUrl()).toBe("https://runtime.example.com");
  });

  it("falls back to build-time env when runtime config is absent", () => {
    env.PUBLIC_LS_API_BASE_URL = "https://build.example.com";
    window.__LS_CONFIG__ = undefined;

    expect(getApiBaseUrl()).toBe("https://build.example.com");
  });

  it("uses default base url when runtime config and env are both missing", () => {
    env.PUBLIC_LS_API_BASE_URL = undefined;
    window.__LS_CONFIG__ = undefined;

    expect(getApiBaseUrl()).toBe("http://127.0.0.1:8096");
  });
});
