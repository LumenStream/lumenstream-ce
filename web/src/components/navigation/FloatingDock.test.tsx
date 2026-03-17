/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { FloatingDock } from "./FloatingDock";
import * as tokenModule from "@/lib/auth/token";

describe("FloatingDock", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => {
      root.unmount();
    });
    container.remove();
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = undefined;
    vi.restoreAllMocks();
  });

  it("hides admin entry for non-admin users", () => {
    vi.spyOn(tokenModule, "canAccessAdmin").mockReturnValue(false);
    vi.spyOn(tokenModule, "getCurrentUser").mockReturnValue(null);

    act(() => {
      root.render(<FloatingDock active="playlists" />);
    });

    const wrapper = container.querySelector("div.fixed");
    expect(wrapper?.className).toContain("safe-area-inset-bottom");
    expect(wrapper?.className).toContain("sm:bottom-5");

    const adminLink = container.querySelector("a[href='/admin/overview']");
    expect(adminLink).toBeNull();

    const activeIndicator = Array.from(container.querySelectorAll("a")).filter((node) =>
      node.className.includes("bg-white/20")
    );
    expect(activeIndicator.length).toBe(1);
  });

  it("shows admin entry for admin users", () => {
    vi.spyOn(tokenModule, "canAccessAdmin").mockReturnValue(true);
    vi.spyOn(tokenModule, "getCurrentUser").mockReturnValue(null);

    act(() => {
      root.render(<FloatingDock active="playlists" />);
    });

    const adminLink = container.querySelector("a[href='/admin/overview']");
    expect(adminLink).not.toBeNull();
  });

  it("uses compact mobile item sizing", () => {
    act(() => {
      root.render(<FloatingDock active="home" />);
    });

    const homeLink = container.querySelector("a[href='/app/home']");
    expect(homeLink?.className).toContain("h-11");
    expect(homeLink?.className).toContain("w-12");
    expect(homeLink?.className).toContain("sm:w-14");
  });
});
