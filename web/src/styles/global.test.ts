import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

describe("global theme tokens", () => {
  it("keeps dark-mode muted foreground at readable contrast", () => {
    const css = readFileSync(new URL("./global.css", import.meta.url), "utf8");

    expect(css).toContain("--muted-foreground: 0 0% 66%;");
  });
});
