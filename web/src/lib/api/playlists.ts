import { apiRequest } from "@/lib/api/client";
import {
  mockAddItemToPlaylist,
  mockCreatePlaylist,
  mockDeletePlaylist,
  mockGetPlaylist,
  mockListMyPlaylists,
  mockListPlaylistItems,
  mockListPublicPlaylistsByUser,
  mockRemoveItemFromPlaylist,
  mockUpdatePlaylist,
} from "@/lib/mock/api";
import { runWithMock } from "@/lib/mock/mode";
import type {
  CreatePlaylistPayload,
  Playlist,
  PlaylistItem,
  PlaylistItemsResponse,
  UpdatePlaylistPayload,
} from "@/lib/types/playlist";

export async function listMyPlaylists(): Promise<Playlist[]> {
  return runWithMock(
    () => mockListMyPlaylists(),
    () => apiRequest<Playlist[]>("/api/playlists/mine")
  );
}

export async function listPublicPlaylistsByUser(userId: string): Promise<Playlist[]> {
  return runWithMock(
    () => mockListPublicPlaylistsByUser(userId),
    () => apiRequest<Playlist[]>(`/api/users/${userId}/playlists/public`)
  );
}

export async function createPlaylist(payload: CreatePlaylistPayload): Promise<Playlist> {
  return runWithMock(
    () => mockCreatePlaylist(payload),
    () =>
      apiRequest<Playlist>("/api/playlists", {
        method: "POST",
        body: JSON.stringify(payload),
      })
  );
}

export async function getPlaylist(playlistId: string): Promise<Playlist> {
  return runWithMock(
    () => mockGetPlaylist(playlistId),
    () => apiRequest<Playlist>(`/api/playlists/${playlistId}`)
  );
}

export async function updatePlaylist(
  playlistId: string,
  payload: UpdatePlaylistPayload
): Promise<Playlist> {
  return runWithMock(
    () => mockUpdatePlaylist(playlistId, payload),
    () =>
      apiRequest<Playlist>(`/api/playlists/${playlistId}`, {
        method: "PATCH",
        body: JSON.stringify(payload),
      })
  );
}

export async function deletePlaylist(playlistId: string): Promise<{ deleted: boolean }> {
  return runWithMock(
    () => mockDeletePlaylist(playlistId),
    () =>
      apiRequest<{ deleted: boolean }>(`/api/playlists/${playlistId}`, {
        method: "DELETE",
      })
  );
}

export async function listPlaylistItems(playlistId: string): Promise<PlaylistItemsResponse> {
  return runWithMock(
    () => mockListPlaylistItems(playlistId),
    () => apiRequest<PlaylistItemsResponse>(`/api/playlists/${playlistId}/items`)
  );
}

export async function addItemToPlaylist(playlistId: string, itemId: string): Promise<PlaylistItem> {
  return runWithMock(
    () => mockAddItemToPlaylist(playlistId, itemId),
    () =>
      apiRequest<PlaylistItem>(`/api/playlists/${playlistId}/items`, {
        method: "POST",
        body: JSON.stringify({ item_id: itemId }),
      })
  );
}

export async function removeItemFromPlaylist(
  playlistId: string,
  itemId: string
): Promise<{ removed: boolean }> {
  return runWithMock(
    () => mockRemoveItemFromPlaylist(playlistId, itemId),
    () =>
      apiRequest<{ removed: boolean }>(`/api/playlists/${playlistId}/items/${itemId}`, {
        method: "DELETE",
      })
  );
}
