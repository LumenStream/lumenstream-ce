/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { BrandLockup } from "./BrandLockup";

describe("BrandLockup", () => {
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
  });

  it("renders logo asset, copy, and home link", () => {
    act(() => {
      root.render(<BrandLockup />);
    });

    const link = container.querySelector("a") as HTMLAnchorElement | null;
    const image = container.querySelector("img") as HTMLImageElement | null;

    expect(link?.getAttribute("href")).toBe("/app/home");
    expect(link?.getAttribute("aria-label")).toContain("LumenStream");
    expect(image?.getAttribute("src")).toBe("/brand/logo.svg");
    expect(image?.getAttribute("alt")).toBe("LumenStream Logo");
    expect(container.textContent).toContain("LumenStream Web");
  });

  it("supports optional subtitle", () => {
    act(() => {
      root.render(<BrandLockup subtitle="CE 应用界面" />);
    });

    expect(container.textContent).toContain("CE 应用界面");
  });
});
