export interface MockItemsQuery {
  parentId?: string;
  includeItemTypes?: string;
  searchTerm?: string;
  filters?: string;
  limit?: number;
  startIndex?: number;
}

function mockDisabled(name: string): never {
  throw new Error(`mock feature disabled: ${name}`);
}

export function mockAddFavoriteItem(..._args: unknown[]): never {
  return mockDisabled("mockAddFavoriteItem");
}

export function mockAddItemToPlaylist(..._args: unknown[]): never {
  return mockDisabled("mockAddItemToPlaylist");
}

export function mockAdminAdjustBalance(..._args: unknown[]): never {
  return mockDisabled("mockAdminAdjustBalance");
}

export function mockAdminAssignSubscription(..._args: unknown[]): never {
  return mockDisabled("mockAdminAssignSubscription");
}

export function mockAdminCancelSubscription(..._args: unknown[]): never {
  return mockDisabled("mockAdminCancelSubscription");
}

export function mockAdminCreatePlan(..._args: unknown[]): never {
  return mockDisabled("mockAdminCreatePlan");
}

export function mockAdminGetBillingConfig(..._args: unknown[]): never {
  return mockDisabled("mockAdminGetBillingConfig");
}

export function mockAdminGetInviteSettings(..._args: unknown[]): never {
  return mockDisabled("mockAdminGetInviteSettings");
}

export function mockAdminGetUserLedger(..._args: unknown[]): never {
  return mockDisabled("mockAdminGetUserLedger");
}

export function mockAdminGetUserSubscriptions(..._args: unknown[]): never {
  return mockDisabled("mockAdminGetUserSubscriptions");
}

export function mockAdminGetUserWallet(..._args: unknown[]): never {
  return mockDisabled("mockAdminGetUserWallet");
}

export function mockAdminListPlans(..._args: unknown[]): never {
  return mockDisabled("mockAdminListPlans");
}

export function mockAdminListPermissionGroups(..._args: unknown[]): never {
  return mockDisabled("mockAdminListPermissionGroups");
}

export function mockAdminCreatePermissionGroup(..._args: unknown[]): never {
  return mockDisabled("mockAdminCreatePermissionGroup");
}

export function mockAdminUpdatePermissionGroup(..._args: unknown[]): never {
  return mockDisabled("mockAdminUpdatePermissionGroup");
}

export function mockAdminListInviteRebates(..._args: unknown[]): never {
  return mockDisabled("mockAdminListInviteRebates");
}

export function mockAdminListInviteRelations(..._args: unknown[]): never {
  return mockDisabled("mockAdminListInviteRelations");
}

export function mockAdminListRechargeOrders(..._args: unknown[]): never {
  return mockDisabled("mockAdminListRechargeOrders");
}

export function mockAdminUpdateBillingConfig(..._args: unknown[]): never {
  return mockDisabled("mockAdminUpdateBillingConfig");
}

export function mockAdminUpsertInviteSettings(..._args: unknown[]): never {
  return mockDisabled("mockAdminUpsertInviteSettings");
}

export function mockAdminUpdatePlan(..._args: unknown[]): never {
  return mockDisabled("mockAdminUpdatePlan");
}

export function mockAdminDeletePlan(..._args: unknown[]): never {
  return mockDisabled("mockAdminDeletePlan");
}

export function mockAdminUpdateSubscription(..._args: unknown[]): never {
  return mockDisabled("mockAdminUpdateSubscription");
}

export function mockAuthenticateByName(..._args: unknown[]): never {
  return mockDisabled("mockAuthenticateByName");
}

export function mockGetMyInviteSummary(..._args: unknown[]): never {
  return mockDisabled("mockGetMyInviteSummary");
}

export function mockGetMyTrafficUsageByMedia(..._args: unknown[]): never {
  return mockDisabled("mockGetMyTrafficUsageByMedia");
}

export function mockBatchSetUserEnabled(..._args: unknown[]): never {
  return mockDisabled("mockBatchSetUserEnabled");
}

export function mockBuildAuditCsv(..._args: unknown[]): never {
  return mockDisabled("mockBuildAuditCsv");
}

export function mockClearStorageCache(..._args: unknown[]): never {
  return mockDisabled("mockClearStorageCache");
}

export function mockCreateApiKey(..._args: unknown[]): never {
  return mockDisabled("mockCreateApiKey");
}

export function mockCreateLibrary(..._args: unknown[]): never {
  return mockDisabled("mockCreateLibrary");
}

export function mockCreatePlaylist(..._args: unknown[]): never {
  return mockDisabled("mockCreatePlaylist");
}

export function mockCreateLumenBackendNode(..._args: unknown[]): never {
  return mockDisabled("mockCreateLumenBackendNode");
}

export function mockCreateUser(..._args: unknown[]): never {
  return mockDisabled("mockCreateUser");
}

export function mockCurrentDemoUser(..._args: unknown[]): never {
  return mockDisabled("mockCurrentDemoUser");
}

export function mockDeletePlaylist(..._args: unknown[]): never {
  return mockDisabled("mockDeletePlaylist");
}

export function mockDeleteItem(..._args: unknown[]): never {
  return mockDisabled("mockDeleteItem");
}

export function mockDeleteUser(..._args: unknown[]): never {
  return mockDisabled("mockDeleteUser");
}

export function mockDeleteLumenBackendNode(..._args: unknown[]): never {
  return mockDisabled("mockDeleteLumenBackendNode");
}

export function mockGetAdminUserProfile(..._args: unknown[]): never {
  return mockDisabled("mockGetAdminUserProfile");
}

export function mockGetItemCounts(..._args: unknown[]): never {
  return mockDisabled("mockGetItemCounts");
}

export function mockGetItemSubtitles(..._args: unknown[]): never {
  return mockDisabled("mockGetItemSubtitles");
}

export function mockGetItems(..._args: unknown[]): never {
  return mockDisabled("mockGetItems");
}

export function mockGetMePlaybackDomains(..._args: unknown[]): never {
  return mockDisabled("mockGetMePlaybackDomains");
}

export function mockGetPlaybackInfo(..._args: unknown[]): never {
  return mockDisabled("mockGetPlaybackInfo");
}

export function mockGetPerson(..._args: unknown[]): never {
  return mockDisabled("mockGetPerson");
}

export function mockGetPlaylist(..._args: unknown[]): never {
  return mockDisabled("mockGetPlaylist");
}

export function mockGetResumeItems(..._args: unknown[]): never {
  return mockDisabled("mockGetResumeItems");
}

export function mockGetRootItems(..._args: unknown[]): never {
  return mockDisabled("mockGetRootItems");
}

export function mockGetSettings(..._args: unknown[]): never {
  return mockDisabled("mockGetSettings");
}

export function mockGetShowEpisodes(..._args: unknown[]): never {
  return mockDisabled("mockGetShowEpisodes");
}

export function mockGetShowSeasons(..._args: unknown[]): never {
  return mockDisabled("mockGetShowSeasons");
}

export function mockListMyAgentRequests(..._args: unknown[]): never {
  return mockDisabled("mockListMyAgentRequests");
}

export function mockCreateMyAgentRequest(..._args: unknown[]): never {
  return mockDisabled("mockCreateMyAgentRequest");
}

export function mockGetMyAgentRequest(..._args: unknown[]): never {
  return mockDisabled("mockGetMyAgentRequest");
}

export function mockResubmitMyAgentRequest(..._args: unknown[]): never {
  return mockDisabled("mockResubmitMyAgentRequest");
}

export function mockAdminListAgentRequests(..._args: unknown[]): never {
  return mockDisabled("mockAdminListAgentRequests");
}

export function mockAdminGetAgentRequest(..._args: unknown[]): never {
  return mockDisabled("mockAdminGetAgentRequest");
}

export function mockAdminReviewAgentRequest(..._args: unknown[]): never {
  return mockDisabled("mockAdminReviewAgentRequest");
}

export function mockAdminRetryAgentRequest(..._args: unknown[]): never {
  return mockDisabled("mockAdminRetryAgentRequest");
}

export function mockAdminGetAgentSettings(..._args: unknown[]): never {
  return mockDisabled("mockAdminGetAgentSettings");
}

export function mockAdminListAgentProviders(..._args: unknown[]): never {
  return mockDisabled("mockAdminListAgentProviders");
}

export function mockAdminUpdateAgentSettings(..._args: unknown[]): never {
  return mockDisabled("mockAdminUpdateAgentSettings");
}

export function mockAdminTestMoviePilot(..._args: unknown[]): never {
  return mockDisabled("mockAdminTestMoviePilot");
}

export function mockGetLumenBackendNodeConfig(..._args: unknown[]): never {
  return mockDisabled("mockGetLumenBackendNodeConfig");
}

export function mockGetLumenBackendNodeSchema(..._args: unknown[]): never {
  return mockDisabled("mockGetLumenBackendNodeSchema");
}

export function mockGetSystemCapabilities(..._args: unknown[]): never {
  return mockDisabled("mockGetSystemCapabilities");
}

export function mockGetScraperSettings(..._args: unknown[]): never {
  return mockDisabled("mockGetScraperSettings");
}

export function mockUpsertScraperSettings(..._args: unknown[]): never {
  return mockDisabled("mockUpsertScraperSettings");
}

export function mockListScraperProviders(..._args: unknown[]): never {
  return mockDisabled("mockListScraperProviders");
}

export function mockTestScraperProvider(..._args: unknown[]): never {
  return mockDisabled("mockTestScraperProvider");
}

export function mockGetScraperCacheStats(..._args: unknown[]): never {
  return mockDisabled("mockGetScraperCacheStats");
}

export function mockListScraperFailures(..._args: unknown[]): never {
  return mockDisabled("mockListScraperFailures");
}

export function mockClearScraperCache(..._args: unknown[]): never {
  return mockDisabled("mockClearScraperCache");
}

export function mockClearScraperFailures(..._args: unknown[]): never {
  return mockDisabled("mockClearScraperFailures");
}

export function mockGetSystemFlags(..._args: unknown[]): never {
  return mockDisabled("mockGetSystemFlags");
}

export function mockGetSystemSummary(..._args: unknown[]): never {
  return mockDisabled("mockGetSystemSummary");
}

export function mockGetTaskRun(..._args: unknown[]): never {
  return mockDisabled("mockGetTaskRun");
}

export function mockCancelTaskRun(..._args: unknown[]): never {
  return mockDisabled("mockCancelTaskRun");
}

export function mockGetTopPlayed(..._args: unknown[]): never {
  return mockDisabled("mockGetTopPlayed");
}

export function mockGetTopTrafficUsers(..._args: unknown[]): never {
  return mockDisabled("mockGetTopTrafficUsers");
}

export function mockGetUserById(..._args: unknown[]): never {
  return mockDisabled("mockGetUserById");
}

export function mockGetUserItem(..._args: unknown[]): never {
  return mockDisabled("mockGetUserItem");
}

export function mockGetUserItems(..._args: unknown[]): never {
  return mockDisabled("mockGetUserItems");
}

export function mockGetUserStreamPolicy(..._args: unknown[]): never {
  return mockDisabled("mockGetUserStreamPolicy");
}

export function mockGetUserTrafficUsage(..._args: unknown[]): never {
  return mockDisabled("mockGetUserTrafficUsage");
}

export function mockInvalidateStorageCache(..._args: unknown[]): never {
  return mockDisabled("mockInvalidateStorageCache");
}

export function mockListApiKeys(..._args: unknown[]): never {
  return mockDisabled("mockListApiKeys");
}

export function mockListAuditLogs(..._args: unknown[]): never {
  return mockDisabled("mockListAuditLogs");
}

export function mockListAuthSessions(..._args: unknown[]): never {
  return mockDisabled("mockListAuthSessions");
}

export function mockListLibraries(..._args: unknown[]): never {
  return mockDisabled("mockListLibraries");
}

export function mockListLibraryStatus(..._args: unknown[]): never {
  return mockDisabled("mockListLibraryStatus");
}

export function mockListMyPlaylists(..._args: unknown[]): never {
  return mockDisabled("mockListMyPlaylists");
}

export function mockListPlaybackDomains(..._args: unknown[]): never {
  return mockDisabled("mockListPlaybackDomains");
}

export function mockListPlaybackSessions(..._args: unknown[]): never {
  return mockDisabled("mockListPlaybackSessions");
}

export function mockListPlaylistItems(..._args: unknown[]): never {
  return mockDisabled("mockListPlaylistItems");
}

export function mockListPublicPlaylistsByUser(..._args: unknown[]): never {
  return mockDisabled("mockListPublicPlaylistsByUser");
}

export function mockListLumenBackendNodes(..._args: unknown[]): never {
  return mockDisabled("mockListLumenBackendNodes");
}

export function mockListStorageConfigs(..._args: unknown[]): never {
  return mockDisabled("mockListStorageConfigs");
}

export function mockListTaskDefinitions(..._args: unknown[]): never {
  return mockDisabled("mockListTaskDefinitions");
}

export function mockListTaskRuns(..._args: unknown[]): never {
  return mockDisabled("mockListTaskRuns");
}

export function mockListUserSummaries(..._args: unknown[]): never {
  return mockDisabled("mockListUserSummaries");
}

export function mockListUsers(..._args: unknown[]): never {
  return mockDisabled("mockListUsers");
}

export function mockLogout(..._args: unknown[]): never {
  return mockDisabled("mockLogout");
}

export function mockRegisterWithInvite(..._args: unknown[]): never {
  return mockDisabled("mockRegisterWithInvite");
}

export function mockPatchTaskDefinition(..._args: unknown[]): never {
  return mockDisabled("mockPatchTaskDefinition");
}

export function mockPatchLibrary(..._args: unknown[]): never {
  return mockDisabled("mockPatchLibrary");
}

export function mockPatchLumenBackendNode(..._args: unknown[]): never {
  return mockDisabled("mockPatchLumenBackendNode");
}

export function mockPatchUserProfile(..._args: unknown[]): never {
  return mockDisabled("mockPatchUserProfile");
}

export function mockRemoveFavoriteItem(..._args: unknown[]): never {
  return mockDisabled("mockRemoveFavoriteItem");
}

export function mockRemoveItemFromPlaylist(..._args: unknown[]): never {
  return mockDisabled("mockRemoveItemFromPlaylist");
}

export function mockResetUserTrafficUsage(..._args: unknown[]): never {
  return mockDisabled("mockResetUserTrafficUsage");
}

export function mockResetMyInviteCode(..._args: unknown[]): never {
  return mockDisabled("mockResetMyInviteCode");
}

export function mockRefreshItemMetadata(..._args: unknown[]): never {
  return mockDisabled("mockRefreshItemMetadata");
}

export function mockDeleteApiKey(..._args: unknown[]): never {
  return mockDisabled("mockDeleteApiKey");
}

export function mockRunTaskNow(..._args: unknown[]): never {
  return mockDisabled("mockRunTaskNow");
}

export function mockSelectMePlaybackDomain(..._args: unknown[]): never {
  return mockDisabled("mockSelectMePlaybackDomain");
}

export function mockSetLibraryEnabled(..._args: unknown[]): never {
  return mockDisabled("mockSetLibraryEnabled");
}

export function mockSetUserEnabled(..._args: unknown[]): never {
  return mockDisabled("mockSetUserEnabled");
}

export function mockSetUserStreamPolicy(..._args: unknown[]): never {
  return mockDisabled("mockSetUserStreamPolicy");
}

export function mockUpdatePlaylist(..._args: unknown[]): never {
  return mockDisabled("mockUpdatePlaylist");
}

export function mockUpdateItemMetadata(..._args: unknown[]): never {
  return mockDisabled("mockUpdateItemMetadata");
}

export function mockUpsertPlaybackDomain(..._args: unknown[]): never {
  return mockDisabled("mockUpsertPlaybackDomain");
}

export function mockDeletePlaybackDomain(..._args: unknown[]): never {
  return mockDisabled("mockDeletePlaybackDomain");
}

export function mockUpsertSettings(..._args: unknown[]): never {
  return mockDisabled("mockUpsertSettings");
}

export function mockUpsertLumenBackendNodeConfig(..._args: unknown[]): never {
  return mockDisabled("mockUpsertLumenBackendNodeConfig");
}

export function mockUpsertStorageConfig(..._args: unknown[]): never {
  return mockDisabled("mockUpsertStorageConfig");
}

export function mockUpdateSystemFlags(..._args: unknown[]): never {
  return mockDisabled("mockUpdateSystemFlags");
}

export function mockGetTmdbCacheStats(..._args: unknown[]): never {
  return mockDisabled("mockGetTmdbCacheStats");
}

export function mockListTmdbFailures(..._args: unknown[]): never {
  return mockDisabled("mockListTmdbFailures");
}

export function mockClearTmdbCache(..._args: unknown[]): never {
  return mockDisabled("mockClearTmdbCache");
}

export function mockClearTmdbFailures(..._args: unknown[]): never {
  return mockDisabled("mockClearTmdbFailures");
}
