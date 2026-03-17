import { apiRequest, getApiBaseUrl } from "@/lib/api/client";
import {
  mockClearStorageCache,
  mockCreateApiKey,
  mockCreateLibrary,
  mockCreateUser,
  mockCancelTaskRun,
  mockGetSettings,
  mockGetTaskRun,
  mockGetSystemCapabilities,
  mockGetSystemFlags,
  mockGetSystemSummary,
  mockInvalidateStorageCache,
  mockListApiKeys,
  mockListAuditLogs,
  mockListAuthSessions,
  mockListTaskDefinitions,
  mockListTaskRuns,
  mockListLibraries,
  mockListLibraryStatus,
  mockListPlaybackSessions,
  mockListUserSummaries,
  mockListStorageConfigs,
  mockListUsers,
  mockGetAdminUserProfile,
  mockPatchTaskDefinition,
  mockPatchLibrary,
  mockPatchUserProfile,
  mockDeleteUser,
  mockDeleteApiKey,
  mockRunTaskNow,
  mockSetLibraryEnabled,
  mockSetUserEnabled,
  mockUpsertSettings,
  mockUpsertStorageConfig,
  mockBatchSetUserEnabled,
  mockListPlaybackDomains,
  mockUpsertPlaybackDomain,
  mockDeletePlaybackDomain,
  mockListLumenBackendNodes,
  mockCreateLumenBackendNode,
  mockPatchLumenBackendNode,
  mockDeleteLumenBackendNode,
  mockGetLumenBackendNodeSchema,
  mockGetLumenBackendNodeConfig,
  mockUpsertLumenBackendNodeConfig,
  mockUpdateSystemFlags,
  mockGetScraperSettings,
  mockUpsertScraperSettings,
  mockListScraperProviders,
  mockTestScraperProvider,
  mockGetScraperCacheStats,
  mockListScraperFailures,
  mockClearScraperCache,
  mockClearScraperFailures,
} from "@/lib/mock/api";
import { runWithMock } from "@/lib/mock/mode";
import type {
  AdminApiKey,
  AdminCreatedApiKey,
  AdminLibrary,
  AdminLibraryStatusResponse,
  AdminSystemCapabilities,
  AdminSystemFlags,
  AdminSystemSummary,
  AdminTaskDefinition,
  AdminTaskRun,
  AdminUserManageProfile,
  AdminUserSummaryPage,
  AdminUpsertSettingsResponse,
  AdminUser,
  AuditLogEntry,
  AuthSession,
  PlaybackSession,
  UserRole,
  WebAppSettings,
  PlaybackDomain,
  LumenBackendNode,
  LumenBackendNodeRuntimeSchema,
  LumenBackendNodeRuntimeConfig,
  TmdbCacheStats,
  TmdbFailureEntry,
  ScraperCacheStats,
  ScraperFailureEntry,
  ScraperProviderStatus,
  ScraperSettingsResponse,
  LibraryType,
} from "@/lib/types/admin";

export {
  adminAdjustBalance,
  adminAssignSubscription,
  adminCancelSubscription,
  adminCreatePermissionGroup,
  adminCreatePlan,
  adminGetBillingConfig,
  adminGetUserLedger,
  adminGetUserSubscriptions,
  adminGetUserWallet,
  adminListPermissionGroups,
  adminListPlans,
  adminListRechargeOrders,
  adminUpdateBillingConfig,
  adminUpdatePermissionGroup,
  adminUpdatePlan,
  adminUpdateSubscription,
  buildAuditExportUrl,
  buildMockAuditExportCsv,
  getInviteSettings,
  getTopTrafficUsers,
  getUserStreamPolicy,
  getUserTrafficUsage,
  listInviteRebates,
  listInviteRelations,
  resetUserTrafficUsage,
  setUserStreamPolicy,
  upsertInviteSettings,
} from "./admin-commercial";

export async function getSystemSummary(): Promise<AdminSystemSummary> {
  return runWithMock(
    () => mockGetSystemSummary(),
    () => apiRequest<AdminSystemSummary>("/admin/system/summary")
  );
}

export async function getSystemFlags(): Promise<AdminSystemFlags> {
  return runWithMock(
    () => mockGetSystemFlags(),
    () => apiRequest<AdminSystemFlags>("/admin/system/flags")
  );
}

export async function updateSystemFlags(
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
  return runWithMock(
    () => mockUpdateSystemFlags(flags),
    () =>
      apiRequest<AdminSystemFlags>("/admin/system/flags", {
        method: "POST",
        body: JSON.stringify(flags),
      })
  );
}

export async function getSystemCapabilities(): Promise<AdminSystemCapabilities> {
  return runWithMock(
    () => mockGetSystemCapabilities(),
    () => apiRequest<AdminSystemCapabilities>("/admin/system/capabilities")
  );
}

export async function listUsers(): Promise<AdminUser[]> {
  return runWithMock(
    () => mockListUsers(),
    () => apiRequest<AdminUser[]>("/admin/users")
  );
}

export async function createUser(payload: {
  username: string;
  password: string;
  role: UserRole;
}): Promise<AdminUser> {
  return runWithMock(
    () => mockCreateUser(payload),
    () =>
      apiRequest<AdminUser>("/admin/users", {
        method: "POST",
        body: JSON.stringify(payload),
      })
  );
}

export async function setUserEnabled(userId: string, enabled: boolean): Promise<AdminUser> {
  return runWithMock(
    () => mockSetUserEnabled(userId, enabled),
    () =>
      apiRequest<AdminUser>(`/admin/users/${userId}/${enabled ? "enable" : "disable"}`, {
        method: "POST",
      })
  );
}

export async function batchSetUserEnabled(
  userIds: string[],
  enabled: boolean
): Promise<{
  updated: number;
  users: AdminUser[];
}> {
  return runWithMock(
    () => mockBatchSetUserEnabled(userIds, enabled),
    () =>
      apiRequest<{ updated: number; users: AdminUser[] }>("/admin/users/batch-status", {
        method: "POST",
        body: JSON.stringify({ user_ids: userIds, disabled: !enabled }),
      })
  );
}

export interface ListUserSummariesQuery {
  q?: string;
  status?: "all" | "enabled" | "disabled";
  role?: "all" | UserRole;
  page?: number;
  page_size?: number;
  sort_by?: "id" | "email" | "online_devices" | "status" | "subscription" | "role" | "used_bytes";
  sort_dir?: "asc" | "desc";
}

export async function listUserSummaries(
  query: ListUserSummariesQuery = {}
): Promise<AdminUserSummaryPage> {
  const queryParams: Record<string, string | number | boolean | null | undefined> = {
    q: query.q,
    status: query.status,
    role: query.role,
    page: query.page,
    page_size: query.page_size,
    sort_by: query.sort_by,
    sort_dir: query.sort_dir,
  };
  return runWithMock(
    () => mockListUserSummaries(query),
    () =>
      apiRequest<AdminUserSummaryPage>("/admin/users/summary", {
        query: queryParams,
      })
  );
}

export async function getAdminUserProfile(userId: string): Promise<AdminUserManageProfile> {
  return runWithMock(
    () => mockGetAdminUserProfile(userId),
    () => apiRequest<AdminUserManageProfile>(`/admin/users/${userId}/profile`)
  );
}

export interface PatchUserProfilePayload {
  email?: string | null;
  display_name?: string | null;
  remark?: string | null;
  role?: UserRole;
  is_disabled?: boolean;
}

export async function patchUserProfile(
  userId: string,
  payload: PatchUserProfilePayload
): Promise<AdminUserManageProfile> {
  return runWithMock(
    () => mockPatchUserProfile(userId, payload),
    () =>
      apiRequest<AdminUserManageProfile>(`/admin/users/${userId}/profile`, {
        method: "PATCH",
        body: JSON.stringify(payload),
      })
  );
}

export async function deleteUser(userId: string): Promise<void> {
  return runWithMock(
    () => mockDeleteUser(userId),
    () =>
      apiRequest<void>(`/admin/users/${userId}`, {
        method: "DELETE",
      })
  );
}

export async function listLibraries(): Promise<AdminLibrary[]> {
  return runWithMock(
    () => mockListLibraries(),
    () => apiRequest<AdminLibrary[]>("/admin/libraries")
  );
}

export interface CreateLibraryPayload {
  name: string;
  paths: string[];
  library_type: LibraryType;
}

export async function createLibrary(payload: CreateLibraryPayload): Promise<AdminLibrary> {
  return runWithMock(
    () => mockCreateLibrary(payload),
    () =>
      apiRequest<AdminLibrary>("/admin/libraries", {
        method: "POST",
        body: JSON.stringify(payload),
      })
  );
}

export async function listLibraryStatus(): Promise<AdminLibraryStatusResponse> {
  return runWithMock(
    () => mockListLibraryStatus(),
    () => apiRequest<AdminLibraryStatusResponse>("/admin/libraries/status")
  );
}

export async function setLibraryEnabled(
  libraryId: string,
  enabled: boolean
): Promise<AdminLibrary> {
  return runWithMock(
    () => mockSetLibraryEnabled(libraryId, enabled),
    () =>
      apiRequest<AdminLibrary>(`/admin/libraries/${libraryId}/${enabled ? "enable" : "disable"}`, {
        method: "POST",
      })
  );
}

export async function patchLibrary(
  libraryId: string,
  patch: {
    name?: string;
    library_type?: LibraryType;
    paths?: string[];
    scraper_policy?: Record<string, unknown>;
  }
): Promise<AdminLibrary> {
  return runWithMock(
    () => mockPatchLibrary(libraryId, patch),
    () =>
      apiRequest<AdminLibrary>(`/admin/libraries/${libraryId}`, {
        method: "PATCH",
        body: JSON.stringify(patch),
      })
  );
}

export async function uploadLibraryCover(
  libraryId: string,
  file: File,
  imageType = "Primary"
): Promise<void> {
  const normalizedType = encodeURIComponent(imageType.trim() || "Primary");
  const contentType = file.type || "application/octet-stream";

  return runWithMock(
    async () => undefined,
    () =>
      apiRequest<void>(`/Items/${libraryId}/Images/${normalizedType}`, {
        method: "POST",
        headers: {
          "Content-Type": contentType,
        },
        body: file,
      })
  );
}

export async function deleteLibraryCover(libraryId: string, imageType = "Primary"): Promise<void> {
  const normalizedType = encodeURIComponent(imageType.trim() || "Primary");

  return runWithMock(
    async () => undefined,
    () =>
      apiRequest<void>(`/Items/${libraryId}/Images/${normalizedType}`, {
        method: "DELETE",
      })
  );
}

export async function listTaskDefinitions(): Promise<AdminTaskDefinition[]> {
  return runWithMock(
    () => mockListTaskDefinitions(),
    () => apiRequest<AdminTaskDefinition[]>("/admin/task-center/tasks")
  );
}

export async function patchTaskDefinition(
  taskKey: string,
  patch: {
    enabled?: boolean;
    cron_expr?: string;
    default_payload?: Record<string, unknown>;
    max_attempts?: number;
  }
): Promise<AdminTaskDefinition> {
  return runWithMock(
    () => mockPatchTaskDefinition(taskKey, patch),
    () =>
      apiRequest<AdminTaskDefinition>(`/admin/task-center/tasks/${taskKey}`, {
        method: "PATCH",
        body: JSON.stringify(patch),
      })
  );
}

export async function runTaskNow(
  taskKey: string,
  payloadOverride?: Record<string, unknown>
): Promise<AdminTaskRun> {
  return runWithMock(
    () => mockRunTaskNow(taskKey, payloadOverride),
    () =>
      apiRequest<AdminTaskRun>(`/admin/task-center/tasks/${taskKey}/run`, {
        method: "POST",
        body: JSON.stringify({ payload_override: payloadOverride }),
      })
  );
}

export async function listTaskRuns(options?: {
  limit?: number;
  task_key?: string;
  status?: string;
  trigger_type?: string;
  exclude_kinds?: string;
}): Promise<AdminTaskRun[]> {
  const limit = options?.limit ?? 50;
  const query = {
    limit,
    task_key: options?.task_key,
    status: options?.status,
    trigger_type: options?.trigger_type,
    exclude_kinds: options?.exclude_kinds,
  };
  return runWithMock(
    () => mockListTaskRuns(query),
    () =>
      apiRequest<AdminTaskRun[]>("/admin/task-center/runs", {
        query,
      })
  );
}

export async function getTaskRun(runId: string): Promise<AdminTaskRun> {
  return runWithMock(
    () => mockGetTaskRun(runId),
    () => apiRequest<AdminTaskRun>(`/admin/task-center/runs/${runId}`)
  );
}

export async function cancelTaskRun(runId: string): Promise<AdminTaskRun> {
  return runWithMock(
    () => mockCancelTaskRun(runId),
    () =>
      apiRequest<AdminTaskRun>(`/admin/task-center/runs/${runId}/cancel`, {
        method: "POST",
      })
  );
}

export function getTaskRunsWebSocketUrl(token: string): string {
  const base = getApiBaseUrl();
  const httpUrl = new URL(`${base}/admin/task-center/ws`);
  httpUrl.searchParams.set("token", token);
  if (httpUrl.protocol === "https:") {
    httpUrl.protocol = "wss:";
  } else {
    httpUrl.protocol = "ws:";
  }
  return httpUrl.toString();
}

export async function listPlaybackSessions(options?: {
  limit?: number;
  active_only?: boolean;
}): Promise<PlaybackSession[]> {
  const limit = options?.limit ?? 80;
  const activeOnly = options?.active_only ?? false;

  return runWithMock(
    () => mockListPlaybackSessions(limit, activeOnly),
    () =>
      apiRequest<PlaybackSession[]>("/admin/sessions", {
        query: options,
      })
  );
}

export async function listAuthSessions(options?: {
  limit?: number;
  active_only?: boolean;
}): Promise<AuthSession[]> {
  const limit = options?.limit ?? 80;
  const activeOnly = options?.active_only ?? false;

  return runWithMock(
    () => mockListAuthSessions(limit, activeOnly),
    () =>
      apiRequest<AuthSession[]>("/admin/auth-sessions", {
        query: options,
      })
  );
}

export async function listApiKeys(limit = 100): Promise<AdminApiKey[]> {
  return runWithMock(
    () => mockListApiKeys(limit),
    () =>
      apiRequest<AdminApiKey[]>("/admin/api-keys", {
        query: { limit },
      })
  );
}

export async function createApiKey(name: string): Promise<AdminCreatedApiKey> {
  return runWithMock(
    () => mockCreateApiKey(name),
    () =>
      apiRequest<AdminCreatedApiKey>("/admin/api-keys", {
        method: "POST",
        body: JSON.stringify({ name }),
      })
  );
}

export async function deleteApiKey(keyId: string): Promise<void> {
  return runWithMock(
    () => mockDeleteApiKey(keyId),
    () =>
      apiRequest<void>(`/admin/api-keys/${keyId}`, {
        method: "DELETE",
      })
  );
}

export async function getSettings(includeSecrets = false): Promise<WebAppSettings> {
  return runWithMock(
    () => mockGetSettings(),
    () =>
      apiRequest<WebAppSettings>("/admin/settings", {
        query: { include_secrets: includeSecrets },
      })
  );
}

export async function upsertSettings(
  settings: WebAppSettings
): Promise<AdminUpsertSettingsResponse> {
  return runWithMock(
    () => mockUpsertSettings(settings),
    () =>
      apiRequest<AdminUpsertSettingsResponse>("/admin/settings", {
        method: "POST",
        body: JSON.stringify(settings),
      })
  );
}

export async function listAuditLogs(limit = 200): Promise<AuditLogEntry[]> {
  return runWithMock(
    () => mockListAuditLogs(limit),
    () =>
      apiRequest<AuditLogEntry[]>("/admin/audit-logs", {
        query: { limit },
      })
  );
}

export async function listStorageConfigs(
  includeSecrets = false
): Promise<Record<string, unknown>[]> {
  return runWithMock(
    () => mockListStorageConfigs(),
    () =>
      apiRequest<Record<string, unknown>[]>("/admin/storage-configs", {
        query: { include_secrets: includeSecrets },
      })
  );
}

export async function upsertStorageConfig(payload: {
  kind: string;
  name: string;
  config: Record<string, unknown>;
  enabled?: boolean;
}): Promise<Record<string, unknown>> {
  return runWithMock(
    () => mockUpsertStorageConfig(payload),
    () =>
      apiRequest<Record<string, unknown>>("/admin/storage-configs", {
        method: "POST",
        body: JSON.stringify(payload),
      })
  );
}

export async function listPlaybackDomains(): Promise<PlaybackDomain[]> {
  return runWithMock(
    () => mockListPlaybackDomains(),
    () => apiRequest<PlaybackDomain[]>("/admin/playback-domains")
  );
}

export async function upsertPlaybackDomain(payload: {
  id?: string;
  name: string;
  base_url: string;
  enabled?: boolean;
  priority?: number;
  is_default?: boolean;
  lumenbackend_node_id?: string | null;
  traffic_multiplier?: number;
}): Promise<PlaybackDomain> {
  return runWithMock(
    () => mockUpsertPlaybackDomain(payload),
    () =>
      apiRequest<PlaybackDomain>("/admin/playback-domains", {
        method: "POST",
        body: JSON.stringify(payload),
      })
  );
}

export async function deletePlaybackDomain(domainId: string): Promise<{ deleted: boolean }> {
  return runWithMock(
    () => mockDeletePlaybackDomain(domainId),
    () =>
      apiRequest<{ deleted: boolean }>(`/admin/playback-domains/${domainId}`, {
        method: "DELETE",
      })
  );
}

export async function listLumenBackendNodes(): Promise<LumenBackendNode[]> {
  return runWithMock(
    () => mockListLumenBackendNodes(),
    () => apiRequest<LumenBackendNode[]>("/admin/lumenbackend/nodes")
  );
}

export async function createLumenBackendNode(payload: {
  node_id: string;
  node_name?: string;
  enabled?: boolean;
}): Promise<LumenBackendNode> {
  return runWithMock(
    () => mockCreateLumenBackendNode(payload),
    () =>
      apiRequest<LumenBackendNode>("/admin/lumenbackend/nodes", {
        method: "POST",
        body: JSON.stringify(payload),
      })
  );
}

export async function patchLumenBackendNode(
  nodeId: string,
  payload: {
    node_name?: string | null;
    enabled?: boolean;
  }
): Promise<LumenBackendNode> {
  return runWithMock(
    () => mockPatchLumenBackendNode(nodeId, payload),
    () =>
      apiRequest<LumenBackendNode>(`/admin/lumenbackend/nodes/${nodeId}`, {
        method: "PATCH",
        body: JSON.stringify(payload),
      })
  );
}

export async function deleteLumenBackendNode(nodeId: string): Promise<{ deleted: boolean }> {
  return runWithMock(
    () => mockDeleteLumenBackendNode(nodeId),
    () =>
      apiRequest<{ deleted: boolean }>(`/admin/lumenbackend/nodes/${nodeId}`, {
        method: "DELETE",
      })
  );
}

export async function getLumenBackendNodeSchema(
  nodeId: string
): Promise<LumenBackendNodeRuntimeSchema> {
  return runWithMock(
    () => mockGetLumenBackendNodeSchema(nodeId),
    () => apiRequest<LumenBackendNodeRuntimeSchema>(`/admin/lumenbackend/nodes/${nodeId}/schema`)
  );
}

export async function getLumenBackendNodeConfig(
  nodeId: string,
  includeSecrets = false
): Promise<LumenBackendNodeRuntimeConfig> {
  return runWithMock(
    () => mockGetLumenBackendNodeConfig(nodeId, includeSecrets),
    () =>
      apiRequest<LumenBackendNodeRuntimeConfig>(`/admin/lumenbackend/nodes/${nodeId}/config`, {
        query: { include_secrets: includeSecrets },
      })
  );
}

export async function upsertLumenBackendNodeConfig(
  nodeId: string,
  config: Record<string, unknown>
): Promise<LumenBackendNodeRuntimeConfig> {
  return runWithMock(
    () => mockUpsertLumenBackendNodeConfig(nodeId, config),
    () =>
      apiRequest<LumenBackendNodeRuntimeConfig>(`/admin/lumenbackend/nodes/${nodeId}/config`, {
        method: "POST",
        body: JSON.stringify({ config }),
      })
  );
}

export interface CacheOperationResult {
  success: boolean;
  message: string;
}

export async function clearStorageCache(): Promise<CacheOperationResult> {
  return runWithMock(
    () => mockClearStorageCache(),
    () =>
      apiRequest<CacheOperationResult>("/admin/storage/cache/cleanup", {
        method: "POST",
      })
  );
}

export async function invalidateStorageCache(): Promise<CacheOperationResult> {
  return runWithMock(
    () => mockInvalidateStorageCache(),
    () =>
      apiRequest<CacheOperationResult>("/admin/storage/cache/invalidate", {
        method: "POST",
      })
  );
}

// Scraper Admin APIs

export async function getScraperSettings(): Promise<ScraperSettingsResponse> {
  return runWithMock(
    () => mockGetScraperSettings(),
    () => apiRequest<ScraperSettingsResponse>("/admin/scraper/settings")
  );
}

export async function upsertScraperSettings(payload: {
  settings: WebAppSettings;
  library_policies: Array<{ library_id: string; scraper_policy: Record<string, unknown> }>;
}): Promise<ScraperSettingsResponse> {
  return runWithMock(
    () => mockUpsertScraperSettings(payload),
    () =>
      apiRequest<ScraperSettingsResponse>("/admin/scraper/settings", {
        method: "POST",
        body: JSON.stringify(payload),
      })
  );
}

export async function listScraperProviders(): Promise<ScraperProviderStatus[]> {
  return runWithMock(
    () => mockListScraperProviders(),
    () => apiRequest<ScraperProviderStatus[]>("/admin/scraper/providers")
  );
}

export async function testScraperProvider(providerId: string): Promise<ScraperProviderStatus> {
  return runWithMock(
    () => mockTestScraperProvider(providerId),
    () =>
      apiRequest<ScraperProviderStatus>(`/admin/scraper/providers/${providerId}/test`, {
        method: "POST",
      })
  );
}

export async function getScraperCacheStats(): Promise<ScraperCacheStats> {
  return runWithMock(
    () => mockGetScraperCacheStats(),
    () => apiRequest<ScraperCacheStats>("/admin/scraper/cache-stats")
  );
}

export async function listScraperFailures(limit = 100): Promise<ScraperFailureEntry[]> {
  return runWithMock(
    () => mockListScraperFailures(limit),
    () =>
      apiRequest<ScraperFailureEntry[]>("/admin/scraper/failures", {
        query: { limit },
      })
  );
}

export async function clearScraperCache(expiredOnly = false): Promise<{ removed: number }> {
  return runWithMock(
    () => mockClearScraperCache(expiredOnly),
    () =>
      apiRequest<{ removed: number }>("/admin/scraper/cache/clear", {
        method: "POST",
        body: JSON.stringify({ expired_only: expiredOnly }),
      })
  );
}

export async function clearScraperFailures(): Promise<{ removed: number }> {
  return runWithMock(
    () => mockClearScraperFailures(),
    () =>
      apiRequest<{ removed: number }>("/admin/scraper/failures/clear", {
        method: "POST",
      })
  );
}

export async function getTmdbCacheStats(): Promise<TmdbCacheStats> {
  return getScraperCacheStats();
}

export async function listTmdbFailures(limit = 100): Promise<TmdbFailureEntry[]> {
  return listScraperFailures(limit);
}

export async function clearTmdbCache(expiredOnly = false): Promise<{ removed: number }> {
  return clearScraperCache(expiredOnly);
}

export async function clearTmdbFailures(): Promise<{ removed: number }> {
  return clearScraperFailures();
}
