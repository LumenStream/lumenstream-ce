import { describe, expect, it } from "vitest";

import {
  getProfileSidebarNavigationItems,
  profileSidebarNavigationItems,
} from "@/lib/profile/sidebar-navigation";

describe("profile sidebar navigation model", () => {
  it("includes core sidebar links with expected targets", () => {
    const linksByLabel = new Map(
      profileSidebarNavigationItems.map((item) => [item.label, item.href])
    );

    expect(linksByLabel.get("进入管理端")).toBe("/admin/overview");
    expect(linksByLabel.get("返回首页")).toBe("/");
  });

  it("returns the centralized navigation collection", () => {
    expect(getProfileSidebarNavigationItems()).toBe(profileSidebarNavigationItems);
  });
});
