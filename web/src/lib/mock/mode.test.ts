/**
 * @vitest-environment jsdom
 */

import { afterEach, describe, expect, it, vi } from "vitest";

import { isMockFeatureEnabled, isMockMode, runWithMock, setMockMode } from "./mode";

const env = import.meta.env as Record<string, string | undefined>;
const originalEnableMock = env.PUBLIC_LS_ENABLE_MOCK;
const originalMockMode = env.PUBLIC_LS_MOCK_MODE;

afterEach(() => {
  env.PUBLIC_LS_ENABLE_MOCK = originalEnableMock;
  env.PUBLIC_LS_MOCK_MODE = originalMockMode;
  window.sessionStorage.clear();
});

describe("mock mode feature gate", () => {
  it("follows dev default when no override is set", () => {
    env.PUBLIC_LS_ENABLE_MOCK = undefined;

    expect(isMockFeatureEnabled()).toBe(import.meta.env.DEV);
  });

  it("keeps mock mode disabled when feature flag is off", () => {
    env.PUBLIC_LS_ENABLE_MOCK = "false";
    env.PUBLIC_LS_MOCK_MODE = "true";
    window.sessionStorage.setItem("ls.mock_mode", "1");

    expect(isMockFeatureEnabled()).toBe(false);
    expect(isMockMode()).toBe(false);

    setMockMode(true);
    expect(window.sessionStorage.getItem("ls.mock_mode")).toBeNull();
  });

  it("does not fallback to mock when feature flag is off", async () => {
    env.PUBLIC_LS_ENABLE_MOCK = "false";

    const mockFn = vi.fn(async () => "mock-result");
    const realFn = vi.fn(async () => {
      throw new TypeError("network down");
    });

    await expect(runWithMock(mockFn, realFn)).rejects.toThrow("network down");
    expect(realFn).toHaveBeenCalledTimes(1);
    expect(mockFn).not.toHaveBeenCalled();
    expect(window.sessionStorage.getItem("ls.mock_mode")).toBeNull();
  });

  it("falls back to mock when enabled and real request has network failure", async () => {
    env.PUBLIC_LS_ENABLE_MOCK = "true";
    env.PUBLIC_LS_MOCK_MODE = "false";

    const mockFn = vi.fn(async () => "mock-result");
    const realFn = vi.fn(async () => {
      throw new TypeError("fetch failed");
    });

    await expect(runWithMock(mockFn, realFn)).resolves.toBe("mock-result");
    expect(realFn).toHaveBeenCalledTimes(1);
    expect(mockFn).toHaveBeenCalledTimes(1);
    expect(window.sessionStorage.getItem("ls.mock_mode")).toBe("1");
  });
});
