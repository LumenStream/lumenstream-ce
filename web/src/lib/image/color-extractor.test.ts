/**
 * @vitest-environment jsdom
 */
import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";

import {
  DEFAULT_FALLBACK_COLOR,
  extractDominantColorFromElement,
  extractDominantColorFromUrl,
  rgbToCss,
  rgbToHex,
} from "@/lib/image/color-extractor";

describe("extractDominantColorFromElement", () => {
  let mockCanvas: HTMLCanvasElement;
  let mockCtx: CanvasRenderingContext2D;

  beforeEach(() => {
    // Mock canvas context
    mockCtx = {
      drawImage: vi.fn(),
      getImageData: vi.fn(),
    } as unknown as CanvasRenderingContext2D;

    mockCanvas = {
      width: 0,
      height: 0,
      getContext: vi.fn().mockReturnValue(mockCtx),
    } as unknown as HTMLCanvasElement;

    vi.spyOn(document, "createElement").mockReturnValue(mockCanvas);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("returns fallback for incomplete image", () => {
    const image = { complete: false, naturalWidth: 0 } as HTMLImageElement;
    expect(extractDominantColorFromElement(image)).toEqual(DEFAULT_FALLBACK_COLOR);
  });

  it("returns fallback for zero-width image", () => {
    const image = { complete: true, naturalWidth: 0 } as HTMLImageElement;
    expect(extractDominantColorFromElement(image)).toEqual(DEFAULT_FALLBACK_COLOR);
  });

  it("returns custom fallback when provided", () => {
    const image = { complete: false, naturalWidth: 0 } as HTMLImageElement;
    const customFallback = { r: 255, g: 0, b: 0 };
    expect(extractDominantColorFromElement(image, customFallback)).toEqual(customFallback);
  });

  it("extracts average color from image data", () => {
    const image = { complete: true, naturalWidth: 100 } as HTMLImageElement;

    // Mock pixel data: all red pixels (RGBA)
    const pixelData = new Uint8ClampedArray(50 * 50 * 4);
    for (let i = 0; i < pixelData.length; i += 4) {
      pixelData[i] = 255; // R
      pixelData[i + 1] = 0; // G
      pixelData[i + 2] = 0; // B
      pixelData[i + 3] = 255; // A
    }

    (mockCtx.getImageData as ReturnType<typeof vi.fn>).mockReturnValue({
      data: pixelData,
    });

    const result = extractDominantColorFromElement(image);
    expect(result).toEqual({ r: 255, g: 0, b: 0 });
  });

  it("skips transparent pixels", () => {
    const image = { complete: true, naturalWidth: 100 } as HTMLImageElement;

    // Create pixel data with mix of opaque red and transparent pixels
    const pixelData = new Uint8ClampedArray(64); // 4 pixels worth, sampled every 16 bytes = 1 pixel
    // First pixel: red, opaque
    pixelData[0] = 200;
    pixelData[1] = 100;
    pixelData[2] = 50;
    pixelData[3] = 255;
    // Pixel at offset 16: transparent (should be skipped)
    pixelData[16] = 0;
    pixelData[17] = 255;
    pixelData[18] = 0;
    pixelData[19] = 0; // transparent

    (mockCtx.getImageData as ReturnType<typeof vi.fn>).mockReturnValue({
      data: pixelData,
    });

    const result = extractDominantColorFromElement(image);
    // Should only count the first pixel
    expect(result).toEqual({ r: 200, g: 100, b: 50 });
  });

  it("returns fallback when canvas context unavailable", () => {
    const image = { complete: true, naturalWidth: 100 } as HTMLImageElement;
    (mockCanvas.getContext as ReturnType<typeof vi.fn>).mockReturnValue(null);

    expect(extractDominantColorFromElement(image)).toEqual(DEFAULT_FALLBACK_COLOR);
  });

  it("returns fallback when all pixels are transparent", () => {
    const image = { complete: true, naturalWidth: 100 } as HTMLImageElement;

    // All transparent pixels
    const pixelData = new Uint8ClampedArray(64);
    for (let i = 0; i < pixelData.length; i += 4) {
      pixelData[i + 3] = 0; // alpha = 0
    }

    (mockCtx.getImageData as ReturnType<typeof vi.fn>).mockReturnValue({
      data: pixelData,
    });

    expect(extractDominantColorFromElement(image)).toEqual(DEFAULT_FALLBACK_COLOR);
  });

  it("handles canvas security errors gracefully", () => {
    const image = { complete: true, naturalWidth: 100 } as HTMLImageElement;
    (mockCtx.getImageData as ReturnType<typeof vi.fn>).mockImplementation(() => {
      throw new DOMException("Security error");
    });

    expect(extractDominantColorFromElement(image)).toEqual(DEFAULT_FALLBACK_COLOR);
  });
});

describe("extractDominantColorFromUrl", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("returns fallback on load error", async () => {
    let errorHandler: (() => void) | undefined;

    vi.spyOn(globalThis, "Image").mockImplementation(function () {
      const img = {
        crossOrigin: "",
        src: "",
        onload: null as (() => void) | null,
        onerror: null as (() => void) | null,
      };

      // Capture the error handler when it is set
      Object.defineProperty(img, "onerror", {
        set(handler: () => void) {
          errorHandler = handler;
        },
      });

      Object.defineProperty(img, "src", {
        set() {
          // Trigger error on next tick
          setTimeout(() => errorHandler?.(), 0);
        },
      });

      return img as unknown as HTMLImageElement;
    } as unknown as typeof Image);

    const promise = extractDominantColorFromUrl("https://example.com/broken.jpg");
    await vi.runAllTimersAsync();

    const result = await promise;
    expect(result).toEqual(DEFAULT_FALLBACK_COLOR);
  });

  it("sets crossOrigin to anonymous", async () => {
    let capturedCrossOrigin = "";

    vi.spyOn(globalThis, "Image").mockImplementation(function () {
      const img = {
        _crossOrigin: "",
        src: "",
        onload: null as (() => void) | null,
        onerror: null as (() => void) | null,
      };

      Object.defineProperty(img, "crossOrigin", {
        get() {
          return img._crossOrigin;
        },
        set(value: string) {
          img._crossOrigin = value;
          capturedCrossOrigin = value;
        },
      });

      return img as unknown as HTMLImageElement;
    } as unknown as typeof Image);

    extractDominantColorFromUrl("https://example.com/image.jpg");

    expect(capturedCrossOrigin).toBe("anonymous");
  });
});

describe("rgbToCss", () => {
  it("converts RGB to CSS string", () => {
    expect(rgbToCss({ r: 255, g: 128, b: 0 })).toBe("rgb(255, 128, 0)");
  });

  it("handles zero values", () => {
    expect(rgbToCss({ r: 0, g: 0, b: 0 })).toBe("rgb(0, 0, 0)");
  });
});

describe("rgbToHex", () => {
  it("converts RGB to hex string", () => {
    expect(rgbToHex({ r: 255, g: 128, b: 0 })).toBe("#ff8000");
  });

  it("pads single digit hex values", () => {
    expect(rgbToHex({ r: 0, g: 15, b: 1 })).toBe("#000f01");
  });

  it("handles black", () => {
    expect(rgbToHex({ r: 0, g: 0, b: 0 })).toBe("#000000");
  });

  it("handles white", () => {
    expect(rgbToHex({ r: 255, g: 255, b: 255 })).toBe("#ffffff");
  });
});
