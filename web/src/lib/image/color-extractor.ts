/**
 * Color extraction utility using Canvas API.
 * Extracts dominant color from images for UI theming purposes.
 */

export interface RGB {
  r: number;
  g: number;
  b: number;
}

/** Default fallback color (neutral gray) when extraction fails */
export const DEFAULT_FALLBACK_COLOR: RGB = { r: 128, g: 128, b: 128 };

/**
 * Extracts the dominant color from an HTMLImageElement using Canvas sampling.
 * Samples a grid of pixels and averages them to find the dominant color.
 *
 * @param image - The image element to extract color from (must be loaded)
 * @param fallback - Color to return if extraction fails (default: gray)
 * @returns RGB color values
 */
export function extractDominantColorFromElement(
  image: HTMLImageElement,
  fallback: RGB = DEFAULT_FALLBACK_COLOR
): RGB {
  if (!image.complete || image.naturalWidth === 0) {
    return fallback;
  }

  try {
    const canvas = document.createElement("canvas");
    const ctx = canvas.getContext("2d", { willReadFrequently: true });

    if (!ctx) {
      return fallback;
    }

    // Use a small sample size for performance
    const sampleSize = 50;
    canvas.width = sampleSize;
    canvas.height = sampleSize;

    // Draw scaled image to canvas
    ctx.drawImage(image, 0, 0, sampleSize, sampleSize);

    // Get pixel data
    const imageData = ctx.getImageData(0, 0, sampleSize, sampleSize);
    const data = imageData.data;

    let totalR = 0;
    let totalG = 0;
    let totalB = 0;
    let count = 0;

    // Sample every 4th pixel for performance
    for (let i = 0; i < data.length; i += 16) {
      const r = data[i];
      const g = data[i + 1];
      const b = data[i + 2];
      const a = data[i + 3];

      // Skip fully transparent pixels
      if (a < 128) continue;

      totalR += r;
      totalG += g;
      totalB += b;
      count++;
    }

    if (count === 0) {
      return fallback;
    }

    return {
      r: Math.round(totalR / count),
      g: Math.round(totalG / count),
      b: Math.round(totalB / count),
    };
  } catch {
    // CORS or other canvas security errors
    return fallback;
  }
}

/**
 * Loads an image from URL and extracts its dominant color.
 * Handles CORS by using crossOrigin="anonymous".
 *
 * @param url - The image URL to load
 * @param fallback - Color to return if loading or extraction fails
 * @returns Promise resolving to RGB color values
 */
export async function extractDominantColorFromUrl(
  url: string,
  fallback: RGB = DEFAULT_FALLBACK_COLOR
): Promise<RGB> {
  return new Promise((resolve) => {
    const image = new Image();
    image.crossOrigin = "anonymous";

    image.onload = () => {
      resolve(extractDominantColorFromElement(image, fallback));
    };

    image.onerror = () => {
      resolve(fallback);
    };

    image.src = url;
  });
}

/**
 * Converts RGB values to a CSS color string.
 */
export function rgbToCss(color: RGB): string {
  return `rgb(${color.r}, ${color.g}, ${color.b})`;
}

/**
 * Converts RGB values to a hex color string.
 */
export function rgbToHex(color: RGB): string {
  const toHex = (n: number) => n.toString(16).padStart(2, "0");
  return `#${toHex(color.r)}${toHex(color.g)}${toHex(color.b)}`;
}
