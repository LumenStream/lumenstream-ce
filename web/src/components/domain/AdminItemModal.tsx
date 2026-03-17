import { useEffect, useState } from "react";

import { Modal } from "@/components/domain/Modal";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { deleteItem, refreshItemMetadata, updateItemMetadata } from "@/lib/api/items";
import type { ApiError } from "@/lib/api/client";
import { toast } from "@/lib/notifications/toast-store";
import type { BaseItem } from "@/lib/types/jellyfin";

interface AdminItemModalProps {
  item: BaseItem | null;
  open: boolean;
  onClose: () => void;
  onSuccess?: () => void;
  onDeleted?: () => void;
}

interface AdminMetadataFormState {
  name: string;
  overview: string;
  productionYear: string;
  tmdbId: string;
  imdbId: string;
}

function findProviderId(item: BaseItem, key: string): string {
  const providerIds = item.ProviderIds || {};
  const exact = providerIds[key];
  if (exact) {
    return exact;
  }

  const matched = Object.entries(providerIds).find(
    ([candidate]) => candidate.toLowerCase() === key.toLowerCase()
  );
  return matched?.[1] || "";
}

export function AdminItemModal({ item, open, onClose, onSuccess, onDeleted }: AdminItemModalProps) {
  const [adminSaving, setAdminSaving] = useState(false);
  const [adminRefreshing, setAdminRefreshing] = useState(false);
  const [adminDeleting, setAdminDeleting] = useState(false);
  const [adminDeleteConfirm, setAdminDeleteConfirm] = useState("");
  const [adminForm, setAdminForm] = useState<AdminMetadataFormState>({
    name: "",
    overview: "",
    productionYear: "",
    tmdbId: "",
    imdbId: "",
  });

  const adminBusy = adminSaving || adminRefreshing || adminDeleting;

  useEffect(() => {
    if (!open || !item) {
      return;
    }

    setAdminForm({
      name: item.Name || "",
      overview: item.Overview || "",
      productionYear: item.ProductionYear ? String(item.ProductionYear) : "",
      tmdbId: findProviderId(item, "Tmdb"),
      imdbId: findProviderId(item, "Imdb"),
    });
    setAdminDeleteConfirm("");
  }, [open, item]);

  async function onAdminSaveMetadata() {
    if (!item) {
      return;
    }

    const name = adminForm.name.trim();
    if (!name) {
      toast.warning("媒体名称不能为空");
      return;
    }

    const productionYear = adminForm.productionYear.trim();
    let parsedYear: number | undefined;
    if (productionYear) {
      parsedYear = Number.parseInt(productionYear, 10);
      if (!Number.isFinite(parsedYear) || parsedYear <= 0) {
        toast.warning("年份格式不正确");
        return;
      }
    }

    const tmdbId = adminForm.tmdbId.trim();
    const imdbId = adminForm.imdbId.trim();
    const providerIds: Record<string, string> = {};
    if (tmdbId) {
      providerIds.Tmdb = tmdbId;
    }
    if (imdbId) {
      providerIds.Imdb = imdbId;
    }

    setAdminSaving(true);
    try {
      await updateItemMetadata(item.Id, {
        Name: name,
        Overview: adminForm.overview.trim(),
        ProductionYear: parsedYear,
        TmdbId: tmdbId || undefined,
        ImdbId: imdbId || undefined,
        ProviderIds: Object.keys(providerIds).length > 0 ? providerIds : undefined,
      });
      toast.success("媒体元数据已更新");
      onSuccess?.();
    } catch (cause) {
      toast.error(`更新失败：${(cause as ApiError).message}`);
    } finally {
      setAdminSaving(false);
    }
  }

  async function onAdminRescrape() {
    if (!item) {
      return;
    }

    setAdminRefreshing(true);
    try {
      await refreshItemMetadata(item.Id, { replaceAllMetadata: true });
      toast.success("重新刮削任务已执行");
      onSuccess?.();
    } catch (cause) {
      toast.error(`重新刮削失败：${(cause as ApiError).message}`);
    } finally {
      setAdminRefreshing(false);
    }
  }

  async function onAdminRefreshImages() {
    if (!item) {
      return;
    }

    setAdminRefreshing(true);
    try {
      await refreshItemMetadata(item.Id, {
        replaceAllImages: true,
        imageRefreshMode: "FullRefresh",
      });
      toast.success("图片已重新入库");
      onSuccess?.();
    } catch (cause) {
      toast.error(`重新入库图片失败：${(cause as ApiError).message}`);
    } finally {
      setAdminRefreshing(false);
    }
  }

  async function onAdminDeleteItem() {
    if (!item) {
      return;
    }

    if (adminDeleteConfirm.trim() !== item.Name.trim()) {
      toast.warning("请输入完整媒体名称以确认删除");
      return;
    }

    setAdminDeleting(true);
    try {
      await deleteItem(item.Id);
      toast.success("媒体已删除");
      onClose();
      onDeleted?.();
    } catch (cause) {
      toast.error(`删除失败：${(cause as ApiError).message}`);
    } finally {
      setAdminDeleting(false);
    }
  }

  if (!item) return null;

  return (
    <Modal
      open={open}
      title="管理员媒体操作"
      description="可编辑元数据、重新刮削或删除当前媒体。"
      onClose={onClose}
      showHeaderClose
      showFooterClose={false}
      overlayClassName="bg-black/60"
    >
      <div className="space-y-4">
        <div className="grid gap-3 sm:grid-cols-2">
          <label className="space-y-1">
            <span className="text-muted-foreground text-xs">标题</span>
            <Input
              value={adminForm.name}
              onChange={(event) =>
                setAdminForm((current) => ({ ...current, name: event.target.value }))
              }
              disabled={adminBusy}
            />
          </label>
          <label className="space-y-1">
            <span className="text-muted-foreground text-xs">年份</span>
            <Input
              value={adminForm.productionYear}
              onChange={(event) =>
                setAdminForm((current) => ({
                  ...current,
                  productionYear: event.target.value.replaceAll(/[^\d]/g, ""),
                }))
              }
              placeholder="例如 2024"
              maxLength={4}
              disabled={adminBusy}
            />
          </label>
        </div>

        <label className="space-y-1">
          <span className="text-muted-foreground text-xs">简介</span>
          <Textarea
            value={adminForm.overview}
            onChange={(event) =>
              setAdminForm((current) => ({ ...current, overview: event.target.value }))
            }
            rows={4}
            disabled={adminBusy}
          />
        </label>

        <div className="grid gap-3 sm:grid-cols-2">
          <label className="space-y-1">
            <span className="text-muted-foreground text-xs">TMDB ID</span>
            <Input
              value={adminForm.tmdbId}
              onChange={(event) =>
                setAdminForm((current) => ({
                  ...current,
                  tmdbId: event.target.value.replaceAll(/[^\d]/g, ""),
                }))
              }
              placeholder="例如 157336"
              disabled={adminBusy}
            />
          </label>
          <label className="space-y-1">
            <span className="text-muted-foreground text-xs">IMDb ID</span>
            <Input
              value={adminForm.imdbId}
              onChange={(event) =>
                setAdminForm((current) => ({ ...current, imdbId: event.target.value }))
              }
              placeholder="例如 tt0816692"
              disabled={adminBusy}
            />
          </label>
        </div>

        <div className="flex flex-wrap gap-2">
          <Button type="button" onClick={() => void onAdminSaveMetadata()} disabled={adminBusy}>
            {adminSaving ? "保存中..." : "保存元数据"}
          </Button>
          <Button
            type="button"
            variant="secondary"
            onClick={() => void onAdminRescrape()}
            disabled={adminBusy}
          >
            {adminRefreshing ? "执行中..." : "重新刮削"}
          </Button>
          <Button
            type="button"
            variant="secondary"
            onClick={() => void onAdminRefreshImages()}
            disabled={adminBusy}
          >
            {adminRefreshing ? "执行中..." : "重新入库图片"}
          </Button>
        </div>

        <div className="space-y-2 rounded-lg border border-red-500/30 bg-red-500/10 p-3">
          <p className="text-sm font-medium text-red-300">危险操作：删除媒体</p>
          <p className="text-xs text-red-200/80">
            删除将同时移除条目，并尝试删除关联的 <code>.strm</code> 文件。
          </p>
          <label className="space-y-1">
            <span className="text-muted-foreground text-xs">
              输入 <span className="font-semibold text-red-200">{item.Name}</span> 以确认删除
            </span>
            <Input
              value={adminDeleteConfirm}
              onChange={(event) => setAdminDeleteConfirm(event.target.value)}
              disabled={adminBusy}
            />
          </label>
          <Button
            type="button"
            variant="destructive"
            onClick={() => void onAdminDeleteItem()}
            disabled={adminBusy || adminDeleteConfirm.trim() !== item.Name.trim()}
          >
            {adminDeleting ? "删除中..." : "删除媒体"}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
