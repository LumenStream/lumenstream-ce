/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { SearchCenter } from "./SearchCenter";

const getItemsMock = vi.fn();

const mockAuthState = {
  ready: true,
  session: {
    token: "token-1",
    user: { Id: "user-1", Name: "tester" },
  },
};

vi.mock("@/lib/auth/use-auth-session", () => ({
  useAuthSession: () => mockAuthState,
}));

vi.mock("@/lib/api/items", () => ({
  getItems: (...args: unknown[]) => getItemsMock(...args),
}));

vi.mock("@/components/domain/PosterItemCard", () => ({
  PosterItemCard: ({ item }: { item: { Id: string; Name?: string } }) =>
    React.createElement("div", { "data-testid": `item-${item.Id}` }, item.Name || item.Id),
}));

vi.mock("@/components/domain/DataState", () => ({
  EmptyState: ({ title }: { title: string }) => React.createElement("div", null, title),
  ErrorState: ({ title }: { title: string }) => React.createElement("div", null, title),
  LoadingState: ({ title }: { title?: string }) => React.createElement("div", null, title || ""),
}));

function setNativeValue(element: HTMLInputElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
  setter?.call(element, value);
}

async function flushEffects() {
  await Promise.resolve();
  await Promise.resolve();
}

describe("SearchCenter", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    (globalThis as typeof globalThis & { React?: typeof React }).React = React;
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    getItemsMock.mockResolvedValue({ Items: [], TotalRecordCount: 0, StartIndex: 0 });
  });

  afterEach(() => {
    act(() => {
      root.unmount();
    });
    container.remove();
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = undefined;
    vi.clearAllMocks();
  });

  it("runs full search when clicking submit while suggestions are open", async () => {
    await act(async () => {
      root.render(<SearchCenter />);
      await flushEffects();
    });

    const input = container.querySelector(
      "input[placeholder='输入片名、剧名、演职员...']"
    ) as HTMLInputElement | null;
    const submitButton = container.querySelector(
      "form button[type='submit']"
    ) as HTMLButtonElement | null;

    expect(input).not.toBeNull();
    expect(submitButton).not.toBeNull();

    await act(async () => {
      if (!input) {
        return;
      }
      setNativeValue(input, "教父");
      input.dispatchEvent(new Event("input", { bubbles: true }));
      await flushEffects();
    });

    await act(async () => {
      if (!submitButton) {
        return;
      }
      submitButton.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
      submitButton.click();
      await flushEffects();
    });

    expect(getItemsMock).toHaveBeenCalledWith(
      expect.objectContaining({
        searchTerm: "教父",
        includeItemTypes: "Movie,Series,Person",
        limit: 60,
        startIndex: 0,
      })
    );
  });

  it("runs initial person poster-wall search with default all scope", async () => {
    await act(async () => {
      root.render(<SearchCenter initialPersonId="person-actor-1" initialPersonName="Al Pacino" />);
      await flushEffects();
    });

    expect(getItemsMock).toHaveBeenCalledWith(
      expect.objectContaining({
        searchTerm: undefined,
        includeItemTypes: "Movie,Series,Person",
        personIds: "person-actor-1",
        limit: 60,
        startIndex: 0,
      })
    );

    expect(container.textContent || "").toContain("人物筛选");
    expect(container.textContent || "").toContain("Al Pacino");
  });

  it("supports searching by person-only type filter", async () => {
    await act(async () => {
      root.render(<SearchCenter />);
      await flushEffects();
    });

    const input = container.querySelector(
      "input[placeholder='输入片名、剧名、演职员...']"
    ) as HTMLInputElement | null;
    const typeSelect = container.querySelector("form select") as HTMLSelectElement | null;
    const submitButton = container.querySelector(
      "form button[type='submit']"
    ) as HTMLButtonElement | null;

    expect(input).not.toBeNull();
    expect(typeSelect).not.toBeNull();
    expect(submitButton).not.toBeNull();

    await act(async () => {
      if (!input || !typeSelect) {
        return;
      }
      setNativeValue(input, "张艺谋");
      input.dispatchEvent(new Event("input", { bubbles: true }));
      typeSelect.value = "Person";
      typeSelect.dispatchEvent(new Event("change", { bubbles: true }));
      await flushEffects();
    });

    await act(async () => {
      if (!submitButton) {
        return;
      }
      submitButton.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
      submitButton.click();
      await flushEffects();
    });

    expect(getItemsMock).toHaveBeenCalledWith(
      expect.objectContaining({
        searchTerm: "张艺谋",
        includeItemTypes: "Person",
        limit: 60,
        startIndex: 0,
      })
    );
  });
});
