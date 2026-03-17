import { describe, expect, it } from "vitest";

import {
  buildHeaderAccountQuickLinks,
  buildHeaderAvatarDisplay,
  resolveAvatarFallbackInitial,
  resolveUserAvatarImageUrl,
} from "@/lib/navigation/header-account-entry";
import type { User } from "@/lib/types/jellyfin";

function buildUser(overrides: Partial<User> = {}, extraFields: Record<string, unknown> = {}): User {
  return {
    Id: "u-1",
    Name: "Alice",
    HasPassword: true,
    ServerId: "s-1",
    Policy: {
      IsAdministrator: false,
      IsDisabled: false,
    },
    ...overrides,
    ...extraFields,
  } as User;
}

describe("header account entry model", () => {
  it("builds visitor fallback when user is missing", () => {
    expect(buildHeaderAvatarDisplay(null)).toEqual({
      displayName: "访客",
      fallbackInitial: "访",
      imageUrl: null,
    });
  });

  it("uses user name initial when avatar image is missing", () => {
    const user = buildUser({ Name: "  bob  " });

    expect(buildHeaderAvatarDisplay(user)).toEqual({
      displayName: "bob",
      fallbackInitial: "B",
      imageUrl: null,
    });
  });

  it("resolves avatar image from supported fields by priority", () => {
    const user = buildUser(
      {},
      {
        AvatarUrl: "https://example.com/avatar.png",
        ImagePrimaryUrl: "https://example.com/primary.png",
      }
    );

    expect(resolveUserAvatarImageUrl(user)).toBe("https://example.com/primary.png");
  });

  it("builds quick links with deduped extra link first", () => {
    expect(buildHeaderAccountQuickLinks({ href: "/admin/overview", label: "管理后台" })).toEqual([
      { href: "/admin/overview", label: "管理后台" },
      { href: "/app/profile", label: "账户中心" },
      { href: "/login", label: "切换账号" },
    ]);

    expect(buildHeaderAccountQuickLinks({ href: "/login", label: "切换账号" })).toEqual([
      { href: "/login", label: "切换账号" },
      { href: "/app/profile", label: "账户中心" },
    ]);
  });

  it("falls back to visitor initial for blank names", () => {
    expect(resolveAvatarFallbackInitial("   ")).toBe("访");
  });
});
