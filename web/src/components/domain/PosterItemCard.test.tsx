/**
 * @vitest-environment jsdom
 */

import React, { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { BaseItem } from "@/lib/types/jellyfin";

import { PosterItemCard } from "./PosterItemCard";

vi.mock("@/lib/hooks/use-image-glow", () => ({
  useImageGlow: vi.fn(() => ({ glowColor: "#336699" })),
}));

vi.mock("@/lib/api/items", () => ({
  buildItemImageUrl: vi.fn(() => "https://example.com/fallback.jpg"),
  buildStreamUrl: vi.fn((itemId: string) => `https://example.com/stream/${itemId}`),
  addFavoriteItem: vi.fn(),
  removeFavoriteItem: vi.fn(),
}));

vi.mock("@/lib/notifications/toast-store", () => ({
  toast: {
    success: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
    error: vi.fn(),
  },
}));

vi.mock("@/lib/player/deeplink", () => ({
  detectPlatform: vi.fn(() => "unknown"),
  getPlayersForPlatform: vi.fn(() => [
    { id: "vlc", name: "VLC", recommended: true, buildUrl: (s: string) => `vlc://${s}` },
  ]),
}));

vi.mock("@/components/domain/Modal", () => ({
  Modal: ({
    open,
    title,
    children,
  }: {
    open: boolean;
    title: string;
    children: React.ReactNode;
  }) =>
    open
      ? React.createElement("div", { "data-testid": "modal", "data-title": title }, children)
      : null,
}));

const testItem: BaseItem = {
  Id: "item-1",
  Name: "Test Movie",
  Type: "Movie",
  Path: "/media/test/movie.mkv",
  ProductionYear: 2024,
  RunTimeTicks: 7_200_000_000,
  ImagePrimaryUrl: "https://example.com/poster.jpg",
};

describe("PosterItemCard", () => {
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

  function renderCard(props: Partial<React.ComponentProps<typeof PosterItemCard>> = {}) {
    act(() => {
      root.render(<PosterItemCard item={testItem} href="/app/item/item-1" showGlow {...props} />);
    });

    const anchor = container.querySelector(
      "a[href='/app/item/item-1']"
    ) as HTMLAnchorElement | null;
    if (!anchor) {
      throw new Error("Expected anchor element");
    }

    const motionShell = anchor.firstElementChild as HTMLDivElement | null;
    if (!motionShell) {
      throw new Error("Expected motion shell element");
    }

    // Card is the aspect-[2/3] container — find it directly (glow is lazy-mounted on hover)
    const card = motionShell.querySelector("div.aspect-\\[2\\/3\\]") as HTMLDivElement | null;
    if (!card) {
      throw new Error("Expected card element");
    }

    return { anchor, motionShell, card };
  }

  /** Simulate hover to trigger lazy glow mount, then return glow elements */
  function hoverAndGetGlow(anchor: HTMLAnchorElement, motionShell: HTMLDivElement) {
    // React delegates onMouseEnter via mouseover (mouseenter doesn't bubble)
    act(() => {
      anchor.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
    });

    const glow = motionShell.querySelector("div[aria-hidden='true']") as HTMLDivElement | null;
    if (!glow) {
      throw new Error("Expected glow element after hover");
    }

    const glowImage = glow.firstElementChild as HTMLDivElement | null;
    if (!glowImage) {
      throw new Error("Expected glow image layer");
    }

    const glowHalo = glowImage.nextElementSibling as HTMLDivElement | null;
    if (!glowHalo) {
      throw new Error("Expected glow halo layer");
    }

    return { glow, glowImage, glowHalo };
  }

  it("positions glow in the same positioning context as the poster card", () => {
    const { anchor, motionShell } = renderCard();

    expect(anchor.className).toContain("block");
    expect(motionShell.className).toContain("group");
    expect(motionShell.className).toContain("py-4");

    // Glow is lazy-mounted — only appears on hover
    expect(motionShell.querySelector("div[aria-hidden='true']")).toBeNull();

    const { glow } = hoverAndGetGlow(anchor, motionShell);
    expect(glow.parentElement).toBe(motionShell);
    expect(glow.parentElement).not.toBe(anchor);
  });

  it("keeps glow static on hover and does not track pointer movement", () => {
    const { anchor, motionShell, card } = renderCard();

    // Trigger hover to mount glow
    const { glow, glowImage, glowHalo } = hoverAndGetGlow(anchor, motionShell);

    expect(glow.style.transform).toBe("");
    expect(card.style.transform).toBe("");
    expect(glowImage.className).toContain("inset-0");
    expect(glowImage.style.opacity).toBe("0.6");
    expect(glowHalo.style.boxShadow).toContain("#336699");

    act(() => {
      motionShell.dispatchEvent(
        new MouseEvent("mousemove", {
          bubbles: true,
          clientX: 85,
          clientY: 170,
        })
      );
    });

    expect(glow.style.transform).toBe("");
    expect(card.style.transform).toBe("");
    expect(glowImage.className).toContain("inset-0");
  });

  it("renders hover action buttons for play, favorite and add-to-list", () => {
    const { card } = renderCard();

    const playButton = card.querySelector("button[aria-label='播放']");
    const favoriteButton = card.querySelector("button[aria-label='喜欢']");
    const listButton = card.querySelector("button[aria-label='添加到列表']");

    expect(playButton).not.toBeNull();
    expect(favoriteButton).not.toBeNull();
    expect(listButton).not.toBeNull();
  });

  it("opens player picker modal when play button is clicked", async () => {
    const { card } = renderCard({ token: "test-token" });
    const playButton = card.querySelector("button[aria-label='播放']") as HTMLButtonElement;

    // No modal initially
    expect(document.querySelector("[data-title='选择播放器']")).toBeNull();

    await act(async () => {
      playButton.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await Promise.resolve();
    });

    // Modal should now be open
    const modal = document.querySelector("[data-title='选择播放器']");
    expect(modal).not.toBeNull();
  });
});
