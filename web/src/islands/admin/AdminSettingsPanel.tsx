import React, { useEffect, useState } from "react";

import { ErrorState, LoadingState } from "@/components/domain/DataState";
import { Modal } from "@/components/domain/Modal";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import {
  clearStorageCache,
  getSettings,
  invalidateStorageCache,
  upsertSettings,
} from "@/lib/api/admin";
import type { ApiError } from "@/lib/api/client";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import { toast } from "@/lib/notifications/toast-store";
import type { WebAppSettings } from "@/lib/types/admin";

export function AdminSettingsPanel() {
  const { ready } = useAuthSession({ requireAdmin: true });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [settings, setSettings] = useState<WebAppSettings | null>(null);
  const [draft, setDraft] = useState("");
  const [tmdbApiKey, setTmdbApiKey] = useState("");
  const [localMediaExts, setLocalMediaExts] = useState("");
  const [localStreamRoute, setLocalStreamRoute] = useState("v1/streams/local");
  const [savingSettings, setSavingSettings] = useState(false);

  const [cacheLoading, setCacheLoading] = useState<"cleanup" | "invalidate" | null>(null);
  const [cacheConfirmAction, setCacheConfirmAction] = useState<"cleanup" | "invalidate" | null>(
    null
  );

  function normalizeExtInput(value: string): string[] {
    return Array.from(
      new Set(
        value
          .split(",")
          .map((item) => item.trim().replace(/^\./, "").toLowerCase())
          .filter((item) => item.length > 0 && item !== "strm")
      )
    );
  }

  function applySettingsToForm(payload: WebAppSettings) {
    setSettings(payload);
    setTmdbApiKey(payload.tmdb?.api_key ?? "");
    setLocalMediaExts((payload.scan?.local_media_exts ?? []).join(","));
    setLocalStreamRoute(String(payload.storage?.local_stream_route ?? "v1/streams/local"));
    setDraft(JSON.stringify(payload, null, 2));
  }

  async function reload() {
    setLoading(true);
    try {
      const payload = await getSettings(false);
      applySettingsToForm(payload);
      setError(null);
    } catch (cause) {
      const apiError = cause as ApiError;
      setError(apiError.message || "加载设置失败");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (!ready) {
      return;
    }
    void reload();
  }, [ready]);

  async function onSave() {
    setSavingSettings(true);

    let parsed: WebAppSettings;
    try {
      parsed = JSON.parse(draft) as WebAppSettings;
    } catch (cause) {
      const message =
        cause instanceof SyntaxError ? `JSON 解析失败：${cause.message}` : "JSON 解析失败";
      toast.error(message);
      setSavingSettings(false);
      return;
    }

    try {
      if (tmdbApiKey) {
        parsed.tmdb.api_key = tmdbApiKey;
      }
      parsed.scan = {
        ...parsed.scan,
        local_media_exts: normalizeExtInput(localMediaExts),
      };
      parsed.storage = {
        ...parsed.storage,
        local_stream_route: localStreamRoute.trim() || "v1/streams/local",
      };
      const result = await upsertSettings(parsed);
      applySettingsToForm(result.settings);
      toast.success(result.restart_required ? "设置已保存，需重启后端生效。" : "设置已保存。");
    } catch (cause) {
      const apiError = cause as ApiError;
      const message = `保存失败：${apiError.message}`;
      toast.error(message);
    } finally {
      setSavingSettings(false);
    }
  }

  async function onConfirmCacheAction() {
    if (!cacheConfirmAction) {
      return;
    }

    const action = cacheConfirmAction;
    setCacheConfirmAction(null);
    setCacheLoading(action);
    try {
      const result =
        action === "cleanup" ? await clearStorageCache() : await invalidateStorageCache();
      toast.success(result.message);
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(
        action === "cleanup"
          ? `清除缓存失败：${apiError.message}`
          : `使缓存失效失败：${apiError.message}`
      );
    } finally {
      setCacheLoading(null);
    }
  }

  if (!ready || loading) {
    return <LoadingState title="加载系统设置" />;
  }

  if (error) {
    return <ErrorState title="设置页加载失败" description={error} />;
  }

  return (
    <div className="space-y-8">
      <section className="border-border/50 border-b pb-6">
        <h3 className="text-sm font-medium">存储缓存管理</h3>
        <p className="text-muted-foreground mt-1 mb-4 text-xs">
          清除或使存储缓存失效。此操作不可逆，请谨慎操作。
        </p>
        <div className="space-y-3">
          <div className="flex flex-wrap gap-2">
            <Button
              variant="secondary"
              onClick={() => setCacheConfirmAction("cleanup")}
              disabled={cacheLoading !== null}
            >
              {cacheLoading === "cleanup" ? "清除中..." : "清除缓存"}
            </Button>
            <Button
              variant="secondary"
              onClick={() => setCacheConfirmAction("invalidate")}
              disabled={cacheLoading !== null}
            >
              {cacheLoading === "invalidate" ? "处理中..." : "使缓存失效"}
            </Button>
          </div>
        </div>
      </section>

      <section className="border-border/50 border-b pb-6">
        <h3 className="text-sm font-medium">Scraper Provider 配置</h3>
        <p className="text-muted-foreground mt-1 mb-4 text-xs">
          配置默认 TMDB Provider 凭据，用于刮削系统的元数据补齐。
        </p>
        <div className="flex items-center gap-3">
          <Input
            type="password"
            className="max-w-sm font-mono text-xs"
            placeholder="输入 TMDB API Key"
            value={tmdbApiKey}
            onChange={(e) => setTmdbApiKey(e.target.value)}
          />
        </div>
      </section>

      <section className="border-border/50 border-b pb-6">
        <h3 className="text-sm font-medium">本地文件扫描与同域推流</h3>
        <p className="text-muted-foreground mt-1 mb-4 text-xs">
          配置可扫描的本地媒体后缀，以及本地文件通过后端域名推流时使用的路径。`.strm`
          始终支持，无需填写。
        </p>
        <div className="grid gap-4 md:grid-cols-2">
          <label className="space-y-1">
            <span className="text-muted-foreground text-xs">本地媒体后缀</span>
            <Input
              className="font-mono text-xs"
              placeholder="mp4,mkv,iso"
              value={localMediaExts}
              onChange={(event) => setLocalMediaExts(event.target.value)}
            />
            <p className="text-muted-foreground text-[11px]">英文逗号分隔，例如：mp4,mkv,flv</p>
          </label>
          <label className="space-y-1">
            <span className="text-muted-foreground text-xs">本地推流路由</span>
            <Input
              className="font-mono text-xs"
              placeholder="v1/streams/local"
              value={localStreamRoute}
              onChange={(event) => setLocalStreamRoute(event.target.value)}
            />
            <p className="text-muted-foreground text-[11px]">
              反向代理把该路径转发到本地推流后端，即可复用现有后端域名。
            </p>
          </label>
        </div>
      </section>

      <section>
        <h3 className="text-sm font-medium">系统设置（JSON）</h3>
        <p className="text-muted-foreground mt-1 mb-4 text-xs">
          显示 `GET /admin/settings` 返回值，提交到 `POST
          /admin/settings`。此页面不包含敏感字段明文。
        </p>
        <div className="space-y-3">
          <Textarea
            className="min-h-[420px] font-mono text-xs"
            value={draft}
            onChange={(event) => {
              setDraft(event.target.value);
            }}
          />
          <div className="flex flex-wrap gap-2">
            <Button onClick={onSave} disabled={savingSettings}>
              {savingSettings ? "保存中..." : "保存设置"}
            </Button>
            <Button
              variant="secondary"
              onClick={() => {
                if (settings) {
                  applySettingsToForm(settings);
                }
              }}
              disabled={savingSettings}
            >
              还原编辑内容
            </Button>
            <Button variant="outline" onClick={() => void reload()} disabled={savingSettings}>
              刷新
            </Button>
          </div>
        </div>
      </section>

      <Modal
        open={Boolean(cacheConfirmAction)}
        title={cacheConfirmAction === "cleanup" ? "确认清除缓存" : "确认使缓存失效"}
        description="该操作不可逆，请确认后继续。"
        onClose={() => setCacheConfirmAction(null)}
        showHeaderClose
        showFooterClose={false}
      >
        <div className="space-y-4 text-sm">
          <p>
            {cacheConfirmAction === "cleanup"
              ? "确认要清除全部存储缓存吗？"
              : "确认要使当前存储缓存全部失效吗？"}
          </p>
          <div className="flex justify-end gap-2">
            <Button type="button" variant="secondary" onClick={() => setCacheConfirmAction(null)}>
              取消
            </Button>
            <Button
              type="button"
              variant="destructive"
              onClick={() => void onConfirmCacheAction()}
              disabled={cacheLoading !== null}
            >
              {cacheConfirmAction === "cleanup" ? "确认清除缓存" : "确认使缓存失效"}
            </Button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
