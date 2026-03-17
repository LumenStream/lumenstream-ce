import type { BaseItem } from "@/lib/types/jellyfin";

export interface Playlist {
  id: string;
  owner_user_id: string;
  name: string;
  description: string;
  is_public: boolean;
  is_default: boolean;
  item_count: number;
  created_at: string;
  updated_at: string;
}

export interface PlaylistItem {
  playlist_id: string;
  media_item_id: string;
  added_at: string;
}

export interface PlaylistItemsResponse {
  items: BaseItem[];
  total: number;
}

export interface CreatePlaylistPayload {
  name: string;
  description?: string;
  is_public?: boolean;
}

export interface UpdatePlaylistPayload {
  name?: string;
  description?: string;
  is_public?: boolean;
}
