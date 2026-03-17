import { useEffect, useState } from "react";

import {
  DEFAULT_FALLBACK_COLOR,
  extractDominantColorFromUrl,
  rgbToCss,
  type RGB,
} from "@/lib/image/color-extractor";

/** Cache for extracted colors to avoid re-computation */
const colorCache = new Map<string, RGB>();

export interface UseImageGlowOptions {
  /** Fallback color when extraction fails */
  fallback?: RGB;
  /** Whether to skip extraction (e.g., for disabled state) */
  enabled?: boolean;
}

export interface UseImageGlowResult {
  /** CSS color string for the glow, or null while loading */
  glowColor: string | null;
  /** Whether extraction is in progress */
  isLoading: boolean;
  /** The extracted RGB values, or null if not yet extracted */
  rgb: RGB | null;
}

/**
 * React hook that extracts the dominant color from an image for glow effects.
 * Caches results by URL to avoid re-computation.
 *
 * @param src - Image URL to extract color from
 * @param options - Configuration options
 * @returns Glow color state and loading status
 */
export function useImageGlow(
  src: string | null | undefined,
  options: UseImageGlowOptions = {}
): UseImageGlowResult {
  const { fallback = DEFAULT_FALLBACK_COLOR, enabled = true } = options;

  const [rgb, setRgb] = useState<RGB | null>(() => {
    if (!src || !enabled) return null;
    return colorCache.get(src) ?? null;
  });
  const [isLoading, setIsLoading] = useState(() => {
    if (!src || !enabled) return false;
    return !colorCache.has(src);
  });

  useEffect(() => {
    if (!src || !enabled) {
      setRgb(null);
      setIsLoading(false);
      return;
    }

    // Check cache first
    const cached = colorCache.get(src);
    if (cached) {
      setRgb(cached);
      setIsLoading(false);
      return;
    }

    let cancelled = false;
    setIsLoading(true);

    extractDominantColorFromUrl(src, fallback).then((color) => {
      if (cancelled) return;

      colorCache.set(src, color);
      setRgb(color);
      setIsLoading(false);
    });

    return () => {
      cancelled = true;
    };
  }, [src, enabled, fallback]);

  return {
    glowColor: rgb ? rgbToCss(rgb) : null,
    isLoading,
    rgb,
  };
}

/**
 * Clears the color cache. Useful for testing or memory management.
 */
export function clearImageGlowCache(): void {
  colorCache.clear();
}
