import { describe, expect, it } from "vitest";

import {
  getAdminNavItems,
  getDefaultProfileSection,
  getProfileSections,
} from "@/lib/edition/capabilities";

describe("edition capability helpers", () => {
  it("hides commercial admin entries by default", () => {
    const labels = getAdminNavItems().map((item) => item.label);
    expect(labels).not.toContain("账单");
    expect(labels).not.toContain("流量");
    expect(labels).toContain("推流与域名");
    expect(labels).toContain("求片 Agent");
  });

  it("shows commercial admin entries when capabilities are enabled", () => {
    const labels = getAdminNavItems({
      billing_enabled: true,
      advanced_traffic_controls_enabled: true,
    }).map((item) => item.label);
    expect(labels).toContain("账单");
    expect(labels).toContain("流量");
  });

  it("defaults profile navigation to playback in CE", () => {
    expect(getProfileSections()).toEqual(["playback", "social"]);
    expect(getDefaultProfileSection()).toBe("playback");
  });

  it("adds billing and traffic sections when commercial features are enabled", () => {
    expect(
      getProfileSections({
        billing_enabled: true,
        advanced_traffic_controls_enabled: true,
      })
    ).toEqual(["billing", "playback", "social", "traffic"]);
  });
});
