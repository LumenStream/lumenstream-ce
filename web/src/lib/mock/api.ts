import type {
  AdminApiKey,
  AdminCreatedApiKey,
  AdminTaskDefinition,
  AdminTaskRun,
  AdminUserManageProfile,
  AdminUserProfileRecord,
  AdminUserSessionsSummary,
  AdminUserSummaryItem,
  AdminUserSummaryPage,
  AdminLibrary,
  AdminLibraryStatusItem,
  AdminLibraryStatusResponse,
  AdminSystemCapabilities,
  AdminSystemFlags,
  AdminSystemSummary,
  AdminInviteSettings,
  AdminUpsertSettingsResponse,
  AdminUser,
  AuditLogEntry,
  AuthSession,
  PlaybackSession,
  PlaybackDomain,
  StreamPolicy,
  LumenBackendNode,
  LumenBackendNodeRuntimeSchema,
  LumenBackendNodeRuntimeConfig,
  LumenBackendRuntimeSchemaDefinition,
  LibraryType,
  TmdbCacheStats,
  TmdbFailureEntry,
  ScraperCacheStats,
  ScraperFailureEntry,
  ScraperProviderStatus,
  ScraperSettingsResponse,
  TopTrafficUser,
  TrafficUsage,
  MyTrafficUsageMediaSummary,
  TrafficUsageMediaItem,
  UserRole,
  WebAppSettings,
  MePlaybackDomainsResponse,
  InviteSummary,
  InviteRelation,
  InviteRebateRecord,
} from "@/lib/types/admin";
import type {
  AuthResult,
  BaseItem,
  ItemCounts,
  PlaybackInfo,
  QueryResult,
  Season,
  SubtitleTrack,
  TopPlayedItem,
  TopPlayedSummary,
  UserItemData,
  User,
} from "@/lib/types/jellyfin";
import type {
  CreatePlaylistPayload,
  Playlist,
  PlaylistItem,
  PlaylistItemsResponse,
  UpdatePlaylistPayload,
} from "@/lib/types/playlist";
import type {
  AgentCreateRequest,
  AgentProviderStatus,
  AgentRequest,
  AgentRequestDetail,
  AgentRequestEvent,
  AgentRequestsQuery,
  AgentReviewRequest,
  AgentSettings,
  AgentWorkflowStepState,
} from "@/lib/types/requests";

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

let idSeed = 1000;

function nextId(prefix: string): string {
  idSeed += 1;
  return `${prefix}-${idSeed}`;
}

function minutesAgo(minutes: number): string {
  return new Date(Date.now() - minutes * 60_000).toISOString();
}

function toQueryResult<T>(items: T[], startIndex = 0, limit = 100): QueryResult<T> {
  const normalizedStart = Math.max(0, startIndex);
  const normalizedLimit = Math.max(1, limit);
  const sliced = items.slice(normalizedStart, normalizedStart + normalizedLimit);

  return {
    Items: clone(sliced),
    TotalRecordCount: items.length,
    StartIndex: normalizedStart,
  };
}

function parseCsv(input?: string): string[] {
  if (!input) {
    return [];
  }

  return input
    .split(",")
    .map((part) => part.trim())
    .filter((part) => part.length > 0);
}

const DEMO_SERVER_ID = "lumenstream-mock-server";
const DEMO_TOKEN = "mock-token-demo-admin";

function posterDataUrl(title: string, from: string, to: string): string {
  const safeTitle = title.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="360" height="540" viewBox="0 0 360 540"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0%" stop-color="${from}" /><stop offset="100%" stop-color="${to}" /></linearGradient></defs><rect width="360" height="540" fill="url(#g)"/><rect x="0" y="360" width="360" height="180" fill="rgba(0,0,0,0.48)"/><text x="24" y="430" fill="#ffffff" font-family="Inter,Arial,sans-serif" font-size="28" font-weight="700">${safeTitle}</text></svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

function movieItem(params: {
  id: string;
  name: string;
  year: number;
  rating: number;
  runtimeMinutes: number;
  path: string;
  colors: [string, string];
  resumeMinutes?: number;
  overview?: string;
}): BaseItem {
  return {
    Id: params.id,
    Name: params.name,
    Type: "Movie",
    Path: params.path,
    ProductionYear: params.year,
    CommunityRating: params.rating,
    Overview: params.overview || `${params.name}（Mock） - 仅用于演示海报卡与元信息布局。`,
    ImagePrimaryUrl: posterDataUrl(params.name, params.colors[0], params.colors[1]),
    ImageTags: {
      Primary: `primary-${params.id}`,
    },
    BackdropImageTags: [`backdrop-${params.id}`],
    OfficialRating: "PG-13",
    Genres: ["Drama", "Crime"],
    Studios: [
      {
        Name: "LumenStream Mock Studio",
      },
    ],
    People: [
      {
        Name: "Mock Actor",
        Id: `person-${params.id}-actor-1`,
        Role: "Lead",
        Type: "Actor",
        PrimaryImageTag: `tag-${params.id}-actor-1`,
      },
      {
        Name: "Mock Director",
        Id: `person-${params.id}-director-1`,
        Role: "Director",
        Type: "Director",
        PrimaryImageTag: `tag-${params.id}-director-1`,
      },
    ],
    PlayAccess: "Full",
    RunTimeTicks: params.runtimeMinutes * 60 * 10_000_000,
    Bitrate: 8_000_000 + Math.round(params.rating * 300_000),
    UserData: {
      Played: !params.resumeMinutes,
      PlaybackPositionTicks: (params.resumeMinutes || 0) * 60 * 10_000_000,
      IsFavorite: false,
    },
  };
}

const ROOT_ITEMS: BaseItem[] = [
  {
    Id: "root-high-score",
    Name: "高分佳作",
    Type: "CollectionFolder",
    Path: "/media/movies/high-score",
  },
  {
    Id: "root-cn-movies",
    Name: "华语电影",
    Type: "CollectionFolder",
    Path: "/media/movies/chinese",
  },
  {
    Id: "root-series",
    Name: "剧集推荐",
    Type: "CollectionFolder",
    Path: "/media/shows",
  },
];

const HIGH_SCORE_ITEMS: BaseItem[] = [
  movieItem({
    id: "movie-001",
    name: "肖申克的救赎",
    year: 1994,
    rating: 8.7,
    runtimeMinutes: 142,
    path: "/media/movies/shawshank.strm",
    colors: ["#1f2937", "#0f766e"],
  }),
  movieItem({
    id: "movie-002",
    name: "教父",
    year: 1972,
    rating: 8.7,
    runtimeMinutes: 176,
    path: "/media/movies/godfather.strm",
    colors: ["#7f1d1d", "#0f172a"],
  }),
  movieItem({
    id: "movie-003",
    name: "教父2",
    year: 1974,
    rating: 8.6,
    runtimeMinutes: 200,
    path: "/media/movies/godfather2.strm",
    colors: ["#78350f", "#312e81"],
    resumeMinutes: 36,
  }),
  movieItem({
    id: "movie-004",
    name: "辛德勒的名单",
    year: 1993,
    rating: 8.6,
    runtimeMinutes: 195,
    path: "/media/movies/schindler.strm",
    colors: ["#111827", "#475569"],
  }),
  movieItem({
    id: "movie-005",
    name: "十二怒汉",
    year: 1957,
    rating: 8.5,
    runtimeMinutes: 95,
    path: "/media/movies/12-angry-men.strm",
    colors: ["#7c2d12", "#dc2626"],
  }),
  movieItem({
    id: "movie-006",
    name: "千与千寻",
    year: 2001,
    rating: 8.5,
    runtimeMinutes: 124,
    path: "/media/movies/spirited-away.strm",
    colors: ["#0ea5e9", "#2563eb"],
  }),
];

const CN_MOVIE_ITEMS: BaseItem[] = [
  movieItem({
    id: "movie-101",
    name: "火拼",
    year: 2024,
    rating: 7.3,
    runtimeMinutes: 118,
    path: "/media/movies/cn-fire.strm",
    colors: ["#7f1d1d", "#b91c1c"],
  }),
  movieItem({
    id: "movie-102",
    name: "大蛇",
    year: 2022,
    rating: 7.0,
    runtimeMinutes: 110,
    path: "/media/movies/cn-snake.strm",
    colors: ["#1e3a8a", "#0f766e"],
  }),
  movieItem({
    id: "movie-103",
    name: "大决杀",
    year: 2023,
    rating: 7.5,
    runtimeMinutes: 126,
    path: "/media/movies/cn-final.strm",
    colors: ["#991b1b", "#1f2937"],
  }),
  movieItem({
    id: "movie-104",
    name: "刺杀小说家",
    year: 2021,
    rating: 7.1,
    runtimeMinutes: 130,
    path: "/media/movies/cn-assassin.strm",
    colors: ["#7c2d12", "#92400e"],
    resumeMinutes: 20,
  }),
  movieItem({
    id: "movie-105",
    name: "猎战狂徒",
    year: 2024,
    rating: 7.2,
    runtimeMinutes: 122,
    path: "/media/movies/cn-hunter.strm",
    colors: ["#172554", "#7f1d1d"],
  }),
  movieItem({
    id: "movie-106",
    name: "山河故人",
    year: 2015,
    rating: 7.4,
    runtimeMinutes: 131,
    path: "/media/movies/cn-mountain-river.strm",
    colors: ["#1f2937", "#334155"],
  }),
];

const SERIES_ITEMS: BaseItem[] = [
  {
    Id: "series-001",
    Name: "The Last of Us",
    Type: "Series",
    Path: "/media/shows/the-last-of-us",
    ProductionYear: 2023,
    CommunityRating: 8.4,
    Overview: "在末世中穿行的生存之旅（Mock 数据）。",
    ImagePrimaryUrl: posterDataUrl("The Last of Us", "#1f2937", "#0f766e"),
  },
  {
    Id: "series-002",
    Name: "三体",
    Type: "Series",
    Path: "/media/shows/three-body",
    ProductionYear: 2024,
    CommunityRating: 7.9,
    Overview: "科幻悬疑剧集，展示卡片布局与详情页信息（Mock）。",
    ImagePrimaryUrl: posterDataUrl("三体", "#172554", "#7c3aed"),
  },
  {
    Id: "series-003",
    Name: "杀死伊芙",
    Type: "Series",
    Path: "/media/shows/killing-eve",
    ProductionYear: 2020,
    CommunityRating: 8.1,
    Overview: "高张力谍战故事（Mock）。",
    ImagePrimaryUrl: posterDataUrl("Killing Eve", "#7f1d1d", "#be123c"),
  },
];

const EPISODE_ITEMS: BaseItem[] = [
  {
    Id: "episode-001",
    Name: "When You're Lost in the Darkness",
    Type: "Episode",
    Path: "/media/shows/the-last-of-us/s01e01.strm",
    ProductionYear: 2023,
    CommunityRating: 8.4,
    Overview: "第一集，含续播进度。",
    ImagePrimaryUrl: posterDataUrl("TLOU S01E01", "#1f2937", "#0f766e"),
    RunTimeTicks: 3_600_000_000,
    Bitrate: 7_000_000,
    SeriesId: "series-001",
    SeasonId: "season-001-01",
    IndexNumber: 1,
    ParentIndexNumber: 1,
    UserData: {
      Played: false,
      PlaybackPositionTicks: 1_200_000_000,
    },
  },
  {
    Id: "episode-002",
    Name: "Infected",
    Type: "Episode",
    Path: "/media/shows/the-last-of-us/s01e02.strm",
    ProductionYear: 2023,
    CommunityRating: 8.5,
    Overview: "第二集。",
    ImagePrimaryUrl: posterDataUrl("TLOU S01E02", "#1f2937", "#0f766e"),
    RunTimeTicks: 3_200_000_000,
    Bitrate: 7_000_000,
    SeriesId: "series-001",
    SeasonId: "season-001-01",
    IndexNumber: 2,
    ParentIndexNumber: 1,
    UserData: {
      Played: false,
      PlaybackPositionTicks: 0,
    },
  },
  {
    Id: "episode-003",
    Name: "三体 第1集",
    Type: "Episode",
    Path: "/media/shows/three-body/s01e01.strm",
    ProductionYear: 2024,
    CommunityRating: 7.8,
    Overview: "第一集，已看完示例。",
    ImagePrimaryUrl: posterDataUrl("Three Body S01E01", "#172554", "#7c3aed"),
    RunTimeTicks: 2_800_000_000,
    Bitrate: 5_000_000,
    SeriesId: "series-002",
    SeasonId: "season-002-01",
    IndexNumber: 1,
    ParentIndexNumber: 1,
    UserData: {
      Played: true,
      PlaybackPositionTicks: 0,
    },
  },
  {
    Id: "episode-004",
    Name: "三体 第2集",
    Type: "Episode",
    Path: "/media/shows/three-body/s01e02.strm",
    ProductionYear: 2024,
    CommunityRating: 7.9,
    Overview: "第二集。",
    ImagePrimaryUrl: posterDataUrl("Three Body S01E02", "#172554", "#7c3aed"),
    RunTimeTicks: 2_700_000_000,
    Bitrate: 5_000_000,
    SeriesId: "series-002",
    SeasonId: "season-002-01",
    IndexNumber: 2,
    ParentIndexNumber: 1,
    UserData: {
      Played: false,
      PlaybackPositionTicks: 0,
    },
  },
];

const SEASONS: Season[] = [
  {
    Id: "season-001-01",
    Name: "第 1 季",
    SeriesId: "series-001",
    IndexNumber: 1,
    ImagePrimaryUrl: posterDataUrl("TLOU S1", "#1f2937", "#0f766e"),
  },
  {
    Id: "season-002-01",
    Name: "第 1 季",
    SeriesId: "series-002",
    IndexNumber: 1,
    ImagePrimaryUrl: posterDataUrl("三体 S1", "#172554", "#7c3aed"),
  },
];

function allMovieItems(): BaseItem[] {
  return [...HIGH_SCORE_ITEMS, ...CN_MOVIE_ITEMS];
}

function itemsByParent(parentId: string): BaseItem[] {
  if (parentId === "root-high-score") {
    return HIGH_SCORE_ITEMS;
  }
  if (parentId === "root-cn-movies") {
    return CN_MOVIE_ITEMS;
  }
  if (parentId === "root-series") {
    return SERIES_ITEMS;
  }
  return EPISODE_ITEMS.filter((ep) => ep.SeriesId === parentId);
}

function allLeafItems(): BaseItem[] {
  return [...allMovieItems(), ...SERIES_ITEMS, ...EPISODE_ITEMS];
}

function allLookupItems(): BaseItem[] {
  return [...ROOT_ITEMS, ...allLeafItems()];
}

function findPersonById(
  personId: string
): { id: string; name: string; role?: string | null } | null {
  for (const item of allLeafItems()) {
    for (const person of item.People || []) {
      if ((person.Id || "") === personId) {
        return {
          id: personId,
          name: person.Name,
          role: person.Role || null,
        };
      }
    }
  }

  return null;
}

function findLeafItem(itemId: string): BaseItem | undefined {
  return allLeafItems().find((item) => item.Id === itemId);
}

function syncFavoriteDefaultPlaylist(userId: string, itemId: string, isFavorite: boolean) {
  let defaultPlaylist = mockPlaylists.find(
    (playlist) =>
      playlist.owner_user_id === userId && (playlist.is_default || playlist.name === "我的喜欢")
  );

  if (!defaultPlaylist && isFavorite) {
    const now = new Date().toISOString();
    defaultPlaylist = {
      id: nextId("playlist-default"),
      owner_user_id: userId,
      name: "我的喜欢",
      description: "默认收藏夹",
      is_public: false,
      is_default: true,
      created_at: now,
      updated_at: now,
      items: [],
    };
    mockPlaylists.push(defaultPlaylist);
  }

  if (!defaultPlaylist) {
    return;
  }

  const existedIndex = defaultPlaylist.items.findIndex((entry) => entry.media_item_id === itemId);
  if (isFavorite) {
    if (existedIndex < 0) {
      const now = new Date().toISOString();
      defaultPlaylist.items.unshift({ media_item_id: itemId, added_at: now });
      defaultPlaylist.updated_at = now;
    }
    return;
  }

  if (existedIndex >= 0) {
    defaultPlaylist.items.splice(existedIndex, 1);
    defaultPlaylist.updated_at = new Date().toISOString();
  }
}

function setFavoriteState(userId: string, itemId: string, isFavorite: boolean): UserItemData {
  const found = findLeafItem(itemId);
  if (!found) {
    throw new Error("media item not found");
  }

  const currentUserData = found.UserData || {
    Played: false,
    PlaybackPositionTicks: 0,
  };
  found.UserData = {
    ...currentUserData,
    IsFavorite: isFavorite,
  };
  syncFavoriteDefaultPlaylist(userId, itemId, isFavorite);

  return {
    PlaybackPositionTicks: found.UserData.PlaybackPositionTicks || 0,
    PlayCount: found.UserData.Played ? 1 : 0,
    IsFavorite: Boolean(found.UserData.IsFavorite),
    Played: Boolean(found.UserData.Played),
    LastPlayedDate: null,
    ItemId: found.Id,
  };
}

function removeItemsById(items: BaseItem[], ids: Set<string>) {
  for (let index = items.length - 1; index >= 0; index -= 1) {
    if (ids.has(items[index]!.Id)) {
      items.splice(index, 1);
    }
  }
}

function removeSeasonsBySeriesId(seriesIds: Set<string>) {
  for (let index = SEASONS.length - 1; index >= 0; index -= 1) {
    if (seriesIds.has(SEASONS[index]!.SeriesId)) {
      SEASONS.splice(index, 1);
    }
  }
}

function removeSeasonsById(ids: Set<string>) {
  for (let index = SEASONS.length - 1; index >= 0; index -= 1) {
    if (ids.has(SEASONS[index]!.Id)) {
      SEASONS.splice(index, 1);
    }
  }
}

function removePlaylistItemsByMediaIds(ids: Set<string>) {
  for (const playlist of mockPlaylists) {
    playlist.items = playlist.items.filter((entry) => !ids.has(entry.media_item_id));
  }
}

function providerIdFromItem(item: BaseItem, key: string): string | undefined {
  const providers = item.ProviderIds || {};
  const exact = providers[key];
  if (exact) {
    return exact;
  }
  const found = Object.entries(providers).find(
    ([candidate]) => candidate.toLowerCase() === key.toLowerCase()
  );
  return found?.[1];
}

export interface MockUpdateItemMetadataPayload {
  Name?: string;
  Overview?: string;
  ProductionYear?: number;
  TmdbId?: string | number;
  ImdbId?: string;
  ProviderIds?: Record<string, string>;
}

interface MockPlaylistState {
  id: string;
  owner_user_id: string;
  name: string;
  description: string;
  is_public: boolean;
  is_default: boolean;
  created_at: string;
  updated_at: string;
  items: Array<{
    media_item_id: string;
    added_at: string;
  }>;
}

const mockPlaylists: MockPlaylistState[] = [
  {
    id: "playlist-default-001",
    owner_user_id: "user-admin-001",
    name: "我的喜欢",
    description: "默认收藏夹",
    is_public: false,
    is_default: true,
    created_at: minutesAgo(7200),
    updated_at: minutesAgo(60),
    items: [{ media_item_id: "movie-001", added_at: minutesAgo(60) }],
  },
  {
    id: "playlist-001",
    owner_user_id: "user-admin-001",
    name: "稍后重看",
    description: "周末补片",
    is_public: false,
    is_default: false,
    created_at: minutesAgo(3600),
    updated_at: minutesAgo(120),
    items: [
      { media_item_id: "movie-003", added_at: minutesAgo(300) },
      { media_item_id: "movie-001", added_at: minutesAgo(240) },
    ],
  },
];

function toPlaylistDto(record: MockPlaylistState): Playlist {
  return {
    id: record.id,
    owner_user_id: record.owner_user_id,
    name: record.name,
    description: record.description,
    is_public: record.is_public,
    is_default: record.is_default,
    item_count: record.items.length,
    created_at: record.created_at,
    updated_at: record.updated_at,
  };
}

const SUBTITLE_TEMPLATE: SubtitleTrack[] = [
  {
    Index: 0,
    Codec: "srt",
    Language: "chi",
    DisplayTitle: "中文字幕",
    IsExternal: true,
    IsDefault: true,
  },
  {
    Index: 1,
    Codec: "ass",
    Language: "eng",
    DisplayTitle: "English",
    IsExternal: true,
    IsDefault: false,
  },
];

const DEFAULT_USERS: AdminUser[] = [
  {
    Id: "user-admin-001",
    Name: "demo-admin",
    HasPassword: true,
    ServerId: DEMO_SERVER_ID,
    Policy: {
      IsAdministrator: true,
      IsDisabled: false,
      Role: "Admin",
    },
  },
  {
    Id: "user-operator-001",
    Name: "demo-operator",
    HasPassword: true,
    ServerId: DEMO_SERVER_ID,
    Policy: {
      IsAdministrator: false,
      IsDisabled: false,
      Role: "Viewer",
    },
  },
  {
    Id: "user-viewer-001",
    Name: "demo-viewer",
    HasPassword: true,
    ServerId: DEMO_SERVER_ID,
    Policy: {
      IsAdministrator: false,
      IsDisabled: true,
      Role: "Viewer",
    },
  },
];

const users = clone(DEFAULT_USERS);

const userProfiles = new Map<string, AdminUserProfileRecord>([
  [
    "user-admin-001",
    {
      user_id: "user-admin-001",
      email: "admin@lumenstream.local",
      display_name: "系统管理员",
      remark: "默认管理账号",
      created_at: minutesAgo(4000),
      updated_at: minutesAgo(120),
    },
  ],
  [
    "user-operator-001",
    {
      user_id: "user-operator-001",
      email: "operator@lumenstream.local",
      display_name: "运营值班",
      remark: "白天班次",
      created_at: minutesAgo(3000),
      updated_at: minutesAgo(90),
    },
  ],
  [
    "user-viewer-001",
    {
      user_id: "user-viewer-001",
      email: "viewer@lumenstream.local",
      display_name: "普通用户",
      remark: null,
      created_at: minutesAgo(2000),
      updated_at: minutesAgo(30),
    },
  ],
]);

const inviteCodeByUserId = new Map<string, string>([
  ["user-admin-001", "NMSADMIN001A"],
  ["user-operator-001", "NMSOPERA002B"],
  ["user-viewer-001", "NMSVIEWR003C"],
]);

const inviteRelations: InviteRelation[] = [
  {
    id: "invite-rel-001",
    inviter_user_id: "user-admin-001",
    inviter_username: "demo-admin",
    invitee_user_id: "user-operator-001",
    invitee_username: "demo-operator",
    invite_code: "NMSADMIN001A",
    created_at: minutesAgo(1800),
  },
];

const inviteRebates: InviteRebateRecord[] = [
  {
    id: "invite-rebate-001",
    invitee_user_id: "user-operator-001",
    invitee_username: "demo-operator",
    inviter_user_id: "user-admin-001",
    inviter_username: "demo-admin",
    recharge_order_id: "recharge-001",
    recharge_amount: "100.00",
    rebate_rate: "0.1000",
    rebate_amount: "10.00",
    created_at: minutesAgo(1600),
  },
];

const libraryStatusItems: AdminLibraryStatusItem[] = [
  {
    id: "lib-001",
    name: "Movies",
    root_path: "/media/movies",
    paths: ["/media/movies"],
    library_type: "Movie",
    enabled: true,
    scraper_policy: {},
    item_count: HIGH_SCORE_ITEMS.length + CN_MOVIE_ITEMS.length,
    last_item_updated_at: minutesAgo(15),
  },
  {
    id: "lib-002",
    name: "Shows",
    root_path: "/media/shows",
    paths: ["/media/shows", "/media/anime"],
    library_type: "Series",
    enabled: true,
    scraper_policy: {
      movie: ["tmdb", "tvdb"],
      series: ["bangumi", "tvdb", "tmdb"],
      image: ["bangumi", "tvdb", "tmdb"],
    },
    item_count: SERIES_ITEMS.length + EPISODE_ITEMS.length,
    last_item_updated_at: minutesAgo(22),
  },
];

const taskRuns: AdminTaskRun[] = [
  {
    id: "job-001",
    kind: "scan_library",
    status: "completed",
    payload: { library_id: "lib-001", mode: "incremental" },
    progress: {
      phase: "finished",
      total: 1,
      completed: 1,
      percent: 100,
      message: "任务完成",
    },
    result: { scanned: 26 },
    error: null,
    attempts: 1,
    max_attempts: 5,
    next_retry_at: null,
    dead_letter: false,
    trigger_type: "manual",
    scheduled_for: null,
    created_at: minutesAgo(90),
    started_at: minutesAgo(89),
    finished_at: minutesAgo(88),
  },
  {
    id: "job-002",
    kind: "search-reindex",
    status: "running",
    payload: { batch_size: 500 },
    progress: {
      phase: "reindex_search",
      total: 1000,
      completed: 420,
      percent: 42,
      message: "重建搜索索引",
    },
    result: null,
    error: null,
    attempts: 1,
    max_attempts: 5,
    next_retry_at: null,
    dead_letter: false,
    trigger_type: "manual",
    scheduled_for: null,
    created_at: minutesAgo(8),
    started_at: minutesAgo(7),
    finished_at: null,
  },
];

const taskDefinitions: AdminTaskDefinition[] = [
  {
    task_key: "cleanup_maintenance",
    display_name: "系统清理维护",
    enabled: true,
    cron_expr: "0 0 * * * *",
    default_payload: {},
    max_attempts: 1,
    created_at: minutesAgo(4000),
    updated_at: minutesAgo(30),
  },
  {
    task_key: "retry_dispatch",
    display_name: "失败任务重试分发",
    enabled: true,
    cron_expr: "0 * * * * *",
    default_payload: { limit: 100 },
    max_attempts: 1,
    created_at: minutesAgo(4000),
    updated_at: minutesAgo(30),
  },
  {
    task_key: "billing_expire",
    display_name: "计费过期处理",
    enabled: true,
    cron_expr: "0 */5 * * * *",
    default_payload: {},
    max_attempts: 1,
    created_at: minutesAgo(4000),
    updated_at: minutesAgo(30),
  },
  {
    task_key: "scan_library",
    display_name: "媒体库扫描",
    enabled: false,
    cron_expr: "0 */3 * * * *",
    default_payload: { mode: "incremental" },
    max_attempts: 3,
    created_at: minutesAgo(4000),
    updated_at: minutesAgo(30),
  },
  {
    task_key: "metadata_repair",
    display_name: "元数据修复",
    enabled: false,
    cron_expr: "0 30 3 * * *",
    default_payload: {},
    max_attempts: 3,
    created_at: minutesAgo(4000),
    updated_at: minutesAgo(30),
  },
  {
    task_key: "subtitle_sync",
    display_name: "字幕同步",
    enabled: false,
    cron_expr: "0 45 3 * * *",
    default_payload: { mode: "incremental" },
    max_attempts: 3,
    created_at: minutesAgo(4000),
    updated_at: minutesAgo(30),
  },
  {
    task_key: "scraper_fill",
    display_name: "刮削补齐",
    enabled: false,
    cron_expr: "0 15 4 * * *",
    default_payload: {},
    max_attempts: 3,
    created_at: minutesAgo(4000),
    updated_at: minutesAgo(30),
  },
  {
    task_key: "cache_prewarm",
    display_name: "缓存预热",
    enabled: false,
    cron_expr: "0 0 5 * * *",
    default_payload: { limit: 100 },
    max_attempts: 3,
    created_at: minutesAgo(4000),
    updated_at: minutesAgo(30),
  },
  {
    task_key: "search_reindex",
    display_name: "搜索索引重建",
    enabled: false,
    cron_expr: "0 0 2 * * *",
    default_payload: { batch_size: 500 },
    max_attempts: 3,
    created_at: minutesAgo(4000),
    updated_at: minutesAgo(30),
  },
];

const playbackSessions: PlaybackSession[] = [
  {
    id: "play-001",
    play_session_id: "play-session-001",
    user_id: "user-admin-001",
    user_name: "demo-admin",
    media_item_id: "movie-002",
    media_item_name: "教父",
    device_name: "MacBook",
    client_name: "ls-web",
    play_method: "DirectPlay",
    position_ticks: 4_800_000_000,
    is_active: true,
    last_heartbeat_at: minutesAgo(1),
    updated_at: minutesAgo(1),
  },
];

const authSessions: AuthSession[] = [
  {
    id: "auth-001",
    user_id: "user-admin-001",
    user_name: "demo-admin",
    client: "ls-web",
    device_name: "Browser",
    device_id: "mock-web-device",
    remote_addr: "127.0.0.1",
    is_active: true,
    created_at: minutesAgo(120),
    last_seen_at: minutesAgo(1),
  },
];

const apiKeys: AdminApiKey[] = [
  {
    id: "key-001",
    name: "ops-key",
    created_at: minutesAgo(200),
    last_used_at: minutesAgo(30),
  },
  {
    id: "key-002",
    name: "readonly-key",
    created_at: minutesAgo(300),
    last_used_at: minutesAgo(250),
  },
];

const storageConfigs: Record<string, unknown>[] = [
  {
    id: "storage-001",
    kind: "gdrive",
    name: "gdrive-main",
    enabled: true,
    config: {
      account: "demo@mock",
      route: "gdrive",
    },
  },
];

const playbackDomains: PlaybackDomain[] = [
  {
    id: "domain-001",
    name: "A 域名",
    base_url: "https://lumenbackend-a.example.com",
    enabled: true,
    priority: 100,
    is_default: true,
    lumenbackend_node_id: "node-a",
    traffic_multiplier: 1,
    created_at: minutesAgo(600),
    updated_at: minutesAgo(12),
  },
  {
    id: "domain-002",
    name: "B 域名",
    base_url: "https://lumenbackend-b.example.com",
    enabled: true,
    priority: 80,
    is_default: false,
    lumenbackend_node_id: "node-b",
    traffic_multiplier: 1.3,
    created_at: minutesAgo(580),
    updated_at: minutesAgo(30),
  },
];

const lumenbackendNodes: LumenBackendNode[] = [
  {
    node_id: "node-a",
    name: "LumenBackend A",
    enabled: true,
    last_seen_at: minutesAgo(1),
    last_version: "0.1.0",
    last_status: {
      active_streams: 12,
      cpu_usage: 0.37,
      memory_usage: 0.41,
    },
    created_at: minutesAgo(2000),
    updated_at: minutesAgo(1),
  },
  {
    node_id: "node-b",
    name: "LumenBackend B",
    enabled: true,
    last_seen_at: minutesAgo(4),
    last_version: "0.1.0",
    last_status: {
      active_streams: 8,
      cpu_usage: 0.28,
      memory_usage: 0.33,
    },
    created_at: minutesAgo(1980),
    updated_at: minutesAgo(4),
  },
];

const defaultRuntimeSchema: LumenBackendRuntimeSchemaDefinition = {
  sections: [
    {
      id: "network",
      title: "节点网络",
      description: "LumenBackend 服务监听地址",
      fields: [
        {
          key: "server.listen_host",
          label: "Listen Host",
          type: "string",
          required: true,
          default: "0.0.0.0",
        },
        {
          key: "server.listen_port",
          label: "Listen Port",
          type: "number",
          required: true,
          default: 8080,
          validators: {
            min: 1,
            max: 65535,
          },
        },
      ],
    },
    {
      id: "backend",
      title: "后端依赖",
      fields: [
        {
          key: "mysql.dsn",
          label: "MySQL DSN",
          type: "password",
          required: true,
          default: "***",
        },
        {
          key: "redis.dsn",
          label: "Redis DSN",
          type: "password",
          required: true,
          default: "***",
        },
      ],
    },
    {
      id: "cache",
      title: "S3 缓存",
      fields: [
        {
          key: "s3_cache.enabled",
          label: "启用 S3 Cache",
          type: "boolean",
          default: false,
        },
        {
          key: "s3_cache.bucket",
          label: "S3 Bucket",
          type: "string",
          default: "",
        },
        {
          key: "s3_cache.region",
          label: "S3 Region",
          type: "string",
          default: "",
        },
      ],
    },
  ],
};

const lumenbackendNodeSchemas = new Map<string, LumenBackendNodeRuntimeSchema>([
  [
    "node-a",
    {
      node_id: "node-a",
      schema_version: "2026.02.1",
      schema_hash: "mock-schema-v1",
      schema: clone(defaultRuntimeSchema),
      updated_at: minutesAgo(3),
    },
  ],
  [
    "node-b",
    {
      node_id: "node-b",
      schema_version: "2026.02.1",
      schema_hash: "mock-schema-v1",
      schema: clone(defaultRuntimeSchema),
      updated_at: minutesAgo(5),
    },
  ],
]);

const lumenbackendNodeConfigs = new Map<string, LumenBackendNodeRuntimeConfig>([
  [
    "node-a",
    {
      node_id: "node-a",
      version: 1,
      config: {
        server: {
          listen_host: "0.0.0.0",
          listen_port: 8080,
        },
        mysql: {
          dsn: "***",
        },
        redis: {
          dsn: "***",
        },
        s3_cache: {
          enabled: false,
          bucket: "",
          region: "",
        },
      },
    },
  ],
]);

const userPlaybackDomainSelection = new Map<string, string>([
  ["user-admin-001", "domain-001"],
  ["user-operator-001", "domain-002"],
]);

let settings: WebAppSettings = {
  server: {
    host: "0.0.0.0",
    port: 8096,
    base_url: "http://127.0.0.1:8096",
    cors_allow_origins: ["http://127.0.0.1:4321"],
  },
  auth: {
    token_ttl_hours: 24 * 30,
    bootstrap_admin_user: "admin",
    bootstrap_admin_password: "******",
    admin_api_key_prefix: "lsadm",
    max_failed_attempts: 10,
    risk_window_seconds: 300,
    risk_block_seconds: 900,
    invite: {
      force_on_register: false,
      invitee_bonus_enabled: false,
      invitee_bonus_amount: "0.00",
      inviter_rebate_enabled: false,
      inviter_rebate_rate: "0.0000",
    },
  },
  scan: {
    default_library_name: "Default Library",
    local_media_exts: ["mp4", "mkv", "flv", "avi", "mov", "m4v", "ts", "m2ts", "wmv", "iso"],
    incremental_window_hours: 24,
  },
  storage: {
    lumenbackend_enabled: false,
    prefer_segment_gateway: false,
    lumenbackend_nodes: [],
    local_stream_route: "v1/streams/local",
  },
  tmdb: {
    enabled: false,
    api_key: "",
    language: "zh-CN",
    timeout_seconds: 10,
    request_interval_ms: 350,
    cache_ttl_seconds: 86400,
    retry_attempts: 3,
    retry_backoff_ms: 2000,
  },
  scraper: {
    enabled: false,
    default_strategy: "primary_with_fallback",
    providers: ["tmdb", "tvdb", "bangumi"],
    default_routes: {
      movie: ["tmdb", "tvdb"],
      series: ["tmdb", "tvdb"],
      image: ["tmdb", "tvdb"],
    },
    tvdb: {
      enabled: true,
      base_url: "https://api4.thetvdb.com/v4",
      api_key: "",
      pin: "",
      timeout_seconds: 15,
    },
    bangumi: {
      enabled: false,
      base_url: "https://api.bgm.tv",
      access_token: "",
      timeout_seconds: 15,
      user_agent: "lumenstream/0.1",
    },
  },
  security: {
    admin_allow_ips: [],
    trust_x_forwarded_for: true,
    redact_sensitive_logs: true,
  },
  observability: {
    metrics_enabled: true,
    traces_enabled: false,
  },
  jobs: {
    retry_base_seconds: 30,
    retry_max_seconds: 1800,
  },
  agent: {
    enabled: true,
    auto_mode: "automatic",
    missing_scan_enabled: true,
    missing_scan_cron: "0 */30 * * * *",
    auto_close_on_library_hit: true,
    review_required_on_parse_ambiguity: true,
    feedback_auto_route: true,
    llm: {
      enabled: false,
      base_url: "https://api.openai.com/v1",
      api_key: "",
      model: "gpt-4o-mini",
    },
    moviepilot: {
      enabled: true,
      base_url: "https://moviepilot.example.com",
      username: "admin",
      password: "***",
      timeout_seconds: 20,
      search_download_enabled: true,
      subscribe_fallback_enabled: true,
      filter: {
        min_seeders: 5,
        max_movie_size_gb: 30,
        max_episode_size_gb: 5,
        preferred_resource_pix: ["2160P", "4K", "1080P"],
        preferred_video_encode: ["X265", "H265", "X264"],
        preferred_resource_type: ["WEB-DL", "BluRay"],
        preferred_labels: ["中字", "中文"],
        excluded_keywords: ["CAM", "TS", "TC"],
      },
    },
  },
};

function currentSystemCapabilities(): AdminSystemCapabilities {
  return {
    edition: "ce",
    strm_only_streaming: false,
    transcoding_enabled: false,
    billing_enabled: false,
    advanced_traffic_controls_enabled: false,
    invite_rewards_enabled: false,
    audit_log_export_enabled: false,
    request_agent_enabled: true,
    playback_routing_enabled: true,
    supported_stream_features: [
      "strm-direct-play",
      "local-file-range",
      "http-range",
      "segment-gateway",
      "distributed-fallback",
      "lumenbackend-302-redirect",
    ],
  };
}

const auditLogs: AuditLogEntry[] = [
  {
    id: "audit-001",
    actor_user_id: "user-admin-001",
    actor_username: "demo-admin",
    action: "admin.job.scan.enqueue",
    target_type: "job",
    target_id: "job-001",
    detail: {
      library_id: "lib-001",
      mode: "incremental",
    },
    created_at: minutesAgo(90),
  },
  {
    id: "audit-002",
    actor_user_id: "user-admin-001",
    actor_username: "demo-admin",
    action: "admin.user.disable",
    target_type: "user",
    target_id: "user-viewer-001",
    detail: {
      enabled: false,
    },
    created_at: minutesAgo(32),
  },
];

function pushAudit(
  action: string,
  targetType: string,
  targetId: string | null,
  detail: Record<string, unknown>
): void {
  auditLogs.unshift({
    id: nextId("audit"),
    actor_user_id: "user-admin-001",
    actor_username: "demo-admin",
    action,
    target_type: targetType,
    target_id: targetId,
    detail,
    created_at: new Date().toISOString(),
  });
}

function userForName(username: string): AdminUser {
  const found = users.find((item) => item.Name.toLowerCase() === username.toLowerCase());
  if (found) {
    return clone(found);
  }

  return clone(users[0]!);
}

function userById(userId: string): AdminUser | undefined {
  return users.find((item) => item.Id === userId);
}

function nextInviteCode(): string {
  const seed = nextId("invite-code")
    .replace(/[^0-9]/g, "")
    .padStart(8, "0");
  return `LS${seed}X`.slice(0, 12).toUpperCase();
}

function ensureInviteCodeForUser(userId: string): string {
  const existing = inviteCodeByUserId.get(userId);
  if (existing) {
    return existing;
  }
  const next = nextInviteCode();
  inviteCodeByUserId.set(userId, next);
  return next;
}

export function mockCurrentDemoUser(): User {
  return clone(users[0]!);
}

export async function mockAuthenticateByName(
  username: string,
  _password: string
): Promise<AuthResult> {
  const user = userForName(username || "demo-admin");

  return {
    User: user,
    SessionInfo: {
      Id: nextId("session"),
      UserId: user.Id,
      UserName: user.Name,
      Client: "ls-web-mock",
      DeviceName: "browser",
      DeviceId: "mock-device",
    },
    AccessToken: DEMO_TOKEN,
    ServerId: DEMO_SERVER_ID,
  };
}

export async function mockRegisterWithInvite(payload: {
  username: string;
  password: string;
  invite_code?: string;
}): Promise<AuthResult> {
  const username = payload.username.trim();
  if (!username) {
    throw new Error("username is required");
  }
  if (payload.password.length < 6) {
    throw new Error("password too short (min 6 chars)");
  }
  if (users.some((item) => item.Name.toLowerCase() === username.toLowerCase())) {
    throw new Error("username already exists");
  }

  const inviteCode = payload.invite_code?.trim().toUpperCase() || "";
  if (settings.auth.invite.force_on_register && !inviteCode) {
    throw new Error("invite code is required");
  }

  let inviterUser: AdminUser | undefined;
  if (inviteCode) {
    const inviterUserId = [...inviteCodeByUserId.entries()].find(
      ([, code]) => code === inviteCode
    )?.[0];
    if (!inviterUserId) {
      throw new Error("invite code is invalid");
    }
    inviterUser = userById(inviterUserId);
  }

  const created: AdminUser = {
    Id: nextId("user"),
    Name: username,
    HasPassword: true,
    ServerId: DEMO_SERVER_ID,
    Policy: roleToPolicy("Viewer"),
  };

  users.unshift(created);
  userProfiles.set(created.Id, {
    user_id: created.Id,
    email: null,
    display_name: username,
    remark: null,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  });
  ensureInviteCodeForUser(created.Id);

  if (inviterUser && inviteCode) {
    inviteRelations.unshift({
      id: nextId("invite-rel"),
      inviter_user_id: inviterUser.Id,
      inviter_username: inviterUser.Name,
      invitee_user_id: created.Id,
      invitee_username: created.Name,
      invite_code: inviteCode,
      created_at: new Date().toISOString(),
    });
  }

  pushAudit("user.register", "user", created.Id, {
    invite_code_present: Boolean(inviteCode),
  });

  return {
    User: clone(created),
    SessionInfo: {
      Id: nextId("session"),
      UserId: created.Id,
      UserName: created.Name,
      Client: "ls-web-mock",
      DeviceName: "browser",
      DeviceId: "mock-device",
    },
    AccessToken: DEMO_TOKEN,
    ServerId: DEMO_SERVER_ID,
  };
}

export async function mockGetMyInviteSummary(userId: string): Promise<InviteSummary> {
  const capabilities = currentSystemCapabilities();
  const user = userById(userId);
  if (!user) {
    throw new Error("user not found");
  }

  const code = ensureInviteCodeForUser(userId);
  const invitedCount = inviteRelations.filter((item) => item.inviter_user_id === userId).length;
  const rebateTotal = inviteRebates
    .filter((item) => item.inviter_user_id === userId)
    .reduce((sum, item) => sum + Number(item.rebate_amount), 0);

  const summary: InviteSummary = {
    code,
    enabled: true,
    invited_count: invitedCount,
  };
  if (capabilities.invite_rewards_enabled) {
    summary.rebate_total = rebateTotal.toFixed(2);
    summary.invitee_bonus_enabled = settings.auth.invite.invitee_bonus_enabled;
  }
  return summary;
}

export async function mockResetMyInviteCode(userId: string): Promise<InviteSummary> {
  const user = userById(userId);
  if (!user) {
    throw new Error("user not found");
  }

  const nextCode = nextInviteCode();
  inviteCodeByUserId.set(userId, nextCode);
  pushAudit("user.invite.reset", "user", userId, {});
  return mockGetMyInviteSummary(userId);
}

export async function mockGetUserById(userId: string): Promise<User> {
  const user = users.find((item) => item.Id === userId) || users[0]!;
  return clone(user);
}

export async function mockLogout(): Promise<void> {
  return;
}

function todayKey(): string {
  return new Date().toISOString().slice(0, 10);
}

function hashKey(input: string): number {
  return [...input].reduce((sum, ch) => sum + ch.charCodeAt(0), 0);
}

function scoreForTopPlayed(item: BaseItem, statDate: string): number {
  const rating = typeof item.CommunityRating === "number" ? item.CommunityRating : 6.5;
  const ratingBase = Math.round(rating * 10);
  const resumeBoost = (item.UserData?.PlaybackPositionTicks || 0) > 0 ? 24 : 0;
  const dayVariance = hashKey(`${statDate}:${item.Id}`) % 40;
  return ratingBase + resumeBoost + dayVariance;
}

export async function mockGetTopPlayed(
  limit = 10,
  windowDays = 1,
  statDate?: string
): Promise<TopPlayedSummary> {
  const safeLimit = Math.max(1, Math.min(100, Math.floor(limit)));
  const safeWindowDays = Math.max(1, Math.min(90, Math.floor(windowDays)));
  const day = /^\d{4}-\d{2}-\d{2}$/.test(statDate || "") ? (statDate as string) : todayKey();

  const items = allLeafItems()
    .slice()
    .sort((left, right) => scoreForTopPlayed(right, day) - scoreForTopPlayed(left, day))
    .slice(0, safeLimit)
    .map((item) => {
      const score = scoreForTopPlayed(item, day);
      const uniqueUsers = Math.max(
        1,
        Math.min(score, 1 + (hashKey(`${item.Id}:users:${day}`) % 18))
      );

      return {
        Id: item.Id,
        Name: item.Name,
        Type: item.Type,
        RunTimeTicks: item.RunTimeTicks,
        Bitrate: item.Bitrate,
        ProductionYear: item.ProductionYear,
        CommunityRating: item.CommunityRating,
        Overview: item.Overview,
        PlayCount: score,
        UniqueUsers: uniqueUsers,
      } satisfies TopPlayedItem;
    });

  return {
    StatDate: day,
    WindowDays: safeWindowDays,
    Items: clone(items),
  };
}

export async function mockGetRootItems(_userId: string): Promise<QueryResult<BaseItem>> {
  return toQueryResult(ROOT_ITEMS, 0, ROOT_ITEMS.length);
}

export async function mockGetResumeItems(_userId: string): Promise<QueryResult<BaseItem>> {
  const resume = allLeafItems().filter((item) => (item.UserData?.PlaybackPositionTicks || 0) > 0);
  return toQueryResult(resume, 0, 24);
}

export interface MockItemsQuery {
  parentId?: string;
  includeItemTypes?: string;
  personIds?: string;
  searchTerm?: string;
  filters?: string;
  limit?: number;
  startIndex?: number;
}

function applyItemFilters(items: BaseItem[], query: MockItemsQuery): BaseItem[] {
  let filtered = [...items];

  const includeTypes = parseCsv(query.includeItemTypes);
  if (includeTypes.length > 0) {
    filtered = filtered.filter((item) => includeTypes.includes(item.Type));
  }

  if (query.searchTerm) {
    const keyword = query.searchTerm.toLowerCase();
    filtered = filtered.filter((item) => item.Name.toLowerCase().includes(keyword));
  }

  const personIds = parseCsv(query.personIds);
  if (personIds.length > 0) {
    filtered = filtered.filter((item) => {
      const itemPersonIds = (item.People || [])
        .map((person) => person.Id || "")
        .filter((id) => id.length > 0);

      return itemPersonIds.some((id) => personIds.includes(id));
    });
  }

  if ((query.filters || "").includes("IsResumable")) {
    filtered = filtered.filter((item) => (item.UserData?.PlaybackPositionTicks || 0) > 0);
  }

  if ((query.filters || "").includes("IsFavorite")) {
    filtered = filtered.filter((item) => Boolean(item.UserData?.IsFavorite));
  }

  return filtered;
}

export async function mockGetUserItems(
  _userId: string,
  query: MockItemsQuery = {}
): Promise<QueryResult<BaseItem>> {
  const source = query.parentId ? itemsByParent(query.parentId) : allLeafItems();
  const filtered = applyItemFilters(source, query);
  return toQueryResult(filtered, query.startIndex || 0, query.limit || 100);
}

export async function mockGetItems(query: MockItemsQuery = {}): Promise<QueryResult<BaseItem>> {
  const source = query.parentId ? itemsByParent(query.parentId) : allLeafItems();
  const filtered = applyItemFilters(source, query);
  return toQueryResult(filtered, query.startIndex || 0, query.limit || 100);
}

export async function mockGetUserItem(_userId: string, itemId: string): Promise<BaseItem> {
  const found = allLookupItems().find((item) => item.Id === itemId);
  return clone(found || allMovieItems()[0]!);
}

export async function mockGetPerson(personId: string): Promise<BaseItem> {
  const person = findPersonById(personId);
  if (!person) {
    return {
      Id: personId,
      Name: "Unknown Person",
      Type: "Person",
      Path: "",
      Overview: "未找到该人物的模拟资料。",
    };
  }

  return {
    Id: person.id,
    Name: person.name,
    Type: "Person",
    Path: "",
    Overview: person.role
      ? `Mock 人物资料：代表角色为 ${person.role}。`
      : "Mock 人物资料：暂无角色描述。",
  };
}

export async function mockGetItemCounts(): Promise<ItemCounts> {
  const all = allLeafItems();
  const countByType = (type: string) => all.filter((item) => item.Type === type).length;

  return {
    MovieCount: countByType("Movie"),
    SeriesCount: countByType("Series"),
    EpisodeCount: countByType("Episode"),
    SongCount: 0,
    AlbumCount: 0,
    ArtistCount: 0,
    ProgramCount: 0,
    TrailerCount: 0,
  };
}

export async function mockGetShowSeasons(showId: string): Promise<QueryResult<Season>> {
  const seasons = SEASONS.filter((season) => season.SeriesId === showId);
  return {
    Items: clone(seasons),
    TotalRecordCount: seasons.length,
    StartIndex: 0,
  };
}

export async function mockGetShowEpisodes(
  showId: string,
  seasonId?: string
): Promise<QueryResult<BaseItem>> {
  let episodes = EPISODE_ITEMS.filter((ep) => ep.SeriesId === showId);
  if (seasonId) {
    episodes = episodes.filter((ep) => ep.SeasonId === seasonId);
  }
  return toQueryResult(episodes, 0, 100);
}

export async function mockGetPlaybackInfo(itemId: string, _userId: string): Promise<PlaybackInfo> {
  return {
    MediaSources: [
      {
        Id: `ms-${itemId}`,
        Path: `https://demo.lumenstream.local/Videos/${itemId}/stream`,
        Protocol: "Http",
        Container: "mkv",
        RunTimeTicks: 8_000_000_000,
        Bitrate: 9_000_000,
        SupportsDirectPlay: true,
        SupportsDirectStream: true,
        SupportsTranscoding: false,
        MediaStreams: [
          {
            Index: 0,
            Type: "Video",
            Language: null,
            IsExternal: false,
            Path: null,
            Codec: "h264",
            BitRate: 4_500_000,
            IsDefault: true,
          },
          {
            Index: 1,
            Type: "Audio",
            Language: "eng",
            IsExternal: false,
            Path: null,
            Codec: "aac",
            Channels: 2,
            BitRate: 192_000,
            IsDefault: true,
          },
          {
            Index: 2,
            Type: "Audio",
            Language: "jpn",
            IsExternal: false,
            Path: null,
            Codec: "aac",
            Channels: 6,
            BitRate: 384_000,
            IsDefault: false,
          },
          {
            Index: 3,
            Type: "Subtitle",
            Language: "zho",
            IsExternal: true,
            Path: "/mock/movie.zh.ass",
            Codec: "ass",
            DisplayTitle: "ZHO (ASS)",
            IsDefault: true,
          },
        ],
      },
    ],
    PlaySessionId: nextId("play-session"),
  };
}

export async function mockGetItemSubtitles(_itemId: string): Promise<SubtitleTrack[]> {
  return clone(SUBTITLE_TEMPLATE);
}

export async function mockAddFavoriteItem(_userId: string, itemId: string): Promise<UserItemData> {
  return clone(setFavoriteState(_userId || currentMockUserId(), itemId, true));
}

export async function mockRemoveFavoriteItem(
  _userId: string,
  itemId: string
): Promise<UserItemData> {
  return clone(setFavoriteState(_userId || currentMockUserId(), itemId, false));
}

export async function mockUpdateItemMetadata(
  itemId: string,
  payload: MockUpdateItemMetadataPayload
): Promise<void> {
  const found = findLeafItem(itemId);
  if (!found) {
    throw new Error("media item not found");
  }

  const nextName = payload.Name?.trim();
  if (nextName) {
    found.Name = nextName;
  }
  if (typeof payload.Overview === "string") {
    const nextOverview = payload.Overview.trim();
    if (nextOverview) {
      found.Overview = nextOverview;
    }
  }
  if (typeof payload.ProductionYear === "number" && Number.isFinite(payload.ProductionYear)) {
    found.ProductionYear = Math.trunc(payload.ProductionYear);
  }

  const providerIds: Record<string, string> = {
    ...(found.ProviderIds || {}),
    ...(payload.ProviderIds || {}),
  };
  const tmdbId = payload.TmdbId?.toString().trim();
  const imdbId = payload.ImdbId?.trim();
  if (tmdbId) {
    providerIds.Tmdb = tmdbId;
  }
  if (imdbId) {
    providerIds.Imdb = imdbId;
  }
  if (Object.keys(providerIds).length > 0) {
    found.ProviderIds = providerIds;
  }
}

export async function mockRefreshItemMetadata(itemId: string): Promise<void> {
  const found = findLeafItem(itemId);
  if (!found) {
    throw new Error("media item not found");
  }

  if (!found.ProviderIds?.Tmdb) {
    const existingTmdb = providerIdFromItem(found, "Tmdb");
    if (existingTmdb) {
      found.ProviderIds = {
        ...(found.ProviderIds || {}),
        Tmdb: existingTmdb,
      };
    }
  }
}

export async function mockDeleteItem(itemId: string): Promise<void> {
  const target = findLeafItem(itemId);
  if (!target) {
    throw new Error("media item not found");
  }

  const deleteIds = new Set<string>([itemId]);
  const deleteSeriesIds = new Set<string>();

  if (target.Type === "Series") {
    deleteSeriesIds.add(itemId);
    for (const episode of EPISODE_ITEMS) {
      if (episode.SeriesId === itemId) {
        deleteIds.add(episode.Id);
      }
    }
  } else if (target.Type === "Season") {
    for (const episode of EPISODE_ITEMS) {
      if (episode.SeasonId === itemId) {
        deleteIds.add(episode.Id);
      }
    }
  }

  removeItemsById(HIGH_SCORE_ITEMS, deleteIds);
  removeItemsById(CN_MOVIE_ITEMS, deleteIds);
  removeItemsById(SERIES_ITEMS, deleteIds);
  removeItemsById(EPISODE_ITEMS, deleteIds);

  if (deleteSeriesIds.size > 0) {
    removeSeasonsBySeriesId(deleteSeriesIds);
  }
  removeSeasonsById(deleteIds);
  removePlaylistItemsByMediaIds(deleteIds);
}

function currentMockUserId(): string {
  return users[0]?.Id || "user-admin-001";
}

export async function mockListMyPlaylists(): Promise<Playlist[]> {
  const userId = currentMockUserId();
  return clone(
    mockPlaylists
      .filter((playlist) => playlist.owner_user_id === userId)
      .sort((a, b) => {
        // Default playlist always first
        if (a.is_default !== b.is_default) {
          return a.is_default ? -1 : 1;
        }
        return b.updated_at.localeCompare(a.updated_at);
      })
      .map(toPlaylistDto)
  );
}

export async function mockListPublicPlaylistsByUser(userId: string): Promise<Playlist[]> {
  return clone(
    mockPlaylists
      .filter((playlist) => playlist.owner_user_id === userId && playlist.is_public)
      .sort((a, b) => {
        if (a.is_default !== b.is_default) {
          return a.is_default ? -1 : 1;
        }
        return b.updated_at.localeCompare(a.updated_at);
      })
      .map(toPlaylistDto)
  );
}

export async function mockCreatePlaylist(payload: CreatePlaylistPayload): Promise<Playlist> {
  const userId = currentMockUserId();
  const name = payload.name?.trim() || "";
  if (!name) {
    throw new Error("playlist name is required");
  }

  const duplicated = mockPlaylists.find(
    (playlist) => playlist.owner_user_id === userId && playlist.name === name
  );
  if (duplicated) {
    throw new Error("playlist conflict");
  }

  const now = new Date().toISOString();
  const created: MockPlaylistState = {
    id: nextId("playlist"),
    owner_user_id: userId,
    name,
    description: payload.description?.trim() || "",
    is_public: Boolean(payload.is_public),
    is_default: false,
    created_at: now,
    updated_at: now,
    items: [],
  };
  mockPlaylists.push(created);
  return clone(toPlaylistDto(created));
}

export async function mockGetPlaylist(playlistId: string): Promise<Playlist> {
  const userId = currentMockUserId();
  const found = mockPlaylists.find((playlist) => playlist.id === playlistId);
  if (!found) {
    throw new Error("playlist not found");
  }
  if (found.owner_user_id !== userId && !found.is_public) {
    throw new Error("playlist access denied");
  }
  return clone(toPlaylistDto(found));
}

export async function mockUpdatePlaylist(
  playlistId: string,
  payload: UpdatePlaylistPayload
): Promise<Playlist> {
  const userId = currentMockUserId();
  const found = mockPlaylists.find((playlist) => playlist.id === playlistId);
  if (!found) {
    throw new Error("playlist not found");
  }
  if (found.owner_user_id !== userId) {
    throw new Error("playlist access denied");
  }

  // Default playlist cannot be renamed
  const nextName = found.is_default
    ? found.name
    : payload.name !== undefined
      ? payload.name.trim()
      : found.name;
  if (!nextName) {
    throw new Error("playlist name is required");
  }
  const duplicated = mockPlaylists.find(
    (playlist) =>
      playlist.id !== playlistId && playlist.owner_user_id === userId && playlist.name === nextName
  );
  if (duplicated) {
    throw new Error("playlist conflict");
  }

  found.name = nextName;
  if (payload.description !== undefined) {
    found.description = payload.description.trim();
  }
  if (payload.is_public !== undefined) {
    found.is_public = payload.is_public;
  }
  found.updated_at = new Date().toISOString();
  return clone(toPlaylistDto(found));
}

export async function mockDeletePlaylist(playlistId: string): Promise<{ deleted: boolean }> {
  const userId = currentMockUserId();
  const index = mockPlaylists.findIndex((playlist) => playlist.id === playlistId);
  if (index < 0) {
    throw new Error("playlist not found");
  }
  const playlist = mockPlaylists[index]!;
  if (playlist.owner_user_id !== userId) {
    throw new Error("playlist access denied");
  }
  if (playlist.is_default) {
    throw new Error("cannot delete default playlist");
  }
  mockPlaylists.splice(index, 1);
  return { deleted: true };
}

export async function mockListPlaylistItems(playlistId: string): Promise<PlaylistItemsResponse> {
  const userId = currentMockUserId();
  const found = mockPlaylists.find((playlist) => playlist.id === playlistId);
  if (!found) {
    throw new Error("playlist not found");
  }
  if (found.owner_user_id !== userId && !found.is_public) {
    throw new Error("playlist access denied");
  }

  const items = found.items
    .slice()
    .sort((a, b) => b.added_at.localeCompare(a.added_at))
    .map((entry) => findLeafItem(entry.media_item_id))
    .filter((item): item is BaseItem => Boolean(item))
    .map((item) => clone(item));

  return {
    items,
    total: items.length,
  };
}

export async function mockAddItemToPlaylist(
  playlistId: string,
  itemId: string
): Promise<PlaylistItem> {
  const userId = currentMockUserId();
  const found = mockPlaylists.find((playlist) => playlist.id === playlistId);
  if (!found) {
    throw new Error("playlist not found");
  }
  if (found.owner_user_id !== userId) {
    throw new Error("playlist access denied");
  }
  if (!findLeafItem(itemId)) {
    throw new Error("media item not found");
  }
  if (found.items.some((entry) => entry.media_item_id === itemId)) {
    throw new Error("playlist conflict");
  }

  const added_at = new Date().toISOString();
  found.items.unshift({
    media_item_id: itemId,
    added_at,
  });
  found.updated_at = added_at;

  return {
    playlist_id: found.id,
    media_item_id: itemId,
    added_at,
  };
}

export async function mockRemoveItemFromPlaylist(
  playlistId: string,
  itemId: string
): Promise<{ removed: boolean }> {
  const userId = currentMockUserId();
  const found = mockPlaylists.find((playlist) => playlist.id === playlistId);
  if (!found) {
    throw new Error("playlist not found");
  }
  if (found.owner_user_id !== userId) {
    throw new Error("playlist access denied");
  }

  const before = found.items.length;
  found.items = found.items.filter((entry) => entry.media_item_id !== itemId);
  if (found.items.length !== before) {
    found.updated_at = new Date().toISOString();
    return { removed: true };
  }
  return { removed: false };
}

export async function mockGetMePlaybackDomains(): Promise<MePlaybackDomainsResponse> {
  const selected = userPlaybackDomainSelection.get(currentMockUserId()) || null;
  const defaultDomain = playbackDomains.find((item) => item.is_default && item.enabled)?.id || null;
  return {
    selected_domain_id: selected,
    default_domain_id: defaultDomain,
    available: clone(playbackDomains.filter((item) => item.enabled)),
  };
}

export async function mockSelectMePlaybackDomain(domainId: string): Promise<{
  selected_domain_id: string;
  selected_domain_name: string;
}> {
  const domain = playbackDomains.find((item) => item.id === domainId && item.enabled);
  if (!domain) {
    throw new Error("playback domain not found");
  }

  const userId = currentMockUserId();
  userPlaybackDomainSelection.set(userId, domain.id);
  pushAudit("user.playback_domain.select", "playback_domain", domain.id, {
    user_id: userId,
    domain_name: domain.name,
  });
  return {
    selected_domain_id: domain.id,
    selected_domain_name: domain.name,
  };
}

export async function mockGetSystemSummary(): Promise<AdminSystemSummary> {
  const jobsByStatus: Record<string, number> = {};
  taskRuns.forEach((job) => {
    jobsByStatus[job.status] = (jobsByStatus[job.status] || 0) + 1;
  });

  return {
    generated_at_utc: new Date().toISOString(),
    server_id: DEMO_SERVER_ID,
    transcoding_enabled: false,
    libraries_total: libraryStatusItems.length,
    libraries_enabled: libraryStatusItems.filter((item) => item.enabled).length,
    media_items_total: allLeafItems().length,
    users_total: users.length,
    users_disabled: users.filter((item) => item.Policy.IsDisabled).length,
    active_playback_sessions: playbackSessions.filter((item) => item.is_active).length,
    active_auth_sessions: authSessions.filter((item) => item.is_active).length,
    jobs_by_status: jobsByStatus,
    infra_metrics: {
      requests_total: 3210,
      status_2xx: 3132,
      status_4xx: 58,
      status_5xx: 20,
      latency_p95_ms: 38,
      scraper_http_requests_total: 148,
      scraper_cache_hits_total: 96,
      scraper_cache_misses_total: 52,
      scraper_hit_rate: 0.6486,
      scraper_success_total: 122,
      scraper_failure_total: 7,
    },
  };
}

export async function mockGetSystemFlags(): Promise<AdminSystemFlags> {
  const scraperEnabled =
    Boolean((settings.scraper as { enabled?: boolean }).enabled) ||
    Boolean((settings.tmdb as { enabled?: boolean }).enabled);
  return {
    strm_only_streaming: false,
    transcoding_enabled: false,
    scraper_enabled: scraperEnabled,
    tmdb_enabled: scraperEnabled,
    lumenbackend_enabled: Boolean(
      (settings.storage as { lumenbackend_enabled?: boolean }).lumenbackend_enabled
    ),
    prefer_segment_gateway: Boolean(
      (settings.storage as { prefer_segment_gateway?: boolean }).prefer_segment_gateway
    ),
    metrics_enabled: Boolean(
      (settings.observability as { metrics_enabled?: boolean }).metrics_enabled
    ),
  };
}

export async function mockUpdateSystemFlags(
  flags: Partial<
    Pick<
      AdminSystemFlags,
      | "scraper_enabled"
      | "tmdb_enabled"
      | "lumenbackend_enabled"
      | "prefer_segment_gateway"
      | "metrics_enabled"
    >
  >
): Promise<AdminSystemFlags> {
  const scraperEnabled = flags.scraper_enabled ?? flags.tmdb_enabled;
  if (scraperEnabled !== undefined) {
    (settings.scraper as { enabled?: boolean }).enabled = scraperEnabled;
    (settings.tmdb as { enabled?: boolean }).enabled = scraperEnabled;
  }
  if (flags.lumenbackend_enabled !== undefined) {
    (settings.storage as { lumenbackend_enabled?: boolean }).lumenbackend_enabled =
      flags.lumenbackend_enabled;
  }
  if (flags.prefer_segment_gateway !== undefined) {
    (settings.storage as { prefer_segment_gateway?: boolean }).prefer_segment_gateway =
      flags.prefer_segment_gateway;
  }
  if (flags.metrics_enabled !== undefined) {
    (settings.observability as { metrics_enabled?: boolean }).metrics_enabled =
      flags.metrics_enabled;
  }
  pushAudit(
    "admin.system.flags.upsert",
    "web_settings",
    "global",
    flags as unknown as Record<string, unknown>
  );
  return mockGetSystemFlags();
}

export async function mockGetSystemCapabilities(): Promise<AdminSystemCapabilities> {
  return currentSystemCapabilities();
}

export async function mockListUsers(): Promise<AdminUser[]> {
  return clone(users);
}

function roleToPolicy(role: UserRole): AdminUser["Policy"] {
  return {
    IsAdministrator: role === "Admin",
    IsDisabled: false,
    Role: role,
  };
}

function defaultUserProfile(userId: string): AdminUserProfileRecord {
  return {
    user_id: userId,
    email: null,
    display_name: null,
    remark: null,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  };
}

function compareNullableText(left: string | null, right: string | null): number {
  if (left === right) return 0;
  if (left === null) return -1;
  if (right === null) return 1;
  return left.localeCompare(right, "zh-CN");
}

function buildUserSummaryItem(user: AdminUser): AdminUserSummaryItem {
  const capabilities = currentSystemCapabilities();
  const profile = userProfiles.get(user.Id) ?? defaultUserProfile(user.Id);
  const subscriptions = adminUserSubscriptions.get(user.Id) ?? [];
  const activeSubscription = subscriptions.find((item) => item.status === "active") ?? null;
  const usage = trafficUsages.get(user.Id);

  return {
    id: user.Id,
    username: user.Name,
    email: profile.email,
    display_name: profile.display_name,
    role: user.Policy.Role || (user.Policy.IsAdministrator ? "Admin" : "Viewer"),
    is_admin: user.Policy.IsAdministrator,
    is_disabled: user.Policy.IsDisabled,
    active_auth_sessions: authSessions.filter((item) => item.user_id === user.Id && item.is_active)
      .length,
    active_playback_sessions: playbackSessions.filter(
      (item) => item.user_id === user.Id && item.is_active
    ).length,
    subscription_name: capabilities.billing_enabled
      ? (activeSubscription?.plan_name ?? null)
      : null,
    used_bytes: capabilities.advanced_traffic_controls_enabled ? (usage?.used_bytes ?? 0) : 0,
    created_at: profile.created_at,
  };
}

function buildUserSessionsSummary(userId: string): AdminUserSessionsSummary {
  const auth = authSessions.filter((item) => item.user_id === userId);
  const playback = playbackSessions.filter((item) => item.user_id === userId);
  return {
    active_auth_sessions: auth.filter((item) => item.is_active).length,
    active_playback_sessions: playback.filter((item) => item.is_active).length,
    last_auth_seen_at: auth.length > 0 ? auth[0].last_seen_at : null,
    last_playback_seen_at: playback.length > 0 ? playback[0].updated_at : null,
  };
}

export async function mockListUserSummaries(
  query: {
    q?: string;
    status?: "all" | "enabled" | "disabled";
    role?: "all" | UserRole;
    page?: number;
    page_size?: number;
    sort_by?: "id" | "email" | "online_devices" | "status" | "subscription" | "role" | "used_bytes";
    sort_dir?: "asc" | "desc";
  } = {}
): Promise<AdminUserSummaryPage> {
  const keyword = query.q?.trim().toLowerCase();
  const status = query.status ?? "all";
  const role = query.role ?? "all";
  const page = Math.max(1, query.page ?? 1);
  const pageSize = Math.max(1, Math.min(200, query.page_size ?? 20));
  const sortBy = query.sort_by ?? "id";
  const sortDesc = (query.sort_dir ?? "desc") === "desc";

  let items = users.map(buildUserSummaryItem);

  if (keyword) {
    items = items.filter((item) => {
      return (
        item.id.toLowerCase().includes(keyword) ||
        item.username.toLowerCase().includes(keyword) ||
        (item.email ?? "").toLowerCase().includes(keyword) ||
        (item.display_name ?? "").toLowerCase().includes(keyword)
      );
    });
  }

  if (status === "enabled") {
    items = items.filter((item) => !item.is_disabled);
  } else if (status === "disabled") {
    items = items.filter((item) => item.is_disabled);
  }

  if (role !== "all") {
    items = items.filter((item) => item.role === role);
  }

  items.sort((left, right) => {
    let result = 0;
    switch (sortBy) {
      case "email":
        result = compareNullableText(left.email, right.email);
        break;
      case "online_devices":
        result = left.active_playback_sessions - right.active_playback_sessions;
        break;
      case "status":
        result = Number(left.is_disabled) - Number(right.is_disabled);
        break;
      case "subscription":
        result = compareNullableText(left.subscription_name, right.subscription_name);
        break;
      case "role":
        result = left.role.localeCompare(right.role, "zh-CN");
        break;
      case "used_bytes":
        result = left.used_bytes - right.used_bytes;
        break;
      default:
        result = left.id.localeCompare(right.id, "zh-CN");
        break;
    }
    return sortDesc ? -result : result;
  });

  const total = items.length;
  const start = (page - 1) * pageSize;

  return {
    page,
    page_size: pageSize,
    total,
    items: clone(items.slice(start, start + pageSize)),
  };
}

export async function mockGetAdminUserProfile(userId: string): Promise<AdminUserManageProfile> {
  const capabilities = currentSystemCapabilities();
  const user = users.find((item) => item.Id === userId);
  if (!user) {
    throw new Error("user not found");
  }

  const profile = userProfiles.get(userId) ?? defaultUserProfile(userId);
  const streamPolicy = capabilities.advanced_traffic_controls_enabled
    ? await mockGetUserStreamPolicy(userId)
    : null;
  const trafficUsage = capabilities.advanced_traffic_controls_enabled
    ? await mockGetUserTrafficUsage(userId)
    : null;
  const wallet = capabilities.billing_enabled ? await mockAdminGetUserWallet(userId) : null;
  const subscriptions = capabilities.billing_enabled
    ? await mockAdminGetUserSubscriptions(userId)
    : null;
  const sessionsSummary = buildUserSessionsSummary(userId);

  return clone({
    user,
    profile,
    stream_policy: streamPolicy,
    traffic_usage: trafficUsage,
    wallet,
    subscriptions,
    sessions_summary: sessionsSummary,
  });
}

export async function mockPatchUserProfile(
  userId: string,
  payload: {
    email?: string | null;
    display_name?: string | null;
    remark?: string | null;
    role?: UserRole;
    is_disabled?: boolean;
  }
): Promise<AdminUserManageProfile> {
  const user = users.find((item) => item.Id === userId);
  if (!user) {
    throw new Error("user not found");
  }

  if (typeof payload.is_disabled === "boolean") {
    user.Policy.IsDisabled = payload.is_disabled;
  }

  if (payload.role) {
    user.Policy.Role = payload.role;
    user.Policy.IsAdministrator = payload.role === "Admin";
  }

  const currentProfile = userProfiles.get(userId) ?? defaultUserProfile(userId);
  const nextProfile: AdminUserProfileRecord = {
    ...currentProfile,
    email:
      payload.email === undefined
        ? currentProfile.email
        : payload.email === null || payload.email.trim() === ""
          ? null
          : payload.email.trim(),
    display_name:
      payload.display_name === undefined
        ? currentProfile.display_name
        : payload.display_name === null || payload.display_name.trim() === ""
          ? null
          : payload.display_name.trim(),
    remark:
      payload.remark === undefined
        ? currentProfile.remark
        : payload.remark === null || payload.remark.trim() === ""
          ? null
          : payload.remark.trim(),
    updated_at: new Date().toISOString(),
  };
  userProfiles.set(userId, nextProfile);

  pushAudit("admin.user.profile.update", "user", userId, payload);
  return await mockGetAdminUserProfile(userId);
}

export async function mockDeleteUser(userId: string): Promise<void> {
  const index = users.findIndex((item) => item.Id === userId);
  if (index < 0) {
    throw new Error("user not found");
  }

  users.splice(index, 1);
  userProfiles.delete(userId);
  streamPolicies.delete(userId);
  trafficUsages.delete(userId);
  adminUserWallets.delete(userId);
  adminUserLedgers.delete(userId);
  adminUserSubscriptions.delete(userId);
  for (let idx = authSessions.length - 1; idx >= 0; idx -= 1) {
    if (authSessions[idx]?.user_id === userId) {
      authSessions.splice(idx, 1);
    }
  }
  for (let idx = playbackSessions.length - 1; idx >= 0; idx -= 1) {
    if (playbackSessions[idx]?.user_id === userId) {
      playbackSessions.splice(idx, 1);
    }
  }

  pushAudit("admin.user.delete", "user", userId, {});
}

export async function mockCreateUser(payload: {
  username: string;
  password: string;
  role: UserRole;
}): Promise<AdminUser> {
  const created: AdminUser = {
    Id: nextId("user"),
    Name: payload.username,
    HasPassword: payload.password.length > 0,
    ServerId: DEMO_SERVER_ID,
    Policy: roleToPolicy(payload.role),
  };

  users.unshift(created);
  userProfiles.set(created.Id, {
    user_id: created.Id,
    email: null,
    display_name: payload.username,
    remark: null,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  });
  pushAudit("admin.user.create", "user", created.Id, { role: payload.role });
  return clone(created);
}

export async function mockSetUserEnabled(userId: string, enabled: boolean): Promise<AdminUser> {
  const user = users.find((item) => item.Id === userId);
  if (!user) {
    throw new Error("user not found");
  }

  user.Policy.IsDisabled = !enabled;
  pushAudit(enabled ? "admin.user.enable" : "admin.user.disable", "user", userId, { enabled });
  return clone(user);
}

export async function mockBatchSetUserEnabled(
  userIds: string[],
  enabled: boolean
): Promise<{ updated: number; users: AdminUser[] }> {
  const affected: AdminUser[] = [];

  users.forEach((user) => {
    if (userIds.includes(user.Id)) {
      user.Policy.IsDisabled = !enabled;
      affected.push(clone(user));
    }
  });

  pushAudit("admin.user.batch_status", "user", null, {
    user_ids: userIds,
    enabled,
    updated: affected.length,
  });
  return {
    updated: affected.length,
    users: affected,
  };
}

export async function mockListLibraries(): Promise<AdminLibrary[]> {
  return clone(
    libraryStatusItems.map((item) => ({
      id: item.id,
      name: item.name,
      root_path: item.root_path,
      paths: item.paths,
      library_type: item.library_type,
      enabled: item.enabled,
      scraper_policy: item.scraper_policy,
      created_at: minutesAgo(60),
    }))
  );
}

export async function mockCreateLibrary(payload: {
  name: string;
  paths: string[];
  library_type: LibraryType;
}): Promise<AdminLibrary> {
  const paths = payload.paths.length > 0 ? payload.paths : ["/media/unknown"];
  const created: AdminLibraryStatusItem = {
    id: nextId("lib"),
    name: payload.name,
    root_path: paths[0],
    paths,
    library_type: payload.library_type,
    enabled: true,
    scraper_policy: {},
    item_count: 0,
    last_item_updated_at: null,
  };

  libraryStatusItems.push(created);
  pushAudit("admin.library.create", "library", created.id, payload);

  return clone({
    id: created.id,
    name: created.name,
    root_path: created.root_path,
    paths: created.paths,
    library_type: created.library_type,
    enabled: created.enabled,
    scraper_policy: created.scraper_policy,
    created_at: new Date().toISOString(),
  });
}

export async function mockListLibraryStatus(): Promise<AdminLibraryStatusResponse> {
  return {
    total: libraryStatusItems.length,
    enabled: libraryStatusItems.filter((item) => item.enabled).length,
    items: clone(libraryStatusItems),
  };
}

export async function mockSetLibraryEnabled(
  libraryId: string,
  enabled: boolean
): Promise<AdminLibrary> {
  const library = libraryStatusItems.find((item) => item.id === libraryId);
  if (!library) {
    throw new Error("library not found");
  }

  library.enabled = enabled;
  pushAudit(enabled ? "admin.library.enable" : "admin.library.disable", "library", libraryId, {
    enabled,
  });

  return clone({
    id: library.id,
    name: library.name,
    root_path: library.root_path,
    paths: library.paths,
    library_type: library.library_type,
    enabled: library.enabled,
    scraper_policy: library.scraper_policy,
    created_at: minutesAgo(20),
  });
}

export async function mockPatchLibrary(
  libraryId: string,
  patch: {
    name?: string;
    library_type?: LibraryType;
    paths?: string[];
    scraper_policy?: Record<string, unknown>;
  }
): Promise<AdminLibrary> {
  const library = libraryStatusItems.find((item) => item.id === libraryId);
  if (!library) {
    throw new Error("library not found");
  }

  if (patch.name) library.name = patch.name;
  if (patch.library_type) library.library_type = patch.library_type;
  if (patch.paths) {
    library.paths = patch.paths;
    library.root_path = patch.paths[0] ?? library.root_path;
  }
  if (patch.scraper_policy) {
    library.scraper_policy = patch.scraper_policy;
  }
  pushAudit("admin.library.update", "library", libraryId, patch);

  return clone({
    id: library.id,
    name: library.name,
    root_path: library.root_path,
    paths: library.paths,
    library_type: library.library_type,
    enabled: library.enabled,
    scraper_policy: library.scraper_policy,
    created_at: minutesAgo(20),
  });
}

export async function mockListTaskDefinitions(): Promise<AdminTaskDefinition[]> {
  return clone(taskDefinitions);
}

export async function mockPatchTaskDefinition(
  taskKey: string,
  patch: {
    enabled?: boolean;
    cron_expr?: string;
    default_payload?: Record<string, unknown>;
    max_attempts?: number;
  }
): Promise<AdminTaskDefinition> {
  const task = taskDefinitions.find((item) => item.task_key === taskKey);
  if (!task) {
    throw new Error("task not found");
  }

  if (typeof patch.enabled === "boolean") {
    task.enabled = patch.enabled;
  }
  if (typeof patch.cron_expr === "string" && patch.cron_expr.trim()) {
    task.cron_expr = patch.cron_expr.trim();
  }
  if (patch.default_payload && typeof patch.default_payload === "object") {
    task.default_payload = clone(patch.default_payload);
  }
  if (typeof patch.max_attempts === "number" && patch.max_attempts > 0) {
    task.max_attempts = Math.floor(patch.max_attempts);
  }
  task.updated_at = new Date().toISOString();

  pushAudit("admin.task_center.task.update", "task_definition", taskKey, patch);
  return clone(task);
}

export async function mockRunTaskNow(
  taskKey: string,
  payloadOverride?: Record<string, unknown>
): Promise<AdminTaskRun> {
  const task = taskDefinitions.find((item) => item.task_key === taskKey);
  if (!task) {
    throw new Error("task not found");
  }

  const payload = {
    ...(task.default_payload || {}),
    ...(payloadOverride || {}),
  };

  const created: AdminTaskRun = {
    id: nextId("run"),
    kind: task.task_key,
    status: "queued",
    payload,
    progress: {
      phase: "queued",
      total: 0,
      completed: 0,
      percent: 0,
      message: "任务已排队",
    },
    result: null,
    error: null,
    attempts: 0,
    max_attempts: task.max_attempts,
    next_retry_at: null,
    dead_letter: false,
    trigger_type: "manual",
    scheduled_for: null,
    created_at: new Date().toISOString(),
    started_at: null,
    finished_at: null,
  };

  taskRuns.unshift(created);
  pushAudit("admin.task_center.run", "task_run", created.id, {
    task_key: taskKey,
    payload_override: payloadOverride || {},
  });

  return clone(created);
}

export async function mockListTaskRuns(options?: {
  limit?: number;
  task_key?: string;
  status?: string;
  trigger_type?: string;
  exclude_kinds?: string;
}): Promise<AdminTaskRun[]> {
  const limit = options?.limit ?? 50;
  const excludeKinds = (options?.exclude_kinds || "")
    .split(",")
    .map((value) => value.trim())
    .filter((value) => value.length > 0);
  const filtered = taskRuns.filter((run) => {
    if (options?.task_key && run.kind !== options.task_key) {
      return false;
    }
    if (options?.status && run.status !== options.status) {
      return false;
    }
    if (options?.trigger_type && (run.trigger_type || "manual") !== options.trigger_type) {
      return false;
    }
    if (excludeKinds.includes(run.kind)) {
      return false;
    }
    return true;
  });
  return clone(filtered.slice(0, limit));
}

export async function mockGetTaskRun(runId: string): Promise<AdminTaskRun> {
  const run = taskRuns.find((item) => item.id === runId);
  if (!run) {
    throw new Error("task run not found");
  }
  return clone(run);
}

export async function mockCancelTaskRun(runId: string): Promise<AdminTaskRun> {
  const run = taskRuns.find((item) => item.id === runId);
  if (!run) {
    throw new Error("task run not found");
  }

  if (run.status === "queued" || run.status === "pending") {
    run.status = "cancelled";
    run.finished_at = new Date().toISOString();
    run.progress = {
      phase: "cancelled",
      total: 1,
      completed: 1,
      percent: 100,
      message: "任务已取消（排队中）",
    };
  } else if (run.status === "running") {
    run.progress = {
      phase: "cancelling",
      total: 1,
      completed: 0,
      percent: run.progress?.percent || 0,
      message: "正在取消任务",
    };
  }

  return clone(run);
}

export async function mockListPlaybackSessions(
  limit = 80,
  activeOnly = false
): Promise<PlaybackSession[]> {
  const rows = activeOnly ? playbackSessions.filter((item) => item.is_active) : playbackSessions;
  return clone(rows.slice(0, limit));
}

export async function mockListAuthSessions(limit = 80, activeOnly = false): Promise<AuthSession[]> {
  const rows = activeOnly ? authSessions.filter((item) => item.is_active) : authSessions;
  return clone(rows.slice(0, limit));
}

export async function mockListApiKeys(limit = 100): Promise<AdminApiKey[]> {
  return clone(apiKeys.slice(0, limit));
}

export async function mockCreateApiKey(name: string): Promise<AdminCreatedApiKey> {
  const created: AdminCreatedApiKey = {
    id: nextId("key"),
    name,
    api_key: `lsadm_mock_${nextId("token")}`,
    created_at: new Date().toISOString(),
  };

  apiKeys.unshift({
    id: created.id,
    name: created.name,
    created_at: created.created_at,
    last_used_at: null,
  });

  pushAudit("admin.api_key.create", "admin_api_key", created.id, { name });
  return clone(created);
}

export async function mockDeleteApiKey(keyId: string): Promise<void> {
  const keyIndex = apiKeys.findIndex((item) => item.id === keyId);
  if (keyIndex < 0) {
    throw new Error("api key not found");
  }

  apiKeys.splice(keyIndex, 1);
  pushAudit("admin.api_key.delete", "admin_api_key", keyId, {});
}

export async function mockGetSettings(): Promise<WebAppSettings> {
  return clone(settings);
}

function mockSettingsRestartRequired(prev: WebAppSettings, next: WebAppSettings): boolean {
  return (
    prev.server.host !== next.server.host ||
    prev.server.port !== next.server.port ||
    JSON.stringify(prev.server.cors_allow_origins) !==
      JSON.stringify(next.server.cors_allow_origins)
  );
}

export async function mockUpsertSettings(
  nextSettings: WebAppSettings
): Promise<AdminUpsertSettingsResponse> {
  const restart_required = mockSettingsRestartRequired(settings, nextSettings);
  settings = clone(nextSettings);
  pushAudit("admin.settings.upsert", "web_settings", "global", { restart_required });

  return {
    settings: clone(settings),
    restart_required,
  };
}

export async function mockAdminGetInviteSettings(): Promise<AdminInviteSettings> {
  const capabilities = currentSystemCapabilities();
  return capabilities.invite_rewards_enabled
    ? clone(settings.auth.invite)
    : { force_on_register: settings.auth.invite.force_on_register };
}

export async function mockAdminUpsertInviteSettings(payload: {
  force_on_register?: boolean;
  invitee_bonus_enabled?: boolean;
  invitee_bonus_amount?: string;
  inviter_rebate_enabled?: boolean;
  inviter_rebate_rate?: string;
}): Promise<AdminInviteSettings> {
  const capabilities = currentSystemCapabilities();
  const next = {
    ...settings.auth.invite,
  };
  if (typeof payload.force_on_register === "boolean") {
    next.force_on_register = payload.force_on_register;
  }
  if (capabilities.invite_rewards_enabled) {
    if (typeof payload.invitee_bonus_enabled === "boolean") {
      next.invitee_bonus_enabled = payload.invitee_bonus_enabled;
    }
    if (typeof payload.invitee_bonus_amount === "string") {
      const parsed = Number.parseFloat(payload.invitee_bonus_amount);
      const safe = Number.isFinite(parsed) ? Math.max(0, parsed) : 0;
      next.invitee_bonus_amount = safe.toFixed(2);
    }
    if (typeof payload.inviter_rebate_enabled === "boolean") {
      next.inviter_rebate_enabled = payload.inviter_rebate_enabled;
    }
    if (typeof payload.inviter_rebate_rate === "string") {
      const parsed = Number.parseFloat(payload.inviter_rebate_rate);
      const safe = Number.isFinite(parsed) ? Math.max(0, Math.min(1, parsed)) : 0;
      next.inviter_rebate_rate = safe.toFixed(4);
    }
  } else {
    next.invitee_bonus_enabled = false;
    next.invitee_bonus_amount = "0.00";
    next.inviter_rebate_enabled = false;
    next.inviter_rebate_rate = "0.0000";
  }
  settings = {
    ...settings,
    auth: {
      ...settings.auth,
      invite: next,
    },
  };
  pushAudit("admin.invite.settings.upsert", "web_settings", "global", {
    ...next,
  });
  return capabilities.invite_rewards_enabled
    ? clone(next)
    : { force_on_register: next.force_on_register };
}

export async function mockAdminListInviteRelations(limit = 100): Promise<InviteRelation[]> {
  const safeLimit = Math.max(1, Math.min(500, Math.floor(limit)));
  return clone(inviteRelations.slice(0, safeLimit));
}

export async function mockAdminListInviteRebates(limit = 100): Promise<InviteRebateRecord[]> {
  const safeLimit = Math.max(1, Math.min(500, Math.floor(limit)));
  return clone(inviteRebates.slice(0, safeLimit));
}

export async function mockListAuditLogs(limit = 200): Promise<AuditLogEntry[]> {
  return clone(auditLogs.slice(0, limit));
}

function escapeCsv(value: string): string {
  if (value.includes(",") || value.includes("\n") || value.includes('"')) {
    return `"${value.replaceAll('"', '""')}"`;
  }

  return value;
}

export function mockBuildAuditCsv(limit = 1000): string {
  const rows = auditLogs.slice(0, Math.max(1, limit));
  const lines = ["created_at,actor_username,action,target_type,target_id,detail"];

  rows.forEach((row) => {
    lines.push(
      [
        row.created_at,
        row.actor_username || "system",
        row.action,
        row.target_type,
        row.target_id || "",
        JSON.stringify(row.detail),
      ]
        .map(escapeCsv)
        .join(",")
    );
  });

  return `${lines.join("\n")}\n`;
}

export async function mockListStorageConfigs(): Promise<Record<string, unknown>[]> {
  return clone(storageConfigs);
}

export async function mockUpsertStorageConfig(payload: {
  kind: string;
  name: string;
  config: Record<string, unknown>;
  enabled?: boolean;
}): Promise<Record<string, unknown>> {
  const enabled = payload.enabled ?? true;

  const found = storageConfigs.find(
    (item) => item.kind === payload.kind && item.name === payload.name
  ) as (Record<string, unknown> & { id?: string }) | undefined;

  if (found) {
    found.config = payload.config;
    found.enabled = enabled;
    pushAudit("admin.storage_config.upsert", "storage_config", String(found.id || ""), payload);
    return clone(found);
  }

  const created = {
    id: nextId("storage"),
    kind: payload.kind,
    name: payload.name,
    enabled,
    config: payload.config,
  };

  storageConfigs.push(created);
  pushAudit("admin.storage_config.upsert", "storage_config", created.id, payload);
  return clone(created);
}

export async function mockListPlaybackDomains(): Promise<PlaybackDomain[]> {
  return clone(playbackDomains);
}

export async function mockUpsertPlaybackDomain(payload: {
  id?: string;
  name: string;
  base_url: string;
  enabled?: boolean;
  priority?: number;
  is_default?: boolean;
  lumenbackend_node_id?: string | null;
  traffic_multiplier?: number;
}): Promise<PlaybackDomain> {
  const enabled = payload.enabled ?? true;
  const priority = payload.priority ?? 0;
  const isDefault = payload.is_default ?? false;
  const lumenbackendNodeId = payload.lumenbackend_node_id?.trim() || null;
  const trafficMultiplier = Math.max(0.01, payload.traffic_multiplier ?? 1);

  if (isDefault) {
    playbackDomains.forEach((item) => {
      item.is_default = false;
      item.updated_at = new Date().toISOString();
    });
  }

  const now = new Date().toISOString();
  if (payload.id) {
    const current = playbackDomains.find((item) => item.id === payload.id);
    if (!current) {
      throw new Error("playback domain not found");
    }
    current.name = payload.name;
    current.base_url = payload.base_url;
    current.enabled = enabled;
    current.priority = priority;
    current.is_default = isDefault;
    current.lumenbackend_node_id = lumenbackendNodeId;
    current.traffic_multiplier = trafficMultiplier;
    current.updated_at = now;
    pushAudit("admin.playback_domain.upsert", "playback_domain", current.id, payload);
    return clone(current);
  }

  const created: PlaybackDomain = {
    id: nextId("domain"),
    name: payload.name,
    base_url: payload.base_url,
    enabled,
    priority,
    is_default: isDefault || playbackDomains.length === 0,
    lumenbackend_node_id: lumenbackendNodeId,
    traffic_multiplier: trafficMultiplier,
    created_at: now,
    updated_at: now,
  };
  if (created.is_default) {
    playbackDomains.forEach((item) => {
      item.is_default = false;
      item.updated_at = now;
    });
  }
  playbackDomains.push(created);
  pushAudit("admin.playback_domain.upsert", "playback_domain", created.id, payload);
  return clone(created);
}

export async function mockDeletePlaybackDomain(domainId: string): Promise<{ deleted: boolean }> {
  const idx = playbackDomains.findIndex((item) => item.id === domainId);
  if (idx === -1) {
    throw new Error("playback domain not found");
  }
  const wasDefault = playbackDomains[idx]!.is_default;
  playbackDomains.splice(idx, 1);
  if (wasDefault && playbackDomains.length > 0) {
    playbackDomains[0]!.is_default = true;
    playbackDomains[0]!.updated_at = new Date().toISOString();
  }
  pushAudit("admin.playback_domain.delete", "playback_domain", domainId, { deleted: true });
  return { deleted: true };
}

export async function mockListLumenBackendNodes(): Promise<LumenBackendNode[]> {
  return clone(lumenbackendNodes);
}

function mergeJsonObjects(
  base: Record<string, unknown>,
  overlay: Record<string, unknown>
): Record<string, unknown> {
  const merged: Record<string, unknown> = clone(base);
  Object.entries(overlay).forEach(([key, value]) => {
    const current = merged[key];
    if (
      current &&
      typeof current === "object" &&
      !Array.isArray(current) &&
      value &&
      typeof value === "object" &&
      !Array.isArray(value)
    ) {
      merged[key] = mergeJsonObjects(
        current as Record<string, unknown>,
        value as Record<string, unknown>
      );
      return;
    }
    merged[key] = clone(value);
  });
  return merged;
}

function setConfigValueByPath(target: Record<string, unknown>, key: string, value: unknown): void {
  const parts = key
    .split(".")
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
  if (parts.length === 0) {
    return;
  }

  let current: Record<string, unknown> = target;
  for (let idx = 0; idx < parts.length; idx += 1) {
    const part = parts[idx]!;
    const isLast = idx + 1 === parts.length;
    if (isLast) {
      current[part] = value;
      return;
    }
    const next = current[part];
    if (!next || typeof next !== "object" || Array.isArray(next)) {
      current[part] = {};
    }
    current = current[part] as Record<string, unknown>;
  }
}

function resolveNodeSchema(nodeId: string): LumenBackendNodeRuntimeSchema {
  const schema = lumenbackendNodeSchemas.get(nodeId);
  if (!schema) {
    throw new Error("runtime schema not reported");
  }
  return schema;
}

function buildConfigDefaultsFromSchema(
  schema: LumenBackendNodeRuntimeSchema
): Record<string, unknown> {
  const base: Record<string, unknown> = {};
  for (const section of schema.schema.sections || []) {
    for (const field of section.fields || []) {
      if (field.default !== undefined) {
        setConfigValueByPath(base, field.key, clone(field.default));
      }
    }
  }
  return base;
}

function mergeSecretPlaceholdersInConfig(incoming: unknown, current: unknown): unknown {
  if (typeof incoming === "string" && incoming.trim() === "***") {
    return clone(current);
  }
  if (
    incoming &&
    typeof incoming === "object" &&
    !Array.isArray(incoming) &&
    current &&
    typeof current === "object" &&
    !Array.isArray(current)
  ) {
    const incomingObj = incoming as Record<string, unknown>;
    const currentObj = current as Record<string, unknown>;
    const merged: Record<string, unknown> = { ...incomingObj };
    Object.entries(incomingObj).forEach(([key, value]) => {
      if (Object.prototype.hasOwnProperty.call(currentObj, key)) {
        merged[key] = mergeSecretPlaceholdersInConfig(value, currentObj[key]);
      }
    });
    return merged;
  }
  if (Array.isArray(incoming) && Array.isArray(current)) {
    return incoming.map((value, index) => mergeSecretPlaceholdersInConfig(value, current[index]));
  }
  return clone(incoming);
}

function maskNodeConfigSecrets(value: unknown): unknown {
  if (!value || typeof value !== "object") {
    return value;
  }
  if (Array.isArray(value)) {
    return value.map((item) => maskNodeConfigSecrets(item));
  }

  const sensitiveNeedles = [
    "secret",
    "password",
    "token",
    "access_key",
    "secret_key",
    "api_key",
    "dsn",
  ];
  const out: Record<string, unknown> = {};
  Object.entries(value as Record<string, unknown>).forEach(([key, raw]) => {
    const sensitive = sensitiveNeedles.some((needle) => key.toLowerCase().includes(needle));
    if (sensitive && typeof raw === "string") {
      out[key] = "***";
      return;
    }
    out[key] = maskNodeConfigSecrets(raw);
  });
  return out;
}

function appendManagedRuntimeFields(config: Record<string, unknown>): Record<string, unknown> {
  const storage = settings.storage as {
    lumenbackend_stream_signing_key?: string;
    lumenbackend_stream_token_ttl_seconds?: number;
    lumenbackend_route?: string;
  };
  const route = (storage.lumenbackend_route || "v1/streams/gdrive").trim() || "v1/streams/gdrive";
  const signingKey = (storage.lumenbackend_stream_signing_key || "").trim();
  const ttl = Number(storage.lumenbackend_stream_token_ttl_seconds || 86_400);

  return {
    ...config,
    stream_route: route,
    stream_token: {
      enabled: signingKey.length > 0,
      signing_key: signingKey,
      max_age_sec: Number.isFinite(ttl) && ttl > 0 ? Math.floor(ttl) : 86_400,
    },
    playback_domains: clone(playbackDomains.filter((domain) => domain.enabled)),
  };
}

export async function mockCreateLumenBackendNode(payload: {
  node_id: string;
  node_name?: string;
  enabled?: boolean;
}): Promise<LumenBackendNode> {
  const nodeId = payload.node_id.trim();
  if (!nodeId) {
    throw new Error("node_id is required");
  }
  if (lumenbackendNodes.some((item) => item.node_id === nodeId)) {
    throw new Error("lumenbackend node already exists");
  }

  const now = new Date().toISOString();
  const created: LumenBackendNode = {
    node_id: nodeId,
    name: payload.node_name?.trim() || null,
    enabled: payload.enabled ?? true,
    last_seen_at: null,
    last_version: null,
    last_status: {},
    created_at: now,
    updated_at: now,
  };
  lumenbackendNodes.unshift(created);
  pushAudit("admin.lumenbackend.node.create", "lumenbackend_node", nodeId, {
    node_id: nodeId,
    enabled: created.enabled,
  });
  return clone(created);
}

export async function mockPatchLumenBackendNode(
  nodeId: string,
  payload: {
    node_name?: string | null;
    enabled?: boolean;
  }
): Promise<LumenBackendNode> {
  const found = lumenbackendNodes.find((item) => item.node_id === nodeId);
  if (!found) {
    throw new Error("lumenbackend node not found");
  }

  if (payload.node_name !== undefined) {
    const name = payload.node_name?.trim() || "";
    found.name = name.length > 0 ? name : null;
  }
  if (typeof payload.enabled === "boolean") {
    found.enabled = payload.enabled;
  }
  found.updated_at = new Date().toISOString();
  pushAudit(
    "admin.lumenbackend.node.update",
    "lumenbackend_node",
    nodeId,
    payload as Record<string, unknown>
  );
  return clone(found);
}

export async function mockDeleteLumenBackendNode(nodeId: string): Promise<{ deleted: boolean }> {
  const foundIndex = lumenbackendNodes.findIndex((item) => item.node_id === nodeId);
  if (foundIndex < 0) {
    throw new Error("lumenbackend node not found");
  }
  if (playbackDomains.some((item) => item.lumenbackend_node_id === nodeId)) {
    throw new Error("lumenbackend node is still bound to playback domains");
  }

  lumenbackendNodes.splice(foundIndex, 1);
  lumenbackendNodeConfigs.delete(nodeId);
  lumenbackendNodeSchemas.delete(nodeId);
  pushAudit("admin.lumenbackend.node.delete", "lumenbackend_node", nodeId, { deleted: true });
  return { deleted: true };
}

export async function mockGetLumenBackendNodeSchema(
  nodeId: string
): Promise<LumenBackendNodeRuntimeSchema> {
  const exists = lumenbackendNodes.some((item) => item.node_id === nodeId);
  if (!exists) {
    throw new Error("lumenbackend node not found");
  }
  const schema = lumenbackendNodeSchemas.get(nodeId);
  if (!schema) {
    throw new Error("runtime schema not reported");
  }
  return clone(schema);
}

export async function mockGetLumenBackendNodeConfig(
  nodeId: string,
  includeSecrets = false
): Promise<LumenBackendNodeRuntimeConfig> {
  const exists = lumenbackendNodes.some((item) => item.node_id === nodeId);
  if (!exists) {
    throw new Error("lumenbackend node not found");
  }
  const found = lumenbackendNodeConfigs.get(nodeId);
  const schema = resolveNodeSchema(nodeId);
  const baseConfig = buildConfigDefaultsFromSchema(schema);
  const mergedConfig = found
    ? mergeJsonObjects(baseConfig, found.config as Record<string, unknown>)
    : baseConfig;
  const withManaged = appendManagedRuntimeFields(mergedConfig);

  const outputConfig = includeSecrets
    ? withManaged
    : (maskNodeConfigSecrets(withManaged) as Record<string, unknown>);

  if (found) {
    return clone({
      ...found,
      config: outputConfig,
    });
  }

  return clone({
    node_id: nodeId,
    version: 0,
    config: outputConfig,
  });
}

export async function mockUpsertLumenBackendNodeConfig(
  nodeId: string,
  config: Record<string, unknown>
): Promise<LumenBackendNodeRuntimeConfig> {
  const exists = lumenbackendNodes.some((item) => item.node_id === nodeId);
  if (!exists) {
    throw new Error("lumenbackend node not found");
  }
  resolveNodeSchema(nodeId);

  const current = lumenbackendNodeConfigs.get(nodeId);
  const mergedConfig = current
    ? (mergeSecretPlaceholdersInConfig(config, current.config) as Record<string, unknown>)
    : clone(config);
  const next: LumenBackendNodeRuntimeConfig = {
    node_id: nodeId,
    version: (current?.version || 0) + 1,
    config: mergedConfig,
  };
  lumenbackendNodeConfigs.set(nodeId, next);
  const node = lumenbackendNodes.find((item) => item.node_id === nodeId);
  if (node) {
    node.updated_at = new Date().toISOString();
  }
  pushAudit("admin.lumenbackend.node_config.upsert", "lumenbackend_node", nodeId, {
    node_id: nodeId,
    version: next.version,
  });
  return mockGetLumenBackendNodeConfig(nodeId, false);
}

export interface MockCacheOperationResult {
  success: boolean;
  message: string;
}

export async function mockClearStorageCache(): Promise<MockCacheOperationResult> {
  pushAudit("admin.storage.cache.cleanup", "storage_cache", null, {});
  return {
    success: true,
    message: "缓存已清除",
  };
}

export async function mockInvalidateStorageCache(): Promise<MockCacheOperationResult> {
  pushAudit("admin.storage.cache.invalidate", "storage_cache", null, {});
  return {
    success: true,
    message: "缓存已失效",
  };
}

const streamPolicies: Map<string, StreamPolicy> = new Map([
  [
    "user-admin-001",
    {
      user_id: "user-admin-001",
      expires_at: null,
      max_concurrent_streams: 5,
      traffic_quota_bytes: null,
      traffic_window_days: 30,
      updated_at: minutesAgo(60),
    },
  ],
  [
    "user-operator-001",
    {
      user_id: "user-operator-001",
      expires_at: null,
      max_concurrent_streams: 3,
      traffic_quota_bytes: 100 * 1024 * 1024 * 1024,
      traffic_window_days: 30,
      updated_at: minutesAgo(120),
    },
  ],
  [
    "user-viewer-001",
    {
      user_id: "user-viewer-001",
      expires_at: null,
      max_concurrent_streams: 1,
      traffic_quota_bytes: 10 * 1024 * 1024 * 1024,
      traffic_window_days: 30,
      updated_at: minutesAgo(180),
    },
  ],
]);

const trafficUsages: Map<string, TrafficUsage> = new Map([
  [
    "user-admin-001",
    {
      user_id: "user-admin-001",
      window_days: 30,
      used_bytes: 107_374_182_400,
      real_used_bytes: 107_374_182_400,
      quota_bytes: null,
      remaining_bytes: null,
      daily: [
        {
          usage_date: "2026-02-14",
          bytes_served: 20_000_000_000,
          real_bytes_served: 20_000_000_000,
        },
        {
          usage_date: "2026-02-15",
          bytes_served: 21_000_000_000,
          real_bytes_served: 21_000_000_000,
        },
        { usage_date: "2026-02-16", bytes_served: 5_368_709_120, real_bytes_served: 5_368_709_120 },
      ],
    },
  ],
  [
    "user-operator-001",
    {
      user_id: "user-operator-001",
      window_days: 30,
      used_bytes: 53_687_091_200,
      real_used_bytes: 41_297_762_462,
      quota_bytes: 100 * 1024 * 1024 * 1024,
      remaining_bytes: 46_312_908_800,
      daily: [
        {
          usage_date: "2026-02-14",
          bytes_served: 10_000_000_000,
          real_bytes_served: 7_692_307_692,
        },
        {
          usage_date: "2026-02-15",
          bytes_served: 11_000_000_000,
          real_bytes_served: 8_461_538_462,
        },
        { usage_date: "2026-02-16", bytes_served: 2_147_483_648, real_bytes_served: 1_651_910_498 },
      ],
    },
  ],
  [
    "user-viewer-001",
    {
      user_id: "user-viewer-001",
      window_days: 30,
      used_bytes: 1_073_741_824,
      real_used_bytes: 1_073_741_824,
      quota_bytes: 10 * 1024 * 1024 * 1024,
      remaining_bytes: 8_926_258_176,
      daily: [{ usage_date: "2026-02-16", bytes_served: 0, real_bytes_served: 0 }],
    },
  ],
]);

const trafficUsageMediaItems: Map<string, TrafficUsageMediaItem[]> = new Map([
  [
    "user-admin-001",
    [
      {
        media_item_id: "movie-001",
        item_name: "星际穿越",
        item_type: "Movie",
        bytes_served: 48_318_382_080,
        real_bytes_served: 48_318_382_080,
        usage_days: 4,
        last_usage_date: "2026-02-23",
      },
      {
        media_item_id: "series-001",
        item_name: "三体",
        item_type: "Series",
        bytes_served: 36_507_222_016,
        real_bytes_served: 36_507_222_016,
        usage_days: 6,
        last_usage_date: "2026-02-24",
      },
      {
        media_item_id: "movie-003",
        item_name: "沙丘",
        item_type: "Movie",
        bytes_served: 18_112_798_304,
        real_bytes_served: 18_112_798_304,
        usage_days: 2,
        last_usage_date: "2026-02-20",
      },
    ],
  ],
  [
    "user-operator-001",
    [
      {
        media_item_id: "movie-002",
        item_name: "盗梦空间",
        item_type: "Movie",
        bytes_served: 23_118_364_672,
        real_bytes_served: 17_783_357_440,
        usage_days: 3,
        last_usage_date: "2026-02-22",
      },
      {
        media_item_id: "series-001",
        item_name: "三体",
        item_type: "Series",
        bytes_served: 18_314_485_760,
        real_bytes_served: 14_088_065_969,
        usage_days: 4,
        last_usage_date: "2026-02-24",
      },
      {
        media_item_id: "movie-004",
        item_name: "银翼杀手 2049",
        item_type: "Movie",
        bytes_served: 7_650_484_224,
        real_bytes_served: 5_884_987_865,
        usage_days: 1,
        last_usage_date: "2026-02-17",
      },
    ],
  ],
  [
    "user-viewer-001",
    [
      {
        media_item_id: "movie-005",
        item_name: "蜘蛛侠：平行宇宙",
        item_type: "Movie",
        bytes_served: 1_073_741_824,
        real_bytes_served: 1_073_741_824,
        usage_days: 1,
        last_usage_date: "2026-02-16",
      },
    ],
  ],
]);

function buildMyTrafficUsageMediaSummary(userId: string, limit = 200): MyTrafficUsageMediaSummary {
  const summary = trafficUsages.get(userId) || {
    user_id: userId,
    window_days: 30,
    used_bytes: 0,
    real_used_bytes: 0,
    quota_bytes: null,
    remaining_bytes: null,
    daily: [],
  };

  const safeLimit = Math.max(1, Math.min(500, limit));
  const items = (trafficUsageMediaItems.get(userId) || []).slice(0, safeLimit);
  const classified = items.reduce((sum, item) => sum + Math.max(0, item.bytes_served), 0);
  const classifiedReal = items.reduce((sum, item) => sum + Math.max(0, item.real_bytes_served), 0);
  const totalRealUsed = summary.real_used_bytes ?? summary.used_bytes;

  return {
    user_id: summary.user_id,
    window_days: summary.window_days,
    used_bytes: summary.used_bytes,
    real_used_bytes: totalRealUsed,
    quota_bytes: summary.quota_bytes,
    remaining_bytes: summary.remaining_bytes,
    unclassified_bytes: Math.max(0, summary.used_bytes - classified),
    unclassified_real_bytes: Math.max(0, totalRealUsed - classifiedReal),
    items: clone(items),
  };
}

export async function mockGetUserStreamPolicy(userId: string): Promise<StreamPolicy> {
  const policy = streamPolicies.get(userId);
  if (policy) {
    return clone(policy);
  }

  return {
    user_id: userId,
    expires_at: null,
    max_concurrent_streams: 2,
    traffic_quota_bytes: null,
    traffic_window_days: 30,
    updated_at: new Date().toISOString(),
  };
}

export async function mockSetUserStreamPolicy(
  userId: string,
  updates: Partial<Omit<StreamPolicy, "user_id" | "updated_at">>
): Promise<StreamPolicy> {
  const existing = streamPolicies.get(userId) || {
    user_id: userId,
    expires_at: null,
    max_concurrent_streams: 2,
    traffic_quota_bytes: null,
    traffic_window_days: 30,
    updated_at: new Date().toISOString(),
  };

  const updated: StreamPolicy = {
    ...existing,
    ...updates,
    user_id: userId,
    updated_at: new Date().toISOString(),
  };

  streamPolicies.set(userId, updated);
  pushAudit("admin.user.stream_policy.update", "stream_policy", userId, updates);
  return clone(updated);
}

export async function mockGetUserTrafficUsage(userId: string): Promise<TrafficUsage> {
  const usage = trafficUsages.get(userId);
  if (usage) {
    return clone(usage);
  }

  return {
    user_id: userId,
    window_days: 30,
    used_bytes: 0,
    quota_bytes: null,
    remaining_bytes: null,
    daily: [],
  };
}

export async function mockResetUserTrafficUsage(userId: string): Promise<TrafficUsage> {
  const reset: TrafficUsage = {
    user_id: userId,
    window_days: 30,
    used_bytes: 0,
    quota_bytes: null,
    remaining_bytes: null,
    daily: [],
  };

  trafficUsages.set(userId, reset);
  pushAudit("admin.user.traffic_usage.reset", "traffic_usage", userId, {});
  return clone(reset);
}

export async function mockGetTopTrafficUsers(limit = 20): Promise<TopTrafficUser[]> {
  const result: TopTrafficUser[] = [];

  trafficUsages.forEach((usage) => {
    result.push({
      user_id: usage.user_id,
      username: users.find((item) => item.Id === usage.user_id)?.Name || "unknown",
      used_bytes: usage.used_bytes,
    });
  });

  return clone(result.sort((a, b) => b.used_bytes - a.used_bytes).slice(0, limit));
}

export async function mockGetMyTrafficUsageByMedia(
  limit = 200
): Promise<MyTrafficUsageMediaSummary> {
  const userId = mockCurrentDemoUser().Id;
  return buildMyTrafficUsageMediaSummary(userId, limit);
}

// Admin Billing Mocks
import type {
  AdjustBalanceRequest,
  AdjustBalanceResult,
  BillingPermissionGroup,
  CreatePlanRequest,
  LedgerEntry,
  Plan,
  RechargeOrder,
  Subscription,
  UpsertBillingPermissionGroupRequest,
  UpdatePlanRequest,
  Wallet,
} from "@/lib/types/billing";

const adminPlans: Plan[] = [
  {
    id: "plan-basic",
    code: "basic",
    name: "基础套餐",
    price: "15.00",
    duration_days: 30,
    traffic_quota_bytes: 107374182400, // 100GB
    traffic_window_days: 30,
    permission_group_id: null,
    permission_group_name: null,
    enabled: true,
    updated_at: minutesAgo(60),
  },
  {
    id: "plan-standard",
    code: "standard",
    name: "标准套餐",
    price: "29.00",
    duration_days: 30,
    traffic_quota_bytes: 322122547200, // 300GB
    traffic_window_days: 30,
    permission_group_id: null,
    permission_group_name: null,
    enabled: true,
    updated_at: minutesAgo(60),
  },
  {
    id: "plan-premium",
    code: "premium",
    name: "高级套餐",
    price: "59.00",
    duration_days: 30,
    traffic_quota_bytes: 858993459200, // 800GB
    traffic_window_days: 30,
    permission_group_id: null,
    permission_group_name: null,
    enabled: true,
    updated_at: minutesAgo(60),
  },
  {
    id: "plan-inactive",
    code: "inactive",
    name: "已下架套餐",
    price: "9.99",
    duration_days: 30,
    traffic_quota_bytes: 53687091200, // 50GB
    traffic_window_days: 30,
    permission_group_id: null,
    permission_group_name: null,
    enabled: false,
    updated_at: minutesAgo(120),
  },
];

const adminPermissionGroups: BillingPermissionGroup[] = [
  {
    id: "perm-group-mainland",
    code: "mainland",
    name: "大陆线路组",
    enabled: true,
    domain_ids: ["domain-001"],
    updated_at: minutesAgo(90),
  },
  {
    id: "perm-group-global",
    code: "global",
    name: "全球线路组",
    enabled: true,
    domain_ids: ["domain-001", "domain-002", "domain-003"],
    updated_at: minutesAgo(40),
  },
];

const adminRechargeOrders: RechargeOrder[] = [
  {
    id: "order-001",
    user_id: "user-admin-001",
    out_trade_no: "trade-001",
    channel: "alipay",
    amount: "100.00",
    status: "paid",
    subject: "账户充值",
    provider_trade_no: "alipay-001",
    paid_at: minutesAgo(118),
    expires_at: minutesAgo(90),
    created_at: minutesAgo(120),
    updated_at: minutesAgo(118),
  },
  {
    id: "order-002",
    user_id: "user-operator-001",
    out_trade_no: "trade-002",
    channel: "wxpay",
    amount: "50.00",
    status: "paid",
    subject: "账户充值",
    provider_trade_no: "wxpay-002",
    paid_at: minutesAgo(58),
    expires_at: minutesAgo(30),
    created_at: minutesAgo(60),
    updated_at: minutesAgo(58),
  },
  {
    id: "order-003",
    user_id: "user-viewer-001",
    out_trade_no: "trade-003",
    channel: "alipay",
    amount: "20.00",
    status: "pending",
    subject: "账户充值",
    provider_trade_no: null,
    paid_at: null,
    expires_at: new Date(Date.now() + 20 * 60 * 1000).toISOString(),
    created_at: minutesAgo(10),
    updated_at: minutesAgo(10),
  },
];

const adminUserWallets: Map<string, Wallet> = new Map([
  [
    "user-admin-001",
    {
      user_id: "user-admin-001",
      balance: "128.50",
      total_recharged: "200.00",
      total_spent: "71.50",
      updated_at: minutesAgo(5),
    },
  ],
  [
    "user-operator-001",
    {
      user_id: "user-operator-001",
      balance: "42.00",
      total_recharged: "50.00",
      total_spent: "8.00",
      updated_at: minutesAgo(30),
    },
  ],
  [
    "user-viewer-001",
    {
      user_id: "user-viewer-001",
      balance: "0.00",
      total_recharged: "0.00",
      total_spent: "0.00",
      updated_at: minutesAgo(60),
    },
  ],
]);

const adminUserLedgers: Map<string, LedgerEntry[]> = new Map([
  [
    "user-admin-001",
    [
      {
        id: "ledger-001",
        user_id: "user-admin-001",
        entry_type: "recharge",
        amount: "100.00",
        balance_after: "100.00",
        reference_type: "recharge_order",
        reference_id: "order-001",
        note: "充值 ¥100",
        meta: {},
        created_at: minutesAgo(120),
      },
      {
        id: "ledger-002",
        user_id: "user-admin-001",
        entry_type: "purchase",
        amount: "-29.00",
        balance_after: "71.00",
        reference_type: "subscription",
        reference_id: "sub-001",
        note: "购买标准套餐",
        meta: {},
        created_at: minutesAgo(100),
      },
      {
        id: "ledger-003",
        user_id: "user-admin-001",
        entry_type: "recharge",
        amount: "50.00",
        balance_after: "121.00",
        reference_type: "recharge_order",
        reference_id: "order-004",
        note: "充值 ¥50",
        meta: {},
        created_at: minutesAgo(50),
      },
      {
        id: "ledger-004",
        user_id: "user-admin-001",
        entry_type: "adjustment",
        amount: "7.50",
        balance_after: "128.50",
        reference_type: null,
        reference_id: null,
        note: "管理员调整：补偿",
        meta: {},
        created_at: minutesAgo(5),
      },
    ],
  ],
  [
    "user-operator-001",
    [
      {
        id: "ledger-010",
        user_id: "user-operator-001",
        entry_type: "recharge",
        amount: "50.00",
        balance_after: "50.00",
        reference_type: "recharge_order",
        reference_id: "order-002",
        note: "充值 ¥50",
        meta: {},
        created_at: minutesAgo(60),
      },
      {
        id: "ledger-011",
        user_id: "user-operator-001",
        entry_type: "purchase",
        amount: "-8.00",
        balance_after: "42.00",
        reference_type: "subscription",
        reference_id: "sub-002",
        note: "购买基础套餐（折扣）",
        meta: {},
        created_at: minutesAgo(45),
      },
    ],
  ],
]);

const adminUserSubscriptions: Map<string, Subscription[]> = new Map([
  [
    "user-admin-001",
    [
      {
        id: "sub-001",
        user_id: "user-admin-001",
        plan_id: "plan-standard",
        plan_code: "standard",
        plan_name: "标准套餐",
        plan_price: "29.00",
        duration_days: 30,
        traffic_quota_bytes: 322122547200,
        traffic_window_days: 30,
        status: "active",
        started_at: minutesAgo(100),
        expires_at: new Date(Date.now() + 25 * 24 * 60 * 60 * 1000).toISOString(),
        replaced_at: null,
        updated_at: minutesAgo(100),
      },
    ],
  ],
  [
    "user-operator-001",
    [
      {
        id: "sub-002",
        user_id: "user-operator-001",
        plan_id: "plan-basic",
        plan_code: "basic",
        plan_name: "基础套餐",
        plan_price: "15.00",
        duration_days: 30,
        traffic_quota_bytes: 107374182400,
        traffic_window_days: 30,
        status: "active",
        started_at: minutesAgo(45),
        expires_at: new Date(Date.now() + 28 * 24 * 60 * 60 * 1000).toISOString(),
        replaced_at: null,
        updated_at: minutesAgo(45),
      },
    ],
  ],
]);

export async function mockAdminListPlans(): Promise<Plan[]> {
  for (const plan of adminPlans) {
    if (!plan.permission_group_id) {
      plan.permission_group_name = null;
      continue;
    }
    const group =
      adminPermissionGroups.find((item) => item.id === plan.permission_group_id) ?? null;
    plan.permission_group_name = group?.name ?? null;
  }
  return clone(adminPlans);
}

export async function mockAdminListPermissionGroups(): Promise<BillingPermissionGroup[]> {
  return clone(adminPermissionGroups);
}

export async function mockAdminCreatePermissionGroup(
  request: UpsertBillingPermissionGroupRequest
): Promise<BillingPermissionGroup> {
  const domainIds = Array.from(new Set(request.domain_ids));
  const group: BillingPermissionGroup = {
    id: request.id || nextId("perm-group"),
    code: request.code,
    name: request.name,
    enabled: request.enabled ?? true,
    domain_ids: domainIds,
    updated_at: new Date().toISOString(),
  };
  adminPermissionGroups.push(group);
  pushAudit("admin.billing.permission_group.create", "permission_group", group.id, {
    code: group.code,
    domain_count: group.domain_ids.length,
  });
  return clone(group);
}

export async function mockAdminUpdatePermissionGroup(
  groupId: string,
  request: UpsertBillingPermissionGroupRequest
): Promise<BillingPermissionGroup> {
  const group = adminPermissionGroups.find((item) => item.id === groupId);
  if (!group) {
    throw { status: 404, message: "权限组不存在" };
  }
  if (request.code !== undefined) group.code = request.code;
  if (request.name !== undefined) group.name = request.name;
  if (request.enabled !== undefined) group.enabled = request.enabled;
  if (request.domain_ids !== undefined) {
    group.domain_ids = Array.from(new Set(request.domain_ids));
  }
  group.updated_at = new Date().toISOString();
  for (const plan of adminPlans) {
    if (plan.permission_group_id === group.id) {
      plan.permission_group_name = group.name;
    }
  }
  pushAudit("admin.billing.permission_group.update", "permission_group", group.id, {
    code: group.code,
    domain_count: group.domain_ids.length,
  });
  return clone(group);
}

export async function mockAdminCreatePlan(request: CreatePlanRequest): Promise<Plan> {
  const group =
    request.permission_group_id == null
      ? null
      : (adminPermissionGroups.find((item) => item.id === request.permission_group_id) ?? null);
  const plan: Plan = {
    id: request.id || nextId("plan"),
    code: request.code,
    name: request.name,
    price: request.price,
    duration_days: request.duration_days,
    traffic_quota_bytes: request.traffic_quota_bytes,
    traffic_window_days: request.traffic_window_days,
    permission_group_id: group?.id ?? null,
    permission_group_name: group?.name ?? null,
    enabled: request.enabled ?? true,
    updated_at: new Date().toISOString(),
  };
  adminPlans.push(plan);
  pushAudit("admin.billing.plan.create", "plan", plan.id, { name: plan.name });
  return clone(plan);
}

export async function mockAdminUpdatePlan(
  planId: string,
  request: UpdatePlanRequest
): Promise<Plan> {
  const plan = adminPlans.find((p) => p.id === planId);
  if (!plan) {
    throw { status: 404, message: "套餐不存在" };
  }
  if (request.code !== undefined) plan.code = request.code;
  if (request.name !== undefined) plan.name = request.name;
  if (request.price !== undefined) plan.price = request.price;
  if (request.duration_days !== undefined) plan.duration_days = request.duration_days;
  if (request.traffic_quota_bytes !== undefined)
    plan.traffic_quota_bytes = request.traffic_quota_bytes;
  if (request.traffic_window_days !== undefined)
    plan.traffic_window_days = request.traffic_window_days;
  if (request.permission_group_id !== undefined) {
    const group =
      request.permission_group_id == null
        ? null
        : (adminPermissionGroups.find((item) => item.id === request.permission_group_id) ?? null);
    if (request.permission_group_id && !group) {
      throw { status: 404, message: "权限组不存在" };
    }
    plan.permission_group_id = group?.id ?? null;
    plan.permission_group_name = group?.name ?? null;
  }
  if (request.enabled !== undefined) plan.enabled = request.enabled;
  plan.updated_at = new Date().toISOString();
  pushAudit(
    "admin.billing.plan.update",
    "plan",
    planId,
    request as unknown as Record<string, unknown>
  );
  return clone(plan);
}

export async function mockAdminDeletePlan(planId: string): Promise<void> {
  const index = adminPlans.findIndex((p) => p.id === planId);
  if (index === -1) {
    throw { status: 404, message: "套餐不存在" };
  }
  adminPlans.splice(index, 1);
  pushAudit("admin.billing.plan.delete", "plan", planId, {});
}

export async function mockAdminListRechargeOrders(limit = 100): Promise<RechargeOrder[]> {
  return clone(adminRechargeOrders.slice(0, limit));
}

export async function mockAdminGetUserWallet(userId: string): Promise<Wallet> {
  const wallet = adminUserWallets.get(userId);
  if (!wallet) {
    return {
      user_id: userId,
      balance: "0.00",
      total_recharged: "0.00",
      total_spent: "0.00",
      updated_at: new Date().toISOString(),
    };
  }
  return clone(wallet);
}

export async function mockAdminGetUserLedger(userId: string, limit = 50): Promise<LedgerEntry[]> {
  const ledger = adminUserLedgers.get(userId) || [];
  return clone(ledger.slice(0, limit));
}

export async function mockAdminGetUserSubscriptions(userId: string): Promise<Subscription[]> {
  const subs = adminUserSubscriptions.get(userId) || [];
  return clone(subs);
}

export async function mockAdminAdjustBalance(
  userId: string,
  request: AdjustBalanceRequest
): Promise<AdjustBalanceResult> {
  let wallet = adminUserWallets.get(userId);
  if (!wallet) {
    wallet = {
      user_id: userId,
      balance: "0.00",
      total_recharged: "0.00",
      total_spent: "0.00",
      updated_at: new Date().toISOString(),
    };
    adminUserWallets.set(userId, wallet);
  }

  const currentBalance = parseFloat(wallet.balance);
  const adjustAmount = parseFloat(request.amount);
  const newBalance = currentBalance + adjustAmount;

  if (newBalance < 0) {
    throw { status: 400, message: "调整后余额不能为负" };
  }

  wallet.balance = newBalance.toFixed(2);
  wallet.updated_at = new Date().toISOString();

  const ledgerEntry: LedgerEntry = {
    id: nextId("ledger"),
    user_id: userId,
    entry_type: "adjustment",
    amount: request.amount,
    balance_after: wallet.balance,
    reference_type: null,
    reference_id: null,
    note: request.note || "管理员调整",
    meta: {},
    created_at: new Date().toISOString(),
  };

  const userLedger = adminUserLedgers.get(userId) || [];
  userLedger.unshift(ledgerEntry);
  adminUserLedgers.set(userId, userLedger);

  pushAudit("admin.billing.balance.adjust", "wallet", userId, {
    amount: request.amount,
    note: request.note,
  });

  return clone(wallet);
}

// Admin Subscription Management Mocks
import type { AssignSubscriptionRequest, UpdateSubscriptionRequest } from "@/lib/types/billing";

export async function mockAdminAssignSubscription(
  userId: string,
  request: AssignSubscriptionRequest
): Promise<Subscription> {
  const plan = adminPlans.find((p) => p.id === request.plan_id);
  if (!plan) {
    throw { status: 404, message: "套餐不存在" };
  }

  const now = new Date();
  const durationDays = request.duration_days ?? plan.duration_days;
  const expiresAt = new Date(now.getTime() + durationDays * 24 * 60 * 60 * 1000);

  const subscription: Subscription = {
    id: nextId("sub"),
    user_id: userId,
    plan_id: plan.id,
    plan_code: plan.code,
    plan_name: plan.name,
    plan_price: plan.price,
    duration_days: durationDays,
    traffic_quota_bytes: plan.traffic_quota_bytes,
    traffic_window_days: plan.traffic_window_days,
    status: "active",
    started_at: now.toISOString(),
    expires_at: expiresAt.toISOString(),
    replaced_at: null,
    updated_at: now.toISOString(),
  };

  const userSubs = adminUserSubscriptions.get(userId) || [];
  userSubs.unshift(subscription);
  adminUserSubscriptions.set(userId, userSubs);

  pushAudit("admin.billing.subscription.assign", "subscription", subscription.id, {
    user_id: userId,
    plan_id: plan.id,
    duration_days: durationDays,
  });

  return clone(subscription);
}

export async function mockAdminUpdateSubscription(
  subscriptionId: string,
  request: UpdateSubscriptionRequest
): Promise<Subscription> {
  for (const [_userId, subs] of adminUserSubscriptions.entries()) {
    const sub = subs.find((s) => s.id === subscriptionId);
    if (sub) {
      sub.expires_at = request.expires_at;
      sub.updated_at = new Date().toISOString();
      pushAudit("admin.billing.subscription.update", "subscription", subscriptionId, {
        expires_at: request.expires_at,
      });
      return clone(sub);
    }
  }
  throw { status: 404, message: "订阅不存在" };
}

export async function mockAdminCancelSubscription(subscriptionId: string): Promise<Subscription> {
  for (const [_userId, subs] of adminUserSubscriptions.entries()) {
    const sub = subs.find((s) => s.id === subscriptionId);
    if (sub) {
      sub.status = "cancelled";
      sub.updated_at = new Date().toISOString();
      pushAudit("admin.billing.subscription.cancel", "subscription", subscriptionId, {});
      return clone(sub);
    }
  }
  throw { status: 404, message: "订阅不存在" };
}

// Admin Billing Config Mocks
import type { BillingConfig, UpdateBillingConfigRequest } from "@/lib/types/billing";

const billingConfig: BillingConfig = {
  epay: {
    gateway_url: "https://pay.example.com/submit.php",
    pid: "1001",
    // key is write-only, not returned
    notify_url: "https://api.example.com/billing/notify",
    return_url: "https://app.example.com/billing/return",
    sitename: "LumenStream",
  },
  billing: {
    enabled: true,
    min_recharge_amount: "10.00",
    max_recharge_amount: "1000.00",
    order_expire_minutes: 30,
    channels: ["alipay", "wxpay"],
  },
};

export async function mockAdminGetBillingConfig(): Promise<BillingConfig> {
  return clone(billingConfig);
}

export async function mockAdminUpdateBillingConfig(
  request: UpdateBillingConfigRequest
): Promise<BillingConfig> {
  if (request.epay) {
    billingConfig.epay = {
      ...billingConfig.epay,
      ...request.epay,
    };
    // If key was provided, we accept it but don't store it in the mock (simulating write-only)
    delete (billingConfig.epay as { key?: string }).key;
  }
  if (request.billing) {
    billingConfig.billing = {
      ...billingConfig.billing,
      ...request.billing,
    };
  }
  pushAudit(
    "admin.billing.config.update",
    "billing_config",
    "global",
    request as unknown as Record<string, unknown>
  );
  return clone(billingConfig);
}

// Scraper Admin Mocks

export async function mockGetScraperSettings(): Promise<ScraperSettingsResponse> {
  return {
    settings: clone(settings),
    libraries: clone(
      libraryStatusItems.map((item) => ({
        id: item.id,
        name: item.name,
        root_path: item.root_path,
        paths: item.paths,
        library_type: item.library_type,
        enabled: item.enabled,
        scraper_policy: item.scraper_policy,
        created_at: minutesAgo(60),
      }))
    ),
  };
}

export async function mockUpsertScraperSettings(payload: {
  settings: WebAppSettings;
  library_policies: Array<{ library_id: string; scraper_policy: Record<string, unknown> }>;
}): Promise<ScraperSettingsResponse> {
  settings = clone(payload.settings);
  payload.library_policies.forEach((policy) => {
    const found = libraryStatusItems.find((library) => library.id === policy.library_id);
    if (found) {
      found.scraper_policy = clone(policy.scraper_policy);
    }
  });
  pushAudit("admin.scraper.settings.upsert", "web_settings", "global", {
    library_policies_updated: payload.library_policies.length,
  });
  return mockGetScraperSettings();
}

export async function mockListScraperProviders(): Promise<ScraperProviderStatus[]> {
  const enabled =
    Boolean((settings.scraper as { enabled?: boolean }).enabled) ||
    Boolean((settings.tmdb as { enabled?: boolean }).enabled);
  const registry = ((settings.scraper as { providers?: string[] }).providers ?? []).map((item) =>
    item.toLowerCase()
  );
  return [
    {
      provider_id: "tmdb",
      display_name: "TMDB",
      provider_kind: "metadata",
      enabled: enabled && registry.includes("tmdb"),
      configured: Boolean((settings.tmdb as { api_key?: string }).api_key),
      healthy:
        enabled &&
        registry.includes("tmdb") &&
        Boolean((settings.tmdb as { api_key?: string }).api_key),
      capabilities: ["search", "details", "images", "people", "external_ids"],
      scenarios: [
        "movie_metadata",
        "series_metadata",
        "season_metadata",
        "episode_metadata",
        "person_metadata",
        "image_fetch",
        "search_by_title",
        "search_by_external_id",
      ],
      message:
        enabled && registry.includes("tmdb")
          ? (settings.tmdb as { api_key?: string }).api_key
            ? "ready"
            : "missing tmdb api key"
          : "scraper disabled or tmdb not selected in provider chain",
      checked_at: new Date().toISOString(),
    },
    {
      provider_id: "tvdb",
      display_name: "TVDB",
      provider_kind: "metadata",
      enabled:
        enabled &&
        registry.includes("tvdb") &&
        Boolean((settings.scraper as { tvdb?: { enabled?: boolean } }).tvdb?.enabled),
      configured: Boolean((settings.scraper as { tvdb?: { api_key?: string } }).tvdb?.api_key),
      healthy:
        enabled &&
        registry.includes("tvdb") &&
        Boolean((settings.scraper as { tvdb?: { enabled?: boolean } }).tvdb?.enabled) &&
        Boolean((settings.scraper as { tvdb?: { api_key?: string } }).tvdb?.api_key),
      capabilities: ["search", "details", "images", "people", "external_ids"],
      scenarios: [
        "movie_metadata",
        "series_metadata",
        "season_metadata",
        "episode_metadata",
        "person_metadata",
        "image_fetch",
        "search_by_title",
        "search_by_external_id",
      ],
      message:
        enabled &&
        registry.includes("tvdb") &&
        Boolean((settings.scraper as { tvdb?: { enabled?: boolean } }).tvdb?.enabled)
          ? (settings.scraper as { tvdb?: { api_key?: string } }).tvdb?.api_key
            ? "ready"
            : "missing tvdb api key"
          : "scraper disabled, tvdb not enabled, or tvdb not selected in provider chain",
      checked_at: new Date().toISOString(),
    },
    {
      provider_id: "bangumi",
      display_name: "Bangumi",
      provider_kind: "metadata",
      enabled:
        enabled &&
        registry.includes("bangumi") &&
        Boolean((settings.scraper as { bangumi?: { enabled?: boolean } }).bangumi?.enabled),
      configured: Boolean(
        (settings.scraper as { bangumi?: { access_token?: string } }).bangumi?.access_token
      ),
      healthy:
        enabled &&
        registry.includes("bangumi") &&
        Boolean((settings.scraper as { bangumi?: { enabled?: boolean } }).bangumi?.enabled) &&
        Boolean(
          (settings.scraper as { bangumi?: { access_token?: string } }).bangumi?.access_token
        ),
      capabilities: ["search", "details", "images", "external_ids"],
      scenarios: [
        "series_metadata",
        "season_metadata",
        "episode_metadata",
        "image_fetch",
        "search_by_title",
      ],
      message:
        enabled &&
        registry.includes("bangumi") &&
        Boolean((settings.scraper as { bangumi?: { enabled?: boolean } }).bangumi?.enabled)
          ? (settings.scraper as { bangumi?: { access_token?: string } }).bangumi?.access_token
            ? "ready"
            : "missing bangumi access token"
          : "scraper disabled, bangumi not enabled, or bangumi not selected in provider chain",
      checked_at: new Date().toISOString(),
    },
  ];
}

export async function mockTestScraperProvider(providerId: string): Promise<ScraperProviderStatus> {
  const providers = await mockListScraperProviders();
  const found = providers.find((provider) => provider.provider_id === providerId);
  if (!found) {
    throw new Error("scraper provider not found");
  }
  return found;
}

export async function mockGetScraperCacheStats(): Promise<ScraperCacheStats> {
  return mockGetTmdbCacheStats();
}

export async function mockListScraperFailures(limit: number): Promise<ScraperFailureEntry[]> {
  return mockListTmdbFailures(limit);
}

export async function mockClearScraperCache(expiredOnly: boolean): Promise<{ removed: number }> {
  return mockClearTmdbCache(expiredOnly);
}

export async function mockClearScraperFailures(): Promise<{ removed: number }> {
  return mockClearTmdbFailures();
}

export async function mockGetTmdbCacheStats(): Promise<TmdbCacheStats> {
  return {
    total_entries: 256,
    entries_with_result: 210,
    expired_entries: 34,
    total_hits: 1580,
  };
}

export async function mockListTmdbFailures(_limit: number): Promise<TmdbFailureEntry[]> {
  return [
    {
      id: "f001",
      media_item_id: "m001",
      item_name: "Unknown Movie 2024",
      item_type: "Movie",
      attempts: 3,
      error: "TMDB API returned 404",
      created_at: minutesAgo(30),
    },
    {
      id: "f002",
      media_item_id: "m002",
      item_name: "Test Series S01E05",
      item_type: "Episode",
      attempts: 2,
      error: "request timeout after 10s",
      created_at: minutesAgo(120),
    },
  ];
}

export async function mockClearTmdbCache(expiredOnly: boolean): Promise<{ removed: number }> {
  pushAudit("admin.tmdb_cache.clear", "tmdb_cache", null, { expired_only: expiredOnly });
  return { removed: expiredOnly ? 34 : 256 };
}

export async function mockClearTmdbFailures(): Promise<{ removed: number }> {
  pushAudit("admin.tmdb_failures.clear", "tmdb_failures", null, {});
  return { removed: 2 };
}

const agentRequests: AgentRequest[] = [
  {
    id: "req-1001",
    request_type: "media_request",
    source: "user_submit",
    user_id: "user-admin-001",
    title: "沙丘：预言",
    content: "想看这部新剧，最好 4K。",
    media_type: "series",
    tmdb_id: 157336,
    series_id: null,
    media_item_id: null,
    season_numbers: [1],
    episode_numbers: [],
    status_user: "processing",
    status_admin: "auto_processing",
    agent_stage: "mp_search",
    priority: 0,
    auto_handled: true,
    admin_note: "",
    agent_note: "已命中 MoviePilot 搜索，正在筛选资源。",
    moviepilot_payload: {},
    moviepilot_result: {},
    last_error: null,
    created_at: minutesAgo(25),
    updated_at: minutesAgo(5),
    closed_at: null,
  },
  {
    id: "req-1002",
    request_type: "missing_episode",
    source: "auto_detected",
    user_id: null,
    title: "基地 缺集",
    content: "自动检测到 S02 缺失 E05, E06",
    media_type: "series",
    tmdb_id: 93740,
    series_id: "series-002",
    media_item_id: null,
    season_numbers: [2],
    episode_numbers: [5, 6],
    status_user: "action_required",
    status_admin: "review_required",
    agent_stage: "manual_review",
    priority: 10,
    auto_handled: false,
    admin_note: "",
    agent_note: "MoviePilot 未找到满足规则的资源，等待管理员处理。",
    moviepilot_payload: {},
    moviepilot_result: { result_count: 1 },
    last_error: "agent fallback to review",
    created_at: minutesAgo(90),
    updated_at: minutesAgo(45),
    closed_at: null,
  },
];

const agentRequestEvents = new Map<string, AgentRequestEvent[]>([
  [
    "req-1001",
    [
      {
        id: "evt-1001-1",
        request_id: "req-1001",
        event_type: "request.created",
        actor_user_id: "user-admin-001",
        actor_username: "admin",
        summary: "已创建求片工单",
        detail: { source: "user_submit" },
        created_at: minutesAgo(25),
      },
      {
        id: "evt-1001-2",
        request_id: "req-1001",
        event_type: "agent.moviepilot.search_started",
        actor_user_id: null,
        actor_username: "system",
        summary: "正在搜索资源",
        detail: { tmdb_id: 157336 },
        created_at: minutesAgo(6),
      },
    ],
  ],
  [
    "req-1002",
    [
      {
        id: "evt-1002-1",
        request_id: "req-1002",
        event_type: "request.auto_created",
        actor_user_id: null,
        actor_username: "system",
        summary: "系统自动发现缺集/漏季并创建工单",
        detail: {},
        created_at: minutesAgo(90),
      },
      {
        id: "evt-1002-2",
        request_id: "req-1002",
        event_type: "agent.review_required",
        actor_user_id: null,
        actor_username: "system",
        summary: "未找到可自动处理结果，已转人工处理",
        detail: { result_count: 1 },
        created_at: minutesAgo(45),
      },
    ],
  ],
]);

function currentAgentSettings(): AgentSettings {
  return clone(settings.agent);
}

function mockWorkflowKindForRequest(request: AgentRequest): string {
  switch (request.request_type) {
    case "media_request":
      return "request_media";
    case "missing_episode":
      return "missing_episode_repair";
    case "missing_season":
      return "missing_season_repair";
    case "feedback":
      return "feedback_triage";
    default:
      return "unknown";
  }
}

function mockWorkflowStepsForRequest(request: AgentRequest): AgentWorkflowStepState[] {
  const common: AgentWorkflowStepState[] = [
    { step: "accepted", label: "接单", status: "completed" },
  ];
  if (request.request_type === "feedback") {
    return [
      ...common,
      {
        step: "normalize",
        label: "标准化",
        status: request.agent_stage === "queued" ? "active" : "completed",
      },
      {
        step: "manual_review",
        label: "人工接管",
        status: request.status_admin === "review_required" ? "blocked" : "pending",
      },
      {
        step: "notify",
        label: "通知回写",
        status: request.status_admin === "completed" ? "completed" : "pending",
      },
    ];
  }
  return [
    ...common,
    {
      step: request.request_type.startsWith("missing_") ? "gap_detect" : "normalize",
      label: request.request_type.startsWith("missing_") ? "缺口检测" : "标准化",
      status: "completed",
    },
    {
      step: request.request_type.startsWith("missing_") ? "metadata_enrich" : "library_check",
      label: request.request_type.startsWith("missing_") ? "元数据补全" : "库内检查",
      status: request.status_admin === "review_required" ? "completed" : "completed",
    },
    {
      step: "provider_search",
      label: "Provider 搜索",
      status:
        request.agent_stage === "mp_search"
          ? request.status_admin === "review_required"
            ? "failed"
            : "active"
          : request.status_admin === "completed"
            ? "completed"
            : "pending",
    },
    {
      step: "filter_dispatch",
      label: "筛选与派发",
      status:
        request.agent_stage === "mp_download" || request.agent_stage === "mp_subscribe"
          ? "active"
          : request.status_admin === "completed"
            ? "completed"
            : request.status_admin === "review_required"
              ? "blocked"
              : "pending",
    },
    {
      step: "verify",
      label: "结果校验",
      status: request.status_admin === "completed" ? "completed" : "pending",
    },
    {
      step: "notify",
      label: "通知回写",
      status: request.status_admin === "completed" ? "completed" : "pending",
    },
  ];
}

function mockManualActionsForRequest(request: AgentRequest) {
  const actions = [
    {
      action: "manual_complete",
      label: "手动完成",
      description: "绕过自动链路，直接完成工单。",
    },
    {
      action: "retry",
      label: "重新触发",
      description: "在补充信息后重新执行处理。",
    },
  ];
  if (request.status_admin === "review_required") {
    actions.unshift(
      {
        action: "approve",
        label: "批准并重试",
        description: "重新进入自动处理。",
      },
      {
        action: "reject",
        label: "拒绝",
        description: "关闭工单并反馈用户。",
      }
    );
  }
  if (request.auto_handled) {
    actions.push({
      action: "handoff",
      label: "转人工接管",
      description: "保留上下文并切换为人工模式。",
    });
  }
  return actions;
}

function mockAgentProviders(): AgentProviderStatus[] {
  return [
    {
      provider_id: "tmdb",
      display_name: "TMDB",
      provider_kind: "metadata",
      enabled: true,
      configured: true,
      healthy: true,
      capabilities: ["metadata"],
      message: "metadata provider ready",
      checked_at: new Date().toISOString(),
    },
    {
      provider_id: "ls_notifications",
      display_name: "LumenStream Notifications",
      provider_kind: "notification",
      enabled: true,
      configured: true,
      healthy: true,
      capabilities: ["notify"],
      message: "internal notification provider ready",
      checked_at: new Date().toISOString(),
    },
    {
      provider_id: "moviepilot",
      display_name: "MoviePilot",
      provider_kind: "subscription_download",
      enabled: settings.agent.moviepilot.enabled,
      configured: Boolean(
        settings.agent.moviepilot.base_url &&
        settings.agent.moviepilot.username &&
        settings.agent.moviepilot.password
      ),
      healthy: settings.agent.moviepilot.enabled,
      capabilities: ["search", "subscribe", "download"],
      message: settings.agent.moviepilot.enabled ? "authentication succeeded" : "provider disabled",
      checked_at: new Date().toISOString(),
    },
  ];
}

function myAgentRequestDetail(requestId: string): AgentRequestDetail {
  const request = agentRequests.find((item) => item.id === requestId);
  if (!request) {
    throw { status: 404, message: "请求不存在" };
  }
  return {
    request: clone(request),
    events: clone(agentRequestEvents.get(requestId) ?? []),
    workflow_kind: mockWorkflowKindForRequest(request),
    workflow_steps: mockWorkflowStepsForRequest(request),
    required_capabilities:
      request.request_type === "feedback"
        ? ["notify"]
        : request.request_type.startsWith("missing_")
          ? ["metadata", "search", "download", "subscribe", "notify"]
          : ["search", "download", "subscribe", "notify"],
    manual_actions: mockManualActionsForRequest(request),
  };
}

export async function mockListMyAgentRequests(
  query: AgentRequestsQuery = {}
): Promise<AgentRequest[]> {
  const currentUserId = mockCurrentDemoUser().Id;
  const items = agentRequests.filter(
    (item) =>
      item.user_id === currentUserId &&
      (!query.request_type || item.request_type === query.request_type)
  );
  return clone(items.slice(0, query.limit ?? 50));
}

export async function mockCreateMyAgentRequest(
  payload: AgentCreateRequest
): Promise<AgentRequestDetail> {
  const now = new Date().toISOString();
  const request: AgentRequest = {
    id: nextId("req"),
    request_type: payload.request_type,
    source: payload.source || "user_submit",
    user_id: mockCurrentDemoUser().Id,
    title: payload.title,
    content: payload.content || "",
    media_type: payload.media_type || "unknown",
    tmdb_id: payload.tmdb_id ?? null,
    media_item_id: payload.media_item_id ?? null,
    series_id: payload.series_id ?? null,
    season_numbers: payload.season_numbers ?? [],
    episode_numbers: payload.episode_numbers ?? [],
    status_user: "processing",
    status_admin: "new",
    agent_stage: "queued",
    priority: 0,
    auto_handled: false,
    admin_note: "",
    agent_note: "请求已入队，等待处理。",
    moviepilot_payload: {},
    moviepilot_result: {},
    last_error: null,
    created_at: now,
    updated_at: now,
    closed_at: null,
  };
  agentRequests.unshift(request);
  agentRequestEvents.set(request.id, [
    {
      id: nextId("evt"),
      request_id: request.id,
      event_type: "request.created",
      actor_user_id: request.user_id,
      actor_username: "admin",
      summary: "已创建求片工单",
      detail: { source: request.source },
      created_at: now,
    },
  ]);
  return myAgentRequestDetail(request.id);
}

export async function mockGetMyAgentRequest(requestId: string): Promise<AgentRequestDetail> {
  return myAgentRequestDetail(requestId);
}

export async function mockResubmitMyAgentRequest(requestId: string): Promise<AgentRequestDetail> {
  const request = agentRequests.find((item) => item.id === requestId);
  if (!request) {
    throw { status: 404, message: "请求不存在" };
  }
  request.status_user = "processing";
  request.status_admin = "new";
  request.agent_stage = "queued";
  request.updated_at = new Date().toISOString();
  request.last_error = null;
  const events = agentRequestEvents.get(requestId) ?? [];
  events.push({
    id: nextId("evt"),
    request_id: requestId,
    event_type: "admin.retry",
    actor_user_id: request.user_id,
    actor_username: "admin",
    summary: "重新触发处理",
    detail: {},
    created_at: request.updated_at,
  });
  agentRequestEvents.set(requestId, events);
  return myAgentRequestDetail(requestId);
}

export async function mockAdminListAgentRequests(
  query: AgentRequestsQuery = {}
): Promise<AgentRequest[]> {
  const items = agentRequests.filter(
    (item) =>
      (!query.request_type || item.request_type === query.request_type) &&
      (!query.status_admin || item.status_admin === query.status_admin)
  );
  return clone(items.slice(0, query.limit ?? 200));
}

export async function mockAdminGetAgentRequest(requestId: string): Promise<AgentRequestDetail> {
  return myAgentRequestDetail(requestId);
}

export async function mockAdminReviewAgentRequest(
  requestId: string,
  payload: AgentReviewRequest
): Promise<AgentRequestDetail> {
  const request = agentRequests.find((item) => item.id === requestId);
  if (!request) {
    throw { status: 404, message: "请求不存在" };
  }
  if (payload.action === "approve") {
    request.status_user = "processing";
    request.status_admin = "approved";
    request.agent_stage = "queued";
    request.agent_note = "管理员批准后重新进入自动处理。";
  } else if (payload.action === "reject") {
    request.status_user = "failed";
    request.status_admin = "rejected";
    request.agent_stage = "closed";
    request.closed_at = new Date().toISOString();
  } else if (payload.action === "ignore") {
    request.status_user = "closed";
    request.status_admin = "ignored";
    request.agent_stage = "closed";
    request.closed_at = new Date().toISOString();
  } else if (payload.action === "manual_complete") {
    request.status_user = "success";
    request.status_admin = "completed";
    request.agent_stage = "closed";
    request.closed_at = new Date().toISOString();
  }
  request.admin_note = payload.note || request.admin_note;
  request.updated_at = new Date().toISOString();
  const events = agentRequestEvents.get(requestId) ?? [];
  events.push({
    id: nextId("evt"),
    request_id: requestId,
    event_type: `admin.${payload.action}`,
    actor_user_id: "user-admin-001",
    actor_username: "admin",
    summary: `管理员执行了 ${payload.action}`,
    detail: { note: payload.note || "" },
    created_at: request.updated_at,
  });
  agentRequestEvents.set(requestId, events);
  pushAudit(
    "admin.agent_request.review",
    "agent_request",
    requestId,
    payload as unknown as Record<string, unknown>
  );
  return myAgentRequestDetail(requestId);
}

export async function mockAdminRetryAgentRequest(requestId: string): Promise<AgentRequestDetail> {
  return mockResubmitMyAgentRequest(requestId);
}

export async function mockAdminGetAgentSettings(): Promise<AgentSettings> {
  return currentAgentSettings();
}

export async function mockAdminListAgentProviders(): Promise<AgentProviderStatus[]> {
  return clone(mockAgentProviders());
}

export async function mockAdminUpdateAgentSettings(payload: AgentSettings): Promise<AgentSettings> {
  settings = {
    ...settings,
    agent: clone(payload),
  };
  pushAudit("admin.agent.settings.upsert", "web_settings", "global", {
    enabled: payload.enabled,
    missing_scan_enabled: payload.missing_scan_enabled,
  });
  return currentAgentSettings();
}

export async function mockAdminTestMoviePilot(
  config: AgentSettings
): Promise<Record<string, unknown>> {
  if (!config.moviepilot.base_url || !config.moviepilot.username || !config.moviepilot.password) {
    throw { status: 400, message: "MoviePilot 配置不完整" };
  }
  return {
    ok: true,
    base_url: config.moviepilot.base_url,
    timeout_seconds: config.moviepilot.timeout_seconds,
  };
}
