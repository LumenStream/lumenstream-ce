import { apiRequest, getApiBaseUrl } from "@/lib/api/client";
import {
  mockAddFavoriteItem,
  mockDeleteItem,
  mockGetItemCounts,
  mockGetItems,
  mockGetItemSubtitles,
  mockGetPlaybackInfo,
  mockGetPerson,
  mockGetTopPlayed,
  mockGetResumeItems,
  mockGetRootItems,
  mockGetShowEpisodes,
  mockGetShowSeasons,
  mockGetUserItem,
  mockGetUserItems,
  mockGetMePlaybackDomains,
  mockRefreshItemMetadata,
  mockRemoveFavoriteItem,
  mockSelectMePlaybackDomain,
  mockUpdateItemMetadata,
  type MockItemsQuery,
} from "@/lib/mock/api";
import { isMockMode, runWithMock } from "@/lib/mock/mode";
import type { MePlaybackDomainsResponse } from "@/lib/types/admin";
import type {
  BaseItem,
  ItemCounts,
  PlaybackInfo,
  QueryResult,
  Season,
  SubtitleTrack,
  TopPlayedSummary,
  UserItemData,
} from "@/lib/types/jellyfin";

export interface ItemsQueryParams {
  parentId?: string;
  includeItemTypes?: string;
  excludeItemTypes?: string;
  personIds?: string;
  searchTerm?: string;
  filters?: string;
  limit?: number;
  startIndex?: number;
}

export interface TopPlayedQueryParams {
  limit?: number;
  windowDays?: number;
  statDate?: string;
}

interface RootItemsCacheEntry {
  value: QueryResult<BaseItem>;
  expiresAt: number;
}

const ROOT_ITEMS_CACHE_TTL_MS = 15_000;
const rootItemsResultCache = new Map<string, RootItemsCacheEntry>();
const rootItemsPendingCache = new Map<string, Promise<QueryResult<BaseItem>>>();

const MOCK_POSTER_GRADIENTS: ReadonlyArray<readonly [string, string]> = [
  ["#7f1d1d", "#be123c"],
  ["#1e3a8a", "#1d4ed8"],
  ["#0f766e", "#059669"],
  ["#4c1d95", "#7c3aed"],
  ["#78350f", "#ea580c"],
];

function hashSeed(seed: string): number {
  return [...seed].reduce((acc, char) => acc + char.charCodeAt(0), 0);
}

function buildMockPosterDataUrl(itemId: string): string {
  const [start, end] = MOCK_POSTER_GRADIENTS[hashSeed(itemId) % MOCK_POSTER_GRADIENTS.length]!;
  const label = `LumenStream ${itemId.slice(0, 8).toUpperCase()}`;
  const svg = `<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 300 450'><defs><linearGradient id='g' x1='0' y1='0' x2='1' y2='1'><stop offset='0%' stop-color='${start}'/><stop offset='100%' stop-color='${end}'/></linearGradient></defs><rect width='300' height='450' fill='url(#g)'/><rect x='20' y='20' width='260' height='410' rx='20' ry='20' fill='rgba(0,0,0,0.25)'/><text x='150' y='230' font-size='24' font-family='Arial, sans-serif' fill='white' text-anchor='middle'>${label}</text></svg>`;
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

function buildMockBackdropDataUrl(itemId: string): string {
  const [start, end] =
    MOCK_POSTER_GRADIENTS[(hashSeed(itemId) + 1) % MOCK_POSTER_GRADIENTS.length]!;
  const label = `LumenStream BACKDROP ${itemId.slice(0, 6).toUpperCase()}`;
  const svg = `<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1280 720'><defs><linearGradient id='g' x1='0' y1='0' x2='1' y2='1'><stop offset='0%' stop-color='${start}'/><stop offset='100%' stop-color='${end}'/></linearGradient></defs><rect width='1280' height='720' fill='url(#g)'/><rect x='64' y='64' width='1152' height='592' rx='28' ry='28' fill='rgba(0,0,0,0.28)'/><text x='640' y='380' font-size='54' font-family='Arial, sans-serif' fill='white' text-anchor='middle'>${label}</text></svg>`;
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

function mapItemsQuery(query: ItemsQueryParams = {}) {
  return {
    ParentId: query.parentId,
    IncludeItemTypes: query.includeItemTypes,
    ExcludeItemTypes: query.excludeItemTypes,
    PersonIds: query.personIds,
    SearchTerm: query.searchTerm,
    Filters: query.filters,
    Limit: query.limit,
    StartIndex: query.startIndex,
  };
}

function toMockQuery(query: ItemsQueryParams = {}): MockItemsQuery {
  return {
    parentId: query.parentId,
    includeItemTypes: query.includeItemTypes,
    personIds: query.personIds,
    searchTerm: query.searchTerm,
    filters: query.filters,
    limit: query.limit,
    startIndex: query.startIndex,
  };
}

export async function getRootItems(userId: string): Promise<QueryResult<BaseItem>> {
  return runWithMock(
    () => mockGetRootItems(userId),
    () => apiRequest<QueryResult<BaseItem>>(`/Users/${userId}/Items/Root`)
  );
}

export async function getRootItemsShared(userId: string): Promise<QueryResult<BaseItem>> {
  const now = Date.now();
  const cached = rootItemsResultCache.get(userId);
  if (cached && cached.expiresAt > now) {
    return cached.value;
  }

  const pending = rootItemsPendingCache.get(userId);
  if (pending) {
    return pending;
  }

  const request = getRootItems(userId)
    .then((result) => {
      rootItemsResultCache.set(userId, {
        value: result,
        expiresAt: Date.now() + ROOT_ITEMS_CACHE_TTL_MS,
      });
      return result;
    })
    .finally(() => {
      rootItemsPendingCache.delete(userId);
    });

  rootItemsPendingCache.set(userId, request);
  return request;
}

export function clearRootItemsSharedCache(userId?: string): void {
  if (typeof userId === "string" && userId.trim()) {
    rootItemsResultCache.delete(userId);
    rootItemsPendingCache.delete(userId);
    return;
  }

  rootItemsResultCache.clear();
  rootItemsPendingCache.clear();
}

export async function getResumeItems(userId: string): Promise<QueryResult<BaseItem>> {
  return runWithMock(
    () => mockGetResumeItems(userId),
    () => apiRequest<QueryResult<BaseItem>>(`/Users/${userId}/Items/Resume`)
  );
}

export async function getUserItems(
  userId: string,
  query: ItemsQueryParams = {}
): Promise<QueryResult<BaseItem>> {
  return runWithMock(
    () => mockGetUserItems(userId, toMockQuery(query)),
    () =>
      apiRequest<QueryResult<BaseItem>>(`/Users/${userId}/Items`, {
        query: mapItemsQuery(query),
      })
  );
}

export async function getItems(query: ItemsQueryParams = {}): Promise<QueryResult<BaseItem>> {
  return runWithMock(
    () => mockGetItems(toMockQuery(query)),
    () =>
      apiRequest<QueryResult<BaseItem>>("/Items", {
        query: mapItemsQuery(query),
      })
  );
}

export async function getPerson(personId: string): Promise<BaseItem> {
  return runWithMock(
    () => mockGetPerson(personId),
    () => apiRequest<BaseItem>(`/Persons/${personId}`)
  );
}

export async function getPersonItems(
  personId: string,
  query: Omit<ItemsQueryParams, "personIds"> = {}
): Promise<QueryResult<BaseItem>> {
  return getItems({
    ...query,
    personIds: personId,
  });
}

export async function getUserItem(userId: string, itemId: string): Promise<BaseItem> {
  return runWithMock(
    () => mockGetUserItem(userId, itemId),
    () => apiRequest<BaseItem>(`/Users/${userId}/Items/${itemId}`)
  );
}

export async function getItemCounts(): Promise<ItemCounts> {
  return runWithMock(
    () => mockGetItemCounts(),
    () => apiRequest<ItemCounts>("/Items/Counts")
  );
}

export async function getTopPlayedItems(
  query: TopPlayedQueryParams = {}
): Promise<TopPlayedSummary> {
  const limit = query.limit ?? 10;
  const windowDays = query.windowDays ?? 1;

  return runWithMock(
    () => mockGetTopPlayed(limit, windowDays, query.statDate),
    () =>
      apiRequest<TopPlayedSummary>("/Items/TopPlayed", {
        query: {
          Limit: limit,
          WindowDays: windowDays,
          StatDate: query.statDate,
        },
      })
  );
}

export async function getPlaybackInfo(itemId: string, userId: string): Promise<PlaybackInfo> {
  return runWithMock(
    () => mockGetPlaybackInfo(itemId, userId),
    () =>
      apiRequest<PlaybackInfo>(`/Items/${itemId}/PlaybackInfo`, {
        query: {
          UserId: userId,
        },
      })
  );
}

export async function getItemSubtitles(itemId: string): Promise<SubtitleTrack[]> {
  return runWithMock(
    () => mockGetItemSubtitles(itemId),
    () => apiRequest<SubtitleTrack[]>(`/Items/${itemId}/Subtitles`)
  );
}

export async function addFavoriteItem(userId: string, itemId: string): Promise<UserItemData> {
  return runWithMock(
    () => mockAddFavoriteItem(userId, itemId),
    () =>
      apiRequest<UserItemData>(`/Users/${userId}/FavoriteItems/${itemId}`, {
        method: "POST",
      })
  );
}

export async function removeFavoriteItem(userId: string, itemId: string): Promise<UserItemData> {
  return runWithMock(
    () => mockRemoveFavoriteItem(userId, itemId),
    () =>
      apiRequest<UserItemData>(`/Users/${userId}/FavoriteItems/${itemId}`, {
        method: "DELETE",
      })
  );
}

export interface UpdateItemMetadataPayload {
  Name?: string;
  Overview?: string;
  ProductionYear?: number;
  TmdbId?: string | number;
  ImdbId?: string;
  ProviderIds?: Record<string, string>;
}

export interface RefreshItemMetadataQuery {
  recursive?: boolean;
  metadataRefreshMode?: string;
  imageRefreshMode?: string;
  replaceAllMetadata?: boolean;
  replaceAllImages?: boolean;
}

export async function updateItemMetadata(
  itemId: string,
  payload: UpdateItemMetadataPayload
): Promise<void> {
  return runWithMock(
    () => mockUpdateItemMetadata(itemId, payload),
    () =>
      apiRequest<void>(`/Items/${itemId}`, {
        method: "POST",
        body: JSON.stringify(payload),
      })
  );
}

export async function refreshItemMetadata(
  itemId: string,
  query: RefreshItemMetadataQuery = {}
): Promise<void> {
  return runWithMock(
    () => mockRefreshItemMetadata(itemId),
    () =>
      apiRequest<void>(`/Items/${itemId}/Refresh`, {
        method: "POST",
        query: {
          Recursive: query.recursive,
          MetadataRefreshMode: query.metadataRefreshMode,
          ImageRefreshMode: query.imageRefreshMode,
          ReplaceAllMetadata: query.replaceAllMetadata,
          ReplaceAllImages: query.replaceAllImages,
        },
      })
  );
}

export async function deleteItem(itemId: string): Promise<void> {
  return runWithMock(
    () => mockDeleteItem(itemId),
    () =>
      apiRequest<void>(`/Items/${itemId}`, {
        method: "DELETE",
      })
  );
}

export async function getMePlaybackDomains(): Promise<MePlaybackDomainsResponse> {
  return runWithMock(
    () => mockGetMePlaybackDomains(),
    () => apiRequest<MePlaybackDomainsResponse>("/Users/Me/PlaybackDomains")
  );
}

export async function selectMePlaybackDomain(domainId: string): Promise<{
  selected_domain_id: string;
  selected_domain_name: string;
}> {
  return runWithMock(
    () => mockSelectMePlaybackDomain(domainId),
    () =>
      apiRequest<{ selected_domain_id: string; selected_domain_name: string }>(
        "/Users/Me/PlaybackDomains/Select",
        {
          method: "POST",
          body: JSON.stringify({ domain_id: domainId }),
        }
      )
  );
}

export interface ShowEpisodesQueryParams {
  seasonId?: string;
  limit?: number;
  startIndex?: number;
}

export async function getShowSeasons(showId: string): Promise<QueryResult<Season>> {
  return runWithMock(
    () => mockGetShowSeasons(showId),
    () =>
      apiRequest<QueryResult<Season>>("/Items", {
        query: {
          ParentId: showId,
          IncludeItemTypes: "Season",
        },
      })
  );
}

export async function getShowEpisodes(
  showId: string,
  query: ShowEpisodesQueryParams = {}
): Promise<QueryResult<BaseItem>> {
  return runWithMock(
    () => mockGetShowEpisodes(showId, query.seasonId),
    () =>
      apiRequest<QueryResult<BaseItem>>(`/Shows/${showId}/Episodes`, {
        query: {
          SeasonId: query.seasonId,
          Limit: query.limit,
          StartIndex: query.startIndex,
        },
      })
  );
}

export function buildItemImageUrl(itemId: string, token?: string): string {
  if (isMockMode()) {
    return buildMockPosterDataUrl(itemId);
  }

  const baseUrl = getApiBaseUrl();
  const query = token ? `?api_key=${encodeURIComponent(token)}` : "";
  return `${baseUrl}/Items/${itemId}/Images/Primary${query}`;
}

export function buildItemBackdropUrl(itemId: string, token?: string, index = 0): string {
  if (isMockMode()) {
    return buildMockBackdropDataUrl(itemId);
  }

  const baseUrl = getApiBaseUrl();
  const query = token ? `?api_key=${encodeURIComponent(token)}` : "";
  return `${baseUrl}/Items/${itemId}/Images/Backdrop/${index}${query}`;
}

export function buildPersonImageUrl(personId: string, token?: string): string {
  if (isMockMode()) {
    return buildMockPosterDataUrl(personId);
  }

  const baseUrl = getApiBaseUrl();
  const query = token ? `?api_key=${encodeURIComponent(token)}` : "";
  return `${baseUrl}/Persons/${personId}/Images/Primary${query}`;
}

export function buildStreamUrl(itemId: string, token: string): string {
  if (isMockMode()) {
    return `https://demo.lumenstream.local/Videos/${itemId}/stream?api_key=${encodeURIComponent(token)}`;
  }

  const baseUrl = getApiBaseUrl();
  return `${baseUrl}/Videos/${itemId}/stream?api_key=${encodeURIComponent(token)}`;
}
