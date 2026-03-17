/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createApiKey, deleteApiKey, listApiKeys } from "@/lib/api/admin";

import { AdminApiKeysPanel } from "./AdminApiKeysPanel";

const mockAuthState = {
  ready: true,
};

vi.mock("@/lib/auth/use-auth-session", () => ({
  useAuthSession: () => mockAuthState,
}));

vi.mock("@/components/domain/DataState", () => ({
  LoadingState: () => React.createElement("div", null, "loading"),
  ErrorState: () => React.createElement("div", null, "error"),
}));

vi.mock("@/lib/api/admin", () => ({
  listApiKeys: vi.fn(),
  createApiKey: vi.fn(),
  deleteApiKey: vi.fn(),
}));

const mockListApiKeys = vi.mocked(listApiKeys);
const mockCreateApiKey = vi.mocked(createApiKey);
const mockDeleteApiKey = vi.mocked(deleteApiKey);

async function flushEffects() {
  await Promise.resolve();
  await Promise.resolve();
}

describe("AdminApiKeysPanel", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);

    mockListApiKeys.mockResolvedValue([
      {
        id: "key-001",
        name: "deployment",
        created_at: "2026-02-16T00:00:00Z",
        last_used_at: null,
      },
    ]);
    mockCreateApiKey.mockResolvedValue({
      id: "key-002",
      name: "new-key",
      api_key: "ls_demo_key",
      created_at: "2026-02-16T00:00:00Z",
    });
    mockDeleteApiKey.mockResolvedValue(undefined);
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

  it("shows a newly created api key in the table until the page reloads", async () => {
    mockListApiKeys
      .mockResolvedValueOnce([
        {
          id: "key-001",
          name: "deployment",
          created_at: "2026-02-16T00:00:00Z",
          last_used_at: null,
        },
      ])
      .mockResolvedValue([
        {
          id: "key-002",
          name: "new-key",
          created_at: "2026-02-16T00:00:00Z",
          last_used_at: null,
        },
        {
          id: "key-001",
          name: "deployment",
          created_at: "2026-02-16T00:00:00Z",
          last_used_at: null,
        },
      ]);

    await act(async () => {
      root.render(<AdminApiKeysPanel />);
      await flushEffects();
    });

    const nameInput = container.querySelector('input[placeholder="Key 名称"]');
    expect(nameInput).not.toBeNull();

    await act(async () => {
      if (!(nameInput instanceof HTMLInputElement)) {
        throw new Error("missing name input");
      }
      nameInput.value = "new-key";
      nameInput.dispatchEvent(new Event("input", { bubbles: true }));
      await flushEffects();
    });

    const form = container.querySelector("form");
    expect(form).not.toBeNull();

    await act(async () => {
      form?.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
      await flushEffects();
    });

    expect(container.textContent).toContain("ls_demo_key");
    expect(container.textContent).toContain("new-key");

    await act(async () => {
      root.unmount();
    });
    root = createRoot(container);

    await act(async () => {
      root.render(<AdminApiKeysPanel />);
      await flushEffects();
    });

    expect(container.textContent).toContain("new-key");
    expect(container.textContent).toContain("****");
    expect(container.textContent).not.toContain("ls_demo_key");
  });

  it("requires explicit confirmation before deleting an API key", async () => {
    await act(async () => {
      root.render(<AdminApiKeysPanel />);
      await flushEffects();
    });

    const deleteButton = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "删除"
    );
    expect(deleteButton).not.toBeUndefined();

    await act(async () => {
      deleteButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(mockDeleteApiKey).not.toHaveBeenCalled();
    expect(document.body.textContent).toContain("确认删除 API Key");

    const confirmButton = Array.from(document.body.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "确认删除"
    );
    expect(confirmButton).not.toBeUndefined();

    await act(async () => {
      confirmButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushEffects();
    });

    expect(mockDeleteApiKey).toHaveBeenCalledWith("key-001");
  });
});
