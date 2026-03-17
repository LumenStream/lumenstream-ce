import type { User } from "@/lib/types/jellyfin";

export interface HeaderAccountQuickLink {
  href: string;
  label: string;
}

export interface HeaderAvatarDisplay {
  displayName: string;
  fallbackInitial: string;
  imageUrl: string | null;
}

const DEFAULT_DISPLAY_NAME = "访客";
const DEFAULT_FALLBACK_INITIAL = "访";

const DEFAULT_ACCOUNT_QUICK_LINKS = [
  { href: "/app/profile", label: "账户中心" },
  { href: "/login", label: "切换账号" },
] as const satisfies readonly HeaderAccountQuickLink[];

const avatarImageFieldCandidates = [
  "ImagePrimaryUrl",
  "AvatarUrl",
  "ProfileImageUrl",
  "ImageUrl",
] as const;

function readOptionalStringField(value: unknown): string | null {
  if (typeof value !== "string") {
    return null;
  }

  const normalized = value.trim();
  return normalized.length > 0 ? normalized : null;
}

export function resolveAvatarFallbackInitial(name: string | null | undefined): string {
  const normalizedName = name?.trim();
  if (!normalizedName) {
    return DEFAULT_FALLBACK_INITIAL;
  }

  const firstChar = Array.from(normalizedName)[0];
  return firstChar ? firstChar.toUpperCase() : DEFAULT_FALLBACK_INITIAL;
}

export function resolveUserAvatarImageUrl(user: User | null): string | null {
  if (!user) {
    return null;
  }

  const userRecord = user as unknown as Record<
    (typeof avatarImageFieldCandidates)[number],
    unknown
  >;
  for (const field of avatarImageFieldCandidates) {
    const candidate = readOptionalStringField(userRecord[field]);
    if (candidate) {
      return candidate;
    }
  }

  return null;
}

export function buildHeaderAvatarDisplay(user: User | null): HeaderAvatarDisplay {
  const normalizedName = user?.Name?.trim();
  const displayName =
    normalizedName && normalizedName.length > 0 ? normalizedName : DEFAULT_DISPLAY_NAME;

  return {
    displayName,
    fallbackInitial: resolveAvatarFallbackInitial(displayName),
    imageUrl: resolveUserAvatarImageUrl(user),
  };
}

export function buildHeaderAccountQuickLinks(
  extraLink?: HeaderAccountQuickLink | null
): HeaderAccountQuickLink[] {
  const quickLinks: HeaderAccountQuickLink[] = [];
  const seenHrefs = new Set<string>();

  const append = (link: HeaderAccountQuickLink | null | undefined) => {
    if (!link || seenHrefs.has(link.href)) {
      return;
    }

    seenHrefs.add(link.href);
    quickLinks.push(link);
  };

  append(extraLink);
  for (const link of DEFAULT_ACCOUNT_QUICK_LINKS) {
    append(link);
  }

  return quickLinks;
}
