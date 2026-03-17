/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { LoginForm } from "./LoginForm";

vi.mock("@/lib/api/auth", () => ({
  authenticateByName: vi.fn(),
}));

vi.mock("@/lib/auth/token", () => ({
  setAccessToken: vi.fn(),
  setCurrentUser: vi.fn(),
}));

vi.mock("@/lib/mock/session", () => ({
  enableMockExperience: vi.fn(),
}));

describe("LoginForm", () => {
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
    vi.clearAllMocks();
  });

  it("does not prefill insecure default credentials", () => {
    act(() => {
      root.render(<LoginForm />);
    });

    const usernameInput = container.querySelector("input#username") as HTMLInputElement | null;
    const passwordInput = container.querySelector("input#password") as HTMLInputElement | null;
    const submitButton = container.querySelector("button[type=submit]") as HTMLButtonElement | null;

    expect(usernameInput).not.toBeNull();
    expect(passwordInput).not.toBeNull();
    expect(submitButton).not.toBeNull();
    expect(usernameInput?.value).toBe("");
    expect(passwordInput?.value).toBe("");
    expect(submitButton?.disabled).toBe(true);
  });

  it("shows remember-me option and keeps it unchecked by default", () => {
    act(() => {
      root.render(<LoginForm />);
    });

    const rememberInput = container.querySelector(
      "input[type=checkbox]"
    ) as HTMLInputElement | null;
    expect(rememberInput).not.toBeNull();
    expect(rememberInput?.checked).toBe(false);
  });
});
