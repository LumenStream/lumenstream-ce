/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { GlobalSearchBox } from "./GlobalSearchBox";

describe("GlobalSearchBox", () => {
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

  it("renders named search controls with accessible labels", () => {
    act(() => {
      root.render(<GlobalSearchBox />);
    });

    const form = container.querySelector("form[role='search']");
    expect(form?.getAttribute("aria-label")).toBe("全局搜索");

    const searchInput = container.querySelector("input[name='q']") as HTMLInputElement | null;
    expect(searchInput).not.toBeNull();
    expect(searchInput?.id).not.toBe("");
    expect(container.querySelector(`label[for='${searchInput?.id}']`)).not.toBeNull();

    const typeSelect = container.querySelector("select[name='type']") as HTMLSelectElement | null;
    expect(typeSelect).not.toBeNull();
    expect(typeSelect?.id).not.toBe("");
    expect(container.querySelector(`label[for='${typeSelect?.id}']`)).not.toBeNull();

    const submitButton = container.querySelector("button[type='submit']");
    expect(submitButton?.getAttribute("aria-label")).toBe("执行搜索");
  });

  it("focuses search input with Ctrl/Cmd+K shortcut", () => {
    act(() => {
      root.render(<GlobalSearchBox />);
    });

    const searchInput = container.querySelector("input[name='q']") as HTMLInputElement;
    expect(searchInput).toBeDefined();

    act(() => {
      document.dispatchEvent(
        new KeyboardEvent("keydown", {
          key: "k",
          ctrlKey: true,
          bubbles: true,
        })
      );
    });

    expect(document.activeElement).toBe(searchInput);
  });
});
