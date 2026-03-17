import React, { useEffect, useId, useState } from "react";

import { Modal } from "@/components/domain/Modal";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { addItemToPlaylist, createPlaylist, listMyPlaylists } from "@/lib/api/playlists";
import type { ApiError } from "@/lib/api/client";
import { toast } from "@/lib/notifications/toast-store";
import type { BaseItem } from "@/lib/types/jellyfin";
import type { Playlist } from "@/lib/types/playlist";

interface AddToPlaylistModalProps {
  open: boolean;
  item: BaseItem;
  onClose: () => void;
}

function sortPlaylists(records: Playlist[]): Playlist[] {
  return [...records].sort((a, b) => {
    if (a.is_default !== b.is_default) {
      return a.is_default ? -1 : 1;
    }
    return b.updated_at.localeCompare(a.updated_at);
  });
}

export function AddToPlaylistModal({ open, item, onClose }: AddToPlaylistModalProps) {
  const [playlists, setPlaylists] = useState<Playlist[]>([]);
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [selectedPlaylistIds, setSelectedPlaylistIds] = useState<string[]>([]);
  const [createName, setCreateName] = useState("");
  const checkboxIdPrefix = useId();

  useEffect(() => {
    if (!open) {
      return;
    }

    let cancelled = false;
    setLoading(true);
    setSelectedPlaylistIds([]);
    setCreateName("");

    listMyPlaylists()
      .then((records) => {
        if (cancelled) {
          return;
        }
        setPlaylists(sortPlaylists(records));
      })
      .catch((cause) => {
        if (cancelled) {
          return;
        }
        const apiError = cause as ApiError;
        toast.error(apiError.message || "加载收藏夹失败");
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [open]);

  function togglePlaylist(playlistId: string) {
    setSelectedPlaylistIds((previous) => {
      if (previous.includes(playlistId)) {
        return previous.filter((id) => id !== playlistId);
      }
      return [...previous, playlistId];
    });
  }

  async function onConfirm() {
    if (selectedPlaylistIds.length === 0) {
      toast.warning("请至少选择一个收藏夹");
      return;
    }

    setSubmitting(true);
    try {
      const results = await Promise.allSettled(
        selectedPlaylistIds.map((playlistId) => addItemToPlaylist(playlistId, item.Id))
      );
      const successCount = results.filter((result) => result.status === "fulfilled").length;
      const failureCount = results.length - successCount;

      if (failureCount === 0) {
        toast.success(`已添加到 ${successCount} 个收藏夹`);
        onClose();
        return;
      }

      if (successCount > 0) {
        toast.warning(`已添加 ${successCount} 个收藏夹，${failureCount} 个添加失败`);
        return;
      }

      const failed = results.find((result) => result.status === "rejected");
      const apiError = (failed as PromiseRejectedResult | undefined)?.reason as
        | ApiError
        | undefined;
      toast.error(apiError?.message || "添加失败");
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(apiError.message || "添加失败");
    } finally {
      setSubmitting(false);
    }
  }

  async function onCreateAndAdd() {
    const name = createName.trim();
    if (!name) {
      toast.warning("名称不能为空");
      return;
    }

    setSubmitting(true);
    let createdId: string | null = null;
    try {
      const created = await createPlaylist({ name });
      createdId = created.id;
      setPlaylists((current) => sortPlaylists([created, ...current]));
      setCreateName("");

      await addItemToPlaylist(created.id, item.Id);
      toast.success("已新建收藏夹并添加");
      onClose();
    } catch (cause) {
      const apiError = cause as ApiError;
      if (createdId) {
        const resolvedCreatedId = createdId;
        setSelectedPlaylistIds((previous) =>
          previous.includes(resolvedCreatedId) ? previous : [...previous, resolvedCreatedId]
        );
        toast.warning(apiError.message || "收藏夹已创建，但添加失败");
        return;
      }
      toast.error(apiError.message || "创建失败");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Modal
      open={open}
      title="添加到收藏夹"
      description="选择要添加的收藏夹。"
      onClose={onClose}
      showHeaderClose
      showFooterClose={false}
      overlayClassName="bg-black/50"
      cardClassName="flex h-[30rem] w-[30rem] max-w-[calc(100vw-2rem)] flex-col sm:h-[32rem] sm:w-[32rem]"
      contentClassName="flex-1 overflow-hidden"
    >
      <div className="flex h-full min-h-[18rem] flex-col gap-4">
        <div className="space-y-2">
          <p className="text-muted-foreground text-xs">新建收藏夹并添加当前条目</p>
          <div className="flex gap-2">
            <Input
              value={createName}
              onChange={(event) => setCreateName(event.target.value)}
              placeholder="新建收藏夹名称"
              maxLength={48}
              disabled={loading || submitting}
            />
            <Button
              type="button"
              className="shrink-0"
              onClick={() => void onCreateAndAdd()}
              disabled={loading || submitting || !createName.trim()}
            >
              新建并添加
            </Button>
          </div>
        </div>

        <div className="min-h-[13rem] flex-1">
          {loading ? (
            <p className="text-muted-foreground text-xs">正在加载收藏夹...</p>
          ) : playlists.length === 0 ? (
            <p className="text-muted-foreground text-xs">暂无收藏夹，可先新建一个。</p>
          ) : (
            <fieldset className="h-full space-y-2 overflow-y-auto pr-1" aria-label="收藏夹列表">
              <legend className="sr-only">收藏夹列表</legend>
              {playlists.map((playlist) => {
                const checkboxId = `${checkboxIdPrefix}-${playlist.id}`;
                const checked = selectedPlaylistIds.includes(playlist.id);
                return (
                  <label
                    key={playlist.id}
                    htmlFor={checkboxId}
                    className="border-border bg-muted/50 hover:border-border/80 hover:bg-muted flex cursor-pointer items-center justify-between gap-3 rounded-lg border px-3 py-2.5 transition-colors"
                  >
                    <span className="flex min-w-0 flex-1 items-center gap-2.5">
                      <input
                        id={checkboxId}
                        type="checkbox"
                        checked={checked}
                        onChange={() => togglePlaylist(playlist.id)}
                        className="h-4 w-4 accent-rose-500"
                      />
                      <span className="flex min-w-0 items-baseline gap-1">
                        <span className="truncate text-sm text-white/95">{playlist.name}</span>
                        {playlist.is_default ? (
                          <span className="text-muted-foreground shrink-0 text-xs">(默认)</span>
                        ) : null}
                      </span>
                    </span>
                    <span className="text-muted-foreground shrink-0 text-xs">
                      {playlist.item_count} 项
                    </span>
                  </label>
                );
              })}
            </fieldset>
          )}
        </div>

        <div className="flex shrink-0 gap-2">
          <Button
            className="flex-1"
            onClick={() => void onConfirm()}
            disabled={
              submitting || loading || playlists.length === 0 || selectedPlaylistIds.length === 0
            }
          >
            {submitting ? "处理中..." : "确定"}
          </Button>
          <Button className="flex-1" variant="secondary" onClick={onClose} disabled={submitting}>
            取消
          </Button>
        </div>
      </div>
    </Modal>
  );
}
