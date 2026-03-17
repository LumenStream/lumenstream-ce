import React, { useEffect, useState } from "react";

import { EmptyState, ErrorState, LoadingState } from "@/components/domain/DataState";
import { PosterItemCard } from "@/components/domain/PosterItemCard";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  createPlaylist,
  deletePlaylist,
  listMyPlaylists,
  listPlaylistItems,
  updatePlaylist,
} from "@/lib/api/playlists";
import type { ApiError } from "@/lib/api/client";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import { resolveMediaItemHref } from "@/lib/media/item-href";
import { toast } from "@/lib/notifications/toast-store";
import type { BaseItem } from "@/lib/types/jellyfin";
import type { Playlist } from "@/lib/types/playlist";

function sortPlaylists(records: Playlist[]): Playlist[] {
  return [...records].sort((a, b) => {
    if (a.is_default !== b.is_default) {
      return a.is_default ? -1 : 1;
    }
    return b.updated_at.localeCompare(a.updated_at);
  });
}

export function PlaylistsCenter() {
  const { session, ready } = useAuthSession();

  const [playlists, setPlaylists] = useState<Playlist[]>([]);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [items, setItems] = useState<BaseItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [itemsLoading, setItemsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [actioningId, setActioningId] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [createName, setCreateName] = useState("");
  const [creating, setCreating] = useState(false);

  useEffect(() => {
    if (!ready || !session) return;

    let cancelled = false;
    setLoading(true);

    listMyPlaylists()
      .then((records) => {
        if (cancelled) return;
        setPlaylists(sortPlaylists(records));
        setError(null);
      })
      .catch((cause) => {
        if (cancelled) return;
        const apiError = cause as ApiError;
        setError(apiError.message || "加载收藏夹失败");
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [ready, session]);

  useEffect(() => {
    if (!ready || !session || !expandedId) {
      setItems([]);
      return;
    }

    let cancelled = false;
    setItemsLoading(true);

    listPlaylistItems(expandedId)
      .then((payload) => {
        if (cancelled) return;
        setItems(payload.items);
      })
      .catch((cause) => {
        if (cancelled) return;
        const apiError = cause as ApiError;
        toast.error(apiError.message || "加载收藏夹内容失败");
        setItems([]);
      })
      .finally(() => {
        if (!cancelled) setItemsLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [ready, session, expandedId]);

  if (!ready || loading) {
    return <LoadingState title="加载收藏夹" description="正在读取你的自定义收藏夹。" />;
  }

  if (error) {
    return <ErrorState title="收藏夹加载失败" description={error} />;
  }

  function toggleExpand(id: string) {
    setExpandedId((prev) => (prev === id ? null : id));
    setEditingId(null);
  }

  async function onTogglePublic(playlist: Playlist, e: React.MouseEvent) {
    e.stopPropagation();
    setActioningId(playlist.id);
    try {
      const updated = await updatePlaylist(playlist.id, { is_public: !playlist.is_public });
      setPlaylists((cur) => sortPlaylists(cur.map((p) => (p.id === updated.id ? updated : p))));
      toast.success(updated.is_public ? "已设为公开" : "已设为私有");
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(apiError.message || "更新失败");
    } finally {
      setActioningId(null);
    }
  }

  async function onDelete(playlist: Playlist, e: React.MouseEvent) {
    e.stopPropagation();
    setActioningId(playlist.id);
    try {
      await deletePlaylist(playlist.id);
      setPlaylists((cur) => cur.filter((p) => p.id !== playlist.id));
      if (expandedId === playlist.id) setExpandedId(null);
      toast.info("收藏夹已删除");
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(apiError.message || "删除失败");
    } finally {
      setActioningId(null);
    }
  }

  function startEdit(playlist: Playlist, e: React.MouseEvent) {
    e.stopPropagation();
    setEditingId(playlist.id);
    setEditName(playlist.name);
  }

  async function saveEdit(playlist: Playlist) {
    const name = editName.trim();
    if (!name) {
      toast.warning("名称不能为空");
      return;
    }
    setActioningId(playlist.id);
    try {
      const updated = await updatePlaylist(playlist.id, { name });
      setPlaylists((cur) => sortPlaylists(cur.map((p) => (p.id === updated.id ? updated : p))));
      setEditingId(null);
      toast.success("已更新");
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(apiError.message || "更新失败");
    } finally {
      setActioningId(null);
    }
  }

  async function onCreate(e: React.FormEvent) {
    e.preventDefault();
    const name = createName.trim();
    if (!name) {
      toast.warning("名称不能为空");
      return;
    }

    setCreating(true);
    try {
      const created = await createPlaylist({ name });
      setPlaylists((cur) => sortPlaylists([created, ...cur]));
      setCreateName("");
      toast.success("收藏夹已创建");
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(apiError.message || "创建失败");
    } finally {
      setCreating(false);
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold">收藏夹</h1>
          <p className="text-muted-foreground text-sm">管理你的自定义收藏夹</p>
        </div>
        <Badge variant="glass">{playlists.length} 个</Badge>
      </div>

      <form className="flex flex-col gap-2 sm:flex-row" onSubmit={(e) => void onCreate(e)}>
        <Input
          value={createName}
          onChange={(e) => setCreateName(e.target.value)}
          placeholder="新建收藏夹名称"
          maxLength={48}
          disabled={creating}
        />
        <Button type="submit" className="sm:w-auto" disabled={creating || !createName.trim()}>
          {creating ? "创建中..." : "新建收藏夹"}
        </Button>
      </form>

      {playlists.length === 0 ? (
        <EmptyState
          title="暂无收藏夹"
          description={'在任意海报上点击"添加到列表"即可创建收藏夹。'}
        />
      ) : (
        <div className="space-y-2">
          {playlists.map((playlist) => {
            const isExpanded = expandedId === playlist.id;
            const isEditing = editingId === playlist.id;
            const isActioning = actioningId === playlist.id;

            return (
              <div key={playlist.id} className="rounded-lg">
                <div
                  role="button"
                  tabIndex={0}
                  className="light:hover:bg-black/[0.03] flex cursor-pointer items-center justify-between gap-3 rounded-lg p-3 transition hover:bg-white/[0.02]"
                  onClick={() => toggleExpand(playlist.id)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      toggleExpand(playlist.id);
                    }
                  }}
                >
                  <div className="min-w-0 flex-1">
                    {isEditing ? (
                      <div className="flex items-center gap-2" onClick={(e) => e.stopPropagation()}>
                        <Input
                          value={editName}
                          onChange={(e) => setEditName(e.target.value)}
                          className="h-8 max-w-[200px]"
                          maxLength={48}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") void saveEdit(playlist);
                            if (e.key === "Escape") setEditingId(null);
                          }}
                        />
                        <Button
                          size="sm"
                          variant="ghost"
                          disabled={isActioning}
                          onClick={() => void saveEdit(playlist)}
                        >
                          保存
                        </Button>
                        <Button size="sm" variant="ghost" onClick={() => setEditingId(null)}>
                          取消
                        </Button>
                      </div>
                    ) : (
                      <div className="flex items-center gap-2">
                        <p className="truncate font-medium">{playlist.name}</p>
                        {playlist.is_default && (
                          <Badge variant="glass" className="shrink-0 text-xs">
                            默认
                          </Badge>
                        )}
                        <Badge variant="glass" className="shrink-0">
                          {playlist.is_public ? "公开" : "私有"}
                        </Badge>
                      </div>
                    )}
                    <p className="text-muted-foreground mt-0.5 text-xs">
                      {playlist.item_count} 项
                      {playlist.description ? ` · ${playlist.description}` : ""}
                    </p>
                  </div>

                  <div className="flex shrink-0 items-center gap-1">
                    {!playlist.is_default && (
                      <Button
                        size="sm"
                        variant="ghost"
                        disabled={isActioning}
                        onClick={(e) => startEdit(playlist, e)}
                      >
                        重命名
                      </Button>
                    )}
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={isActioning}
                      onClick={(e) => void onTogglePublic(playlist, e)}
                    >
                      {playlist.is_public ? "私有" : "公开"}
                    </Button>
                    {!playlist.is_default && (
                      <Button
                        size="sm"
                        variant="ghost"
                        className="text-destructive hover:text-destructive"
                        disabled={isActioning}
                        onClick={(e) => void onDelete(playlist, e)}
                      >
                        删除
                      </Button>
                    )}
                    <span className="text-muted-foreground ml-1 text-sm">
                      {isExpanded ? "▲" : "▼"}
                    </span>
                  </div>
                </div>

                {isExpanded && (
                  <div className="space-y-3 px-3 pb-3">
                    {itemsLoading ? (
                      <p className="text-muted-foreground py-4 text-center text-sm">加载中...</p>
                    ) : items.length === 0 ? (
                      <p className="text-muted-foreground py-4 text-center text-sm">
                        收藏夹为空，可在海报卡上添加内容。
                      </p>
                    ) : (
                      <div className="flex flex-wrap gap-3">
                        {items.map((item) => (
                          <PosterItemCard
                            key={item.Id}
                            item={item}
                            href={resolveMediaItemHref(item)}
                            token={session?.token}
                            userId={session?.user.Id}
                          />
                        ))}
                      </div>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
