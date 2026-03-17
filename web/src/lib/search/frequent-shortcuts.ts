export type SearchIncludeItemType = "" | "Movie" | "Series" | "Episode";

export interface FrequentSearchShortcutLinkAction {
  type: "link";
  href: "/app/home" | "/app/profile" | "/admin/overview";
}

export interface FrequentSearchShortcutAction {
  type: "search";
  searchTerm: string;
  includeItemTypes?: SearchIncludeItemType;
}

interface FrequentSearchShortcutBase {
  id: "home" | "profile" | "admin" | "movie-search" | "series-search";
  label: string;
  description: string;
}

export interface FrequentSearchShortcutLink extends FrequentSearchShortcutBase {
  action: FrequentSearchShortcutLinkAction;
}

export interface FrequentSearchShortcutSearch extends FrequentSearchShortcutBase {
  action: FrequentSearchShortcutAction;
}

export type FrequentSearchShortcut = FrequentSearchShortcutLink | FrequentSearchShortcutSearch;

export const frequentSearchShortcuts = [
  {
    id: "home",
    label: "返回首页",
    description: "快速跳转到首页继续浏览。",
    action: {
      type: "link",
      href: "/app/home",
    },
  },
  {
    id: "profile",
    label: "账户中心",
    description: "快速进入账户页查看会话与订阅信息。",
    action: {
      type: "link",
      href: "/app/profile",
    },
  },
  {
    id: "admin",
    label: "进入管理端",
    description: "快速进入管理端总览。",
    action: {
      type: "link",
      href: "/admin/overview",
    },
  },
  {
    id: "movie-search",
    label: "找电影",
    description: "回填“电影”并按电影类型发起搜索。",
    action: {
      type: "search",
      searchTerm: "电影",
      includeItemTypes: "Movie",
    },
  },
  {
    id: "series-search",
    label: "找剧集",
    description: "回填“剧集”并按剧集类型发起搜索。",
    action: {
      type: "search",
      searchTerm: "剧集",
      includeItemTypes: "Series",
    },
  },
] as const satisfies readonly FrequentSearchShortcut[];

export function getFrequentSearchShortcuts(): readonly FrequentSearchShortcut[] {
  return frequentSearchShortcuts;
}
