/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Modal } from "./Modal";

describe("Modal", () => {
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

  it("adds dialog semantics and locks body scroll while open", () => {
    const onClose = vi.fn();

    act(() => {
      root.render(
        <Modal open title="测试标题" description="测试描述" onClose={onClose}>
          <button type="button">Action</button>
        </Modal>
      );
    });

    const dialog = document.body.querySelector("[role='dialog']");
    expect(dialog).not.toBeNull();
    expect(dialog?.getAttribute("aria-modal")).toBe("true");
    expect(container.querySelector("[role='dialog']")).toBeNull();
    expect(document.body.style.overflow).toBe("hidden");

    act(() => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("restores body scroll and focus when closed", () => {
    const launcher = document.createElement("button");
    launcher.type = "button";
    launcher.textContent = "launcher";
    document.body.appendChild(launcher);
    launcher.focus();

    act(() => {
      root.render(
        <Modal open title="测试标题" onClose={() => undefined}>
          <p>内容</p>
        </Modal>
      );
    });

    expect(document.body.style.overflow).toBe("hidden");

    act(() => {
      root.render(
        <Modal open={false} title="测试标题" onClose={() => undefined}>
          <p>内容</p>
        </Modal>
      );
    });

    expect(document.body.style.overflow).toBe("");
    expect(document.activeElement).toBe(launcher);

    launcher.remove();
  });

  it("supports header close button without rendering footer close action", () => {
    const onClose = vi.fn();

    act(() => {
      root.render(
        <Modal
          open
          title="测试标题"
          description="测试描述"
          onClose={onClose}
          showHeaderClose
          showFooterClose={false}
        >
          <p>内容</p>
        </Modal>
      );
    });

    const headerCloseButton = document.body.querySelector(
      "button[aria-label='关闭弹窗']"
    ) as HTMLButtonElement | null;
    expect(headerCloseButton).not.toBeNull();

    const footerCloseButton = Array.from(document.body.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "关闭"
    );
    expect(footerCloseButton).toBeUndefined();

    act(() => {
      headerCloseButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("applies optional overlay/card/content class overrides", () => {
    act(() => {
      root.render(
        <Modal
          open
          title="测试标题"
          onClose={() => undefined}
          overlayClassName="test-overlay bg-black/50"
          cardClassName="test-card min-h-[20rem]"
          contentClassName="test-content overflow-hidden"
        >
          <p>内容</p>
        </Modal>
      );
    });

    const overlay = document.body.querySelector("div.fixed.inset-0") as HTMLDivElement | null;
    expect(overlay?.className).toContain("test-overlay");
    expect(overlay?.className).toContain("bg-black/50");

    const card = document.body.querySelector("[role='dialog'] > div") as HTMLDivElement | null;
    expect(card?.className).toContain("test-card");
    expect(card?.className).toContain("min-h-[20rem]");

    const content = document.body.querySelector(".test-content") as HTMLDivElement | null;
    expect(content?.className).toContain("overflow-hidden");
  });
});
