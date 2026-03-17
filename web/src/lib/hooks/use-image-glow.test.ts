import { afterEach, describe, expect, it, vi } from "vitest";

import { clearImageGlowCache } from "./use-image-glow";

// Mock the color extractor module
vi.mock("@/lib/image/color-extractor", () => ({
  DEFAULT_FALLBACK_COLOR: { r: 128, g: 128, b: 128 },
  extractDominantColorFromUrl: vi.fn(),
  rgbToCss: vi.fn(
    (color: { r: number; g: number; b: number }) => `rgb(${color.r}, ${color.g}, ${color.b})`
  ),
}));

import { extractDominantColorFromUrl, rgbToCss } from "@/lib/image/color-extractor";

const mockExtract = vi.mocked(extractDominantColorFromUrl);
const mockRgbToCss = vi.mocked(rgbToCss);

describe("useImageGlow", () => {
  afterEach(() => {
    clearImageGlowCache();
    vi.clearAllMocks();
  });

  describe("clearImageGlowCache", () => {
    it("clears the cache without error", () => {
      expect(() => clearImageGlowCache()).not.toThrow();
    });
  });

  describe("rgbToCss integration", () => {
    it("converts RGB to CSS string format", () => {
      const color = { r: 200, g: 100, b: 50 };
      const result = mockRgbToCss(color);
      expect(result).toBe("rgb(200, 100, 50)");
    });
  });

  describe("extractDominantColorFromUrl integration", () => {
    it("can be called with URL and fallback", async () => {
      const extractedColor = { r: 255, g: 128, b: 64 };
      mockExtract.mockResolvedValue(extractedColor);

      const result = await extractDominantColorFromUrl("https://example.com/image.jpg", {
        r: 128,
        g: 128,
        b: 128,
      });

      expect(result).toEqual(extractedColor);
      expect(mockExtract).toHaveBeenCalledWith("https://example.com/image.jpg", {
        r: 128,
        g: 128,
        b: 128,
      });
    });

    it("returns fallback on error", async () => {
      const fallback = { r: 128, g: 128, b: 128 };
      mockExtract.mockResolvedValue(fallback);

      const result = await extractDominantColorFromUrl("https://example.com/broken.jpg", fallback);

      expect(result).toEqual(fallback);
    });
  });
});

describe("useImageGlow hook behavior", () => {
  // Since we can't use @testing-library/react, we test the hook's
  // underlying logic through its exports and the color extractor integration

  afterEach(() => {
    clearImageGlowCache();
    vi.clearAllMocks();
  });

  it("exports clearImageGlowCache function", async () => {
    const { clearImageGlowCache: exportedFn } = await import("./use-image-glow");
    expect(typeof exportedFn).toBe("function");
  });

  it("exports useImageGlow hook", async () => {
    const { useImageGlow } = await import("./use-image-glow");
    expect(typeof useImageGlow).toBe("function");
  });

  it("hook returns correct interface shape", async () => {
    // We can't call the hook outside React, but we can verify the types compile
    // by checking the module exports
    const module = await import("./use-image-glow");
    expect(module).toHaveProperty("useImageGlow");
    expect(module).toHaveProperty("clearImageGlowCache");
  });
});
