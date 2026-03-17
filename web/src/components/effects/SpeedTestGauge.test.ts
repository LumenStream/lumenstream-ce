import { describe, expect, it } from "vitest";

import { buildJitterValue, createGaugeArcPath, valueToGaugeAngle } from "./SpeedTestGauge";

describe("SpeedTestGauge animation helpers", () => {
  it("maps speed values to gauge angles using gauge bounds", () => {
    expect(valueToGaugeAngle(0, 1000)).toBe(-135);
    expect(valueToGaugeAngle(500, 1000)).toBe(0);
    expect(valueToGaugeAngle(800, 1000)).toBe(81);
    expect(valueToGaugeAngle(1200, 1000)).toBe(135);
    expect(valueToGaugeAngle(-50, 1000)).toBe(-135);
  });

  it("builds jitter values around target while keeping bounds", () => {
    expect(buildJitterValue(800, 1000, 1)).toBe(824);
    expect(buildJitterValue(800, 1000, -1)).toBe(776);
    expect(buildJitterValue(1000, 1000, 1)).toBe(1000);
    expect(buildJitterValue(0, 1000, -1)).toBe(0);
  });

  it("builds gauge arc path with clockwise large arc to avoid reversed progress", () => {
    const path = createGaugeArcPath(160, 160, 100, -135, 135);
    expect(path).toContain("A 100 100 0 1 1");
  });
});
