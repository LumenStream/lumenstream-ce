/**
 * @vitest-environment jsdom
 */
import { describe, it, expect, afterEach } from "vitest";
import { getRouteParam } from "./use-route-param";

describe("getRouteParam", () => {
  const originalLocation = window.location;

  function setPathname(pathname: string) {
    Object.defineProperty(window, "location", {
      value: { ...originalLocation, pathname },
      writable: true,
      configurable: true,
    });
  }

  afterEach(() => {
    Object.defineProperty(window, "location", {
      value: originalLocation,
      writable: true,
      configurable: true,
    });
  });

  it("extracts segment after key", () => {
    setPathname("/app/item/abc-123");
    expect(getRouteParam("item")).toBe("abc-123");
  });

  it("extracts segment for library key", () => {
    setPathname("/app/library/some-parent-id");
    expect(getRouteParam("library")).toBe("some-parent-id");
  });

  it("extracts segment for person key", () => {
    setPathname("/app/person/person-uuid");
    expect(getRouteParam("person")).toBe("person-uuid");
  });

  it("returns empty string when key not found", () => {
    setPathname("/app/other/value");
    expect(getRouteParam("item")).toBe("");
  });

  it("returns empty string when key is last segment", () => {
    setPathname("/app/item");
    expect(getRouteParam("item")).toBe("");
  });

  it("decodes URI-encoded segments", () => {
    setPathname("/app/item/hello%20world");
    expect(getRouteParam("item")).toBe("hello world");
  });

  it("handles trailing slash", () => {
    setPathname("/app/item/abc-123/");
    expect(getRouteParam("item")).toBe("abc-123");
  });
});
