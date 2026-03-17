/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import * as tokenModule from "@/lib/auth/token";

import HeaderAccountEntry from "./HeaderAccountEntry";

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

describe("HeaderAccountEntry", () => {
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

  it("clears auth session when clicking switch-account quick link", () => {
    const clearSpy = vi.spyOn(tokenModule, "clearAuthSession").mockImplementation(() => undefined);
    vi.spyOn(tokenModule, "getCurrentUser").mockReturnValue(DEMO_USER);
    vi.spyOn(tokenModule, "canAccessAdmin").mockReturnValue(false);

    act(() => {
      root.render(<HeaderAccountEntry />);
    });

    const menuButton = container.querySelector("button[aria-haspopup='menu']");
    expect(menuButton).not.toBeNull();

    act(() => {
      menuButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    const switchAccountLink = container.querySelector("a[href='/login']");
    expect(switchAccountLink).not.toBeNull();

    act(() => {
      switchAccountLink?.dispatchEvent(
        new MouseEvent("click", { bubbles: true, cancelable: true })
      );
    });

    expect(clearSpy).toHaveBeenCalledTimes(1);
  });
});
