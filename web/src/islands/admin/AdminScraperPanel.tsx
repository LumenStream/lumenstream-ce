import React, { useEffect, useMemo, useState } from "react";

import { ErrorState, LoadingState } from "@/components/domain/DataState";
import { SortableProviderList } from "@/components/ui/sortable-provider-list";
import { Modal } from "@/components/domain/Modal";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  clearScraperCache,
  clearScraperFailures,
  getScraperCacheStats,
  getScraperSettings,
  getSystemFlags,
  getSystemSummary,
  listScraperFailures,
  listScraperProviders,
  runTaskNow,
  testScraperProvider,
  updateSystemFlags,
  upsertScraperSettings,
} from "@/lib/api/admin";
import { SCRAPER_SCENARIO_KEYS } from "@/lib/admin/scraper-policy";
import type { ApiError } from "@/lib/api/client";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import { toast } from "@/lib/notifications/toast-store";
import type {
  AdminSystemFlags,
  AdminSystemSummary,
  ScraperCacheStats,
  ScraperFailureEntry,
  ScraperProviderStatus,
  WebAppSettings,
} from "@/lib/types/admin";

type ConfirmAction = "clear-expired" | "clear-all" | "clear-failures" | null;

export function AdminScraperPanel() {
  const { ready } = useAuthSession({ requireAdmin: true });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [flags, setFlags] = useState<AdminSystemFlags | null>(null);
  const [summary, setSummary] = useState<AdminSystemSummary | null>(null);
  const [settings, setSettings] = useState<WebAppSettings | null>(null);
  const [providers, setProviders] = useState<ScraperProviderStatus[]>([]);
  const [cacheStats, setCacheStats] = useState<ScraperCacheStats | null>(null);
  const [failures, setFailures] = useState<ScraperFailureEntry[]>([]);

  const [defaultStrategy, setDefaultStrategy] = useState("");
  const [providersInput, setProvidersInput] = useState("");
  const [scenarioInputs, setScenarioInputs] = useState<Record<string, string>>({});
  const [tmdbApiKey, setTmdbApiKey] = useState("");
  const [tmdbLanguage, setTmdbLanguage] = useState("");
  const [timeoutSeconds, setTimeoutSeconds] = useState("");
  const [requestIntervalMs, setRequestIntervalMs] = useState("");
  const [cacheTtlSeconds, setCacheTtlSeconds] = useState("");
  const [retryAttempts, setRetryAttempts] = useState("");
  const [retryBackoffMs, setRetryBackoffMs] = useState("");
  const [tvdbEnabled, setTvdbEnabled] = useState(false);
  const [tvdbBaseUrl, setTvdbBaseUrl] = useState("");
  const [tvdbApiKey, setTvdbApiKey] = useState("");
  const [tvdbPin, setTvdbPin] = useState("");
  const [tvdbTimeoutSeconds, setTvdbTimeoutSeconds] = useState("");
  const [bangumiEnabled, setBangumiEnabled] = useState(false);
  const [bangumiBaseUrl, setBangumiBaseUrl] = useState("");
  const [bangumiAccessToken, setBangumiAccessToken] = useState("");
  const [bangumiTimeoutSeconds, setBangumiTimeoutSeconds] = useState("");
  const [bangumiUserAgent, setBangumiUserAgent] = useState("");

  const [saving, setSaving] = useState(false);
  const [togglingEnabled, setTogglingEnabled] = useState(false);
  const [actionLoading, setActionLoading] = useState(false);
  const [triggerLoading, setTriggerLoading] = useState(false);
  const [confirmAction, setConfirmAction] = useState<ConfirmAction>(null);
  const [testingProviderId, setTestingProviderId] = useState<string | null>(null);

  const populateDraft = React.useCallback((payload: WebAppSettings) => {
    setDefaultStrategy(payload.scraper?.default_strategy ?? "primary_with_fallback");
    setProvidersInput((payload.scraper?.providers ?? []).join(", "));
    const nextScenarioInputs: Record<string, string> = {};
    SCRAPER_SCENARIO_KEYS.forEach((scenarioKey) => {
      nextScenarioInputs[scenarioKey] = (
        payload.scraper?.scenario_defaults?.[scenarioKey] ?? []
      ).join(", ");
    });
    setScenarioInputs(nextScenarioInputs);
    setTmdbApiKey(payload.tmdb?.api_key ?? "");
    setTmdbLanguage(payload.tmdb?.language ?? "zh-CN");
    setTimeoutSeconds(String(payload.tmdb?.timeout_seconds ?? 10));
    setRequestIntervalMs(String(payload.tmdb?.request_interval_ms ?? 350));
    setCacheTtlSeconds(String(payload.tmdb?.cache_ttl_seconds ?? 86400));
    setRetryAttempts(String(payload.tmdb?.retry_attempts ?? 3));
    setRetryBackoffMs(String(payload.tmdb?.retry_backoff_ms ?? 2000));
    setTvdbEnabled(Boolean(payload.scraper?.tvdb?.enabled));
    setTvdbBaseUrl(payload.scraper?.tvdb?.base_url ?? "https://api4.thetvdb.com/v4");
    setTvdbApiKey(payload.scraper?.tvdb?.api_key ?? "");
    setTvdbPin(payload.scraper?.tvdb?.pin ?? "");
    setTvdbTimeoutSeconds(String(payload.scraper?.tvdb?.timeout_seconds ?? 15));
    setBangumiEnabled(Boolean(payload.scraper?.bangumi?.enabled));
    setBangumiBaseUrl(payload.scraper?.bangumi?.base_url ?? "https://api.bgm.tv");
    setBangumiAccessToken(payload.scraper?.bangumi?.access_token ?? "");
    setBangumiTimeoutSeconds(String(payload.scraper?.bangumi?.timeout_seconds ?? 15));
    setBangumiUserAgent(payload.scraper?.bangumi?.user_agent ?? "lumenstream/0.1");
  }, []);

  const reload = React.useCallback(async () => {
    setLoading(true);
    try {
      const [nextFlags, nextSummary, scraperSettings, nextProviders, nextCacheStats, nextFailures] =
        await Promise.all([
          getSystemFlags(),
          getSystemSummary(),
          getScraperSettings(),
          listScraperProviders(),
          getScraperCacheStats(),
          listScraperFailures(),
        ]);
      setFlags(nextFlags);
      setSummary(nextSummary);
      setSettings(scraperSettings.settings);
      setProviders(nextProviders);
      setCacheStats(nextCacheStats);
      setFailures(nextFailures);
      populateDraft(scraperSettings.settings);
      setError(null);
    } catch (cause) {
      setError((cause as ApiError).message || "加载失败");
    } finally {
      setLoading(false);
    }
  }, [populateDraft]);

  useEffect(() => {
    if (ready) {
      void reload();
    }
  }, [ready, reload]);

  const metrics = useMemo(
    () => (summary?.infra_metrics ?? {}) as Record<string, number>,
    [summary]
  );

  async function onToggleEnabled() {
    if (!flags) return;
    setTogglingEnabled(true);
    try {
      const updated = await updateSystemFlags({ scraper_enabled: !flags.scraper_enabled });
      const nextProviders = await listScraperProviders();
      setFlags(updated);
      setProviders(nextProviders);
      toast.success(updated.scraper_enabled ? "刮削系统已启用" : "刮削系统已禁用");
    } catch (cause) {
      toast.error(`切换失败：${(cause as ApiError).message}`);
    } finally {
      setTogglingEnabled(false);
    }
  }

  async function onSave() {
    if (!settings) return;
    const scenarioDefaults = Object.fromEntries(
      SCRAPER_SCENARIO_KEYS.map((scenarioKey) => [
        scenarioKey,
        scenarioInputs[scenarioKey]
          ?.split(",")
          .map((item) => item.trim())
          .filter(Boolean) ?? [],
      ])
    );

    setSaving(true);
    try {
      const nextSettings: WebAppSettings = {
        ...settings,
        scraper: {
          ...settings.scraper,
          enabled: flags?.scraper_enabled ?? settings.scraper.enabled,
          default_strategy: defaultStrategy.trim() || "primary_with_fallback",
          providers: providersInput
            .split(",")
            .map((item) => item.trim())
            .filter(Boolean),
          scenario_defaults: scenarioDefaults,
          tvdb: {
            ...settings.scraper.tvdb,
            enabled: tvdbEnabled,
            base_url: tvdbBaseUrl,
            api_key: tvdbApiKey,
            pin: tvdbPin,
            timeout_seconds: Number(tvdbTimeoutSeconds),
          },
          bangumi: {
            ...settings.scraper.bangumi,
            enabled: bangumiEnabled,
            base_url: bangumiBaseUrl,
            access_token: bangumiAccessToken,
            timeout_seconds: Number(bangumiTimeoutSeconds),
            user_agent: bangumiUserAgent,
          },
        },
        tmdb: {
          ...settings.tmdb,
          enabled: flags?.scraper_enabled ?? settings.tmdb.enabled,
          api_key: tmdbApiKey,
          language: tmdbLanguage,
          timeout_seconds: Number(timeoutSeconds),
          request_interval_ms: Number(requestIntervalMs),
          cache_ttl_seconds: Number(cacheTtlSeconds),
          retry_attempts: Number(retryAttempts),
          retry_backoff_ms: Number(retryBackoffMs),
        },
      };
      const result = await upsertScraperSettings({
        settings: nextSettings,
        library_policies: [],
      });
      const nextProviders = await listScraperProviders();
      setSettings(result.settings);
      setProviders(nextProviders);
      populateDraft(result.settings);
      toast.success("刮削系统配置已保存");
    } catch (cause) {
      toast.error(`保存失败：${(cause as ApiError).message}`);
    } finally {
      setSaving(false);
    }
  }

  async function onConfirmAction() {
    if (!confirmAction) return;
    const action = confirmAction;
    setConfirmAction(null);
    setActionLoading(true);
    try {
      if (action === "clear-expired") {
        const result = await clearScraperCache(true);
        toast.success(`已清除 ${result.removed} 条过期缓存`);
      } else if (action === "clear-all") {
        const result = await clearScraperCache(false);
        toast.success(`已清除 ${result.removed} 条缓存`);
      } else {
        const result = await clearScraperFailures();
        toast.success(`已清除 ${result.removed} 条失败记录`);
      }
      const [nextCacheStats, nextFailures] = await Promise.all([
        getScraperCacheStats(),
        listScraperFailures(),
      ]);
      setCacheStats(nextCacheStats);
      setFailures(nextFailures);
    } catch (cause) {
      toast.error(`操作失败：${(cause as ApiError).message}`);
    } finally {
      setActionLoading(false);
    }
  }

  async function onTriggerFill() {
    setTriggerLoading(true);
    try {
      await runTaskNow("scraper_fill");
      toast.success("刮削补齐任务已触发");
    } catch (cause) {
      toast.error(`触发失败：${(cause as ApiError).message}`);
    } finally {
      setTriggerLoading(false);
    }
  }

  async function onTestProvider(providerId: string) {
    setTestingProviderId(providerId);
    try {
      const updated = await testScraperProvider(providerId);
      setProviders((current) =>
        current.map((provider) => (provider.provider_id === providerId ? updated : provider))
      );
      toast.success(`${updated.display_name} 检查完成：${updated.message}`);
    } catch (cause) {
      toast.error(`检测失败：${(cause as ApiError).message}`);
    } finally {
      setTestingProviderId(null);
    }
  }

  if (!ready || loading) return <LoadingState title="加载刮削系统管理" />;
  if (error) return <ErrorState title="刮削系统页加载失败" description={error} />;

  return (
    <div className="space-y-6">
      <section className="rounded-lg border border-white/10 bg-black p-6">
        <div className="flex flex-col gap-6 xl:flex-row xl:items-start xl:justify-between">
          <div className="space-y-3">
            <h3 className="text-2xl font-medium text-white">刮削框架配置</h3>
            <p className="max-w-2xl text-sm text-slate-400">
              统一维护 provider
              顺序、默认场景链、连接健康度与运行状态。媒体库启用什么刮削已迁移到媒体库管理页面逐库配置。
            </p>
            <div className="flex flex-wrap gap-2">
              <Badge variant={flags?.scraper_enabled ? "success" : "outline"}>
                {flags?.scraper_enabled ? "已启用" : "已禁用"}
              </Badge>
              <Badge variant="outline">{providers.length} 个 Provider</Badge>
              <Badge variant="outline">默认：{providersInput || "tmdb"}</Badge>
            </div>
          </div>

          <div className="flex flex-col gap-2 xl:min-w-[280px]">
            <div className="flex gap-2 text-sm">
              <div className="flex-1 rounded border border-white/10 bg-white/5 px-3 py-2">
                <div className="text-xs text-slate-500">健康</div>
                <div className="text-lg font-medium text-white">
                  {providers.filter((provider) => provider.healthy).length}
                </div>
              </div>
              <div className="flex-1 rounded border border-white/10 bg-white/5 px-3 py-2">
                <div className="text-xs text-slate-500">失败</div>
                <div className="text-lg font-medium text-white">{failures.length}</div>
              </div>
            </div>
            <Button
              variant="secondary"
              className="justify-between"
              onClick={onToggleEnabled}
              disabled={togglingEnabled}
            >
              {togglingEnabled
                ? "切换中..."
                : flags?.scraper_enabled
                  ? "禁用全局刮削"
                  : "启用全局刮削"}
            </Button>
            <a
              href="/admin/libraries"
              className="border-primary/20 bg-primary/10 text-primary hover:bg-primary/15 inline-flex items-center justify-center rounded-lg border px-4 py-2 text-sm font-medium transition"
            >
              转到媒体库配置
            </a>
          </div>
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-[1.2fr_0.8fr]">
        <div className="rounded-lg border border-white/10 bg-black p-5">
          <div className="mb-4">
            <h4 className="text-sm font-medium text-white">全局策略</h4>
            <p className="mt-1 text-xs text-slate-500">
              全局策略控制默认链路与 provider 凭据；逐库覆盖请前往媒体库页。
            </p>
          </div>

          <div className="grid gap-4 xl:grid-cols-2">
            <div className="space-y-3 rounded border border-white/5 bg-white/[0.02] p-4">
              <Field label="默认策略">
                <Input
                  value={defaultStrategy}
                  onChange={(event) => setDefaultStrategy(event.target.value)}
                />
              </Field>
              <Field label="全局 Provider 列表 (拖拽排序)">
                <SortableProviderList
                  providers={providers.map((p) => ({ id: p.provider_id, label: p.display_name }))}
                  activeIds={providersInput
                    .split(",")
                    .map((s) => s.trim())
                    .filter(Boolean)
                    .filter((id) => providers.some((p) => p.provider_id === id))}
                  onChange={(activeIds) => setProvidersInput(activeIds.join(", "))}
                />
              </Field>
              <Field label="TMDB API Key">
                <Input
                  type="password"
                  value={tmdbApiKey}
                  onChange={(event) => setTmdbApiKey(event.target.value)}
                  placeholder="输入 TMDB API Key"
                />
              </Field>
              <div className="grid gap-3 sm:grid-cols-2">
                <Field label="语言">
                  <Input
                    value={tmdbLanguage}
                    onChange={(event) => setTmdbLanguage(event.target.value)}
                  />
                </Field>
                <Field label="超时（秒）">
                  <Input
                    value={timeoutSeconds}
                    onChange={(event) => setTimeoutSeconds(event.target.value)}
                  />
                </Field>
                <Field label="请求间隔（ms）">
                  <Input
                    value={requestIntervalMs}
                    onChange={(event) => setRequestIntervalMs(event.target.value)}
                  />
                </Field>
                <Field label="缓存 TTL（秒）">
                  <Input
                    value={cacheTtlSeconds}
                    onChange={(event) => setCacheTtlSeconds(event.target.value)}
                  />
                </Field>
                <Field label="重试次数">
                  <Input
                    value={retryAttempts}
                    onChange={(event) => setRetryAttempts(event.target.value)}
                  />
                </Field>
                <Field label="退避（ms）">
                  <Input
                    value={retryBackoffMs}
                    onChange={(event) => setRetryBackoffMs(event.target.value)}
                  />
                </Field>
              </div>
            </div>

            <div className="space-y-3 rounded border border-white/5 bg-white/[0.02] p-4">
              <div className="flex items-center justify-between">
                <label className="text-xs font-medium text-slate-300">TVDB 启用</label>
                <Switch checked={tvdbEnabled} onCheckedChange={setTvdbEnabled} />
              </div>
              <Field label="TVDB Base URL">
                <Input
                  value={tvdbBaseUrl}
                  onChange={(event) => setTvdbBaseUrl(event.target.value)}
                />
              </Field>
              <Field label="TVDB API Key">
                <Input
                  type="password"
                  value={tvdbApiKey}
                  onChange={(event) => setTvdbApiKey(event.target.value)}
                  placeholder="输入 TVDB API Key"
                />
              </Field>
              <Field label="TVDB PIN">
                <Input
                  type="password"
                  value={tvdbPin}
                  onChange={(event) => setTvdbPin(event.target.value)}
                  placeholder="输入 TVDB PIN"
                />
              </Field>
              <Field label="TVDB 超时（秒）">
                <Input
                  value={tvdbTimeoutSeconds}
                  onChange={(event) => setTvdbTimeoutSeconds(event.target.value)}
                />
              </Field>
              <div className="mt-3 flex items-center justify-between border-t border-white/5 pt-3">
                <label className="text-xs font-medium text-slate-300">Bangumi 启用</label>
                <Switch checked={bangumiEnabled} onCheckedChange={setBangumiEnabled} />
              </div>
              <Field label="Bangumi Base URL">
                <Input
                  value={bangumiBaseUrl}
                  onChange={(event) => setBangumiBaseUrl(event.target.value)}
                />
              </Field>
              <Field label="Bangumi Access Token">
                <Input
                  type="password"
                  value={bangumiAccessToken}
                  onChange={(event) => setBangumiAccessToken(event.target.value)}
                  placeholder="输入 Bangumi Access Token"
                />
              </Field>
              <Field label="Bangumi 超时（秒）">
                <Input
                  value={bangumiTimeoutSeconds}
                  onChange={(event) => setBangumiTimeoutSeconds(event.target.value)}
                />
              </Field>
              <Field label="Bangumi User-Agent">
                <Input
                  value={bangumiUserAgent}
                  onChange={(event) => setBangumiUserAgent(event.target.value)}
                />
              </Field>
            </div>
          </div>
        </div>

        <div className="rounded-lg border border-white/10 bg-black p-5">
          <div className="mb-4">
            <h4 className="text-sm font-medium text-white">场景路由</h4>
            <p className="mt-1 text-xs text-slate-500">
              每个场景按默认 provider chain 依次回退；库级链路请在媒体库页定制。
            </p>
          </div>
          <div className="grid gap-4 sm:grid-cols-2">
            {SCRAPER_SCENARIO_KEYS.map((scenarioKey) => {
              const currentChainRaw = scenarioInputs[scenarioKey] ?? "";
              const currentChain = currentChainRaw
                .split(",")
                .map((s) => s.trim())
                .filter(Boolean);

              return (
                <div
                  key={scenarioKey}
                  className="rounded border border-white/5 bg-white/[0.02] p-4"
                >
                  <Field label={scenarioKey}>
                    <SortableProviderList
                      providers={providers.map((p) => ({
                        id: p.provider_id,
                        label: p.display_name,
                      }))}
                      activeIds={currentChain.filter((id) =>
                        providers.some((p) => p.provider_id === id)
                      )}
                      onChange={(activeIds) => {
                        setScenarioInputs((current) => ({
                          ...current,
                          [scenarioKey]: activeIds.join(", "),
                        }));
                      }}
                    />
                  </Field>
                </div>
              );
            })}
          </div>
          <div className="mt-4 flex flex-wrap gap-2">
            <Button onClick={onSave} disabled={saving}>
              {saving ? "保存中..." : "保存配置"}
            </Button>
            <Button variant="outline" onClick={onTriggerFill} disabled={triggerLoading}>
              {triggerLoading ? "触发中..." : "触发刮削"}
            </Button>
          </div>
        </div>
      </section>

      <section className="rounded-lg border border-white/10 bg-black p-5">
        <div className="mb-4">
          <h3 className="text-sm font-medium text-white">Provider 健康状态</h3>
          <p className="mt-1 text-xs text-slate-500">
            展示当前已注册 Provider 的能力、启用状态和检查结果。
          </p>
        </div>
        <div className="grid gap-3 lg:grid-cols-2">
          {providers.map((provider) => (
            <div
              key={provider.provider_id}
              className="rounded border border-white/5 bg-white/[0.02] p-4"
            >
              <div className="flex items-start justify-between gap-3">
                <div>
                  <p className="text-sm font-medium text-white">{provider.display_name}</p>
                  <p className="mt-1 text-xs text-slate-500">{provider.provider_kind}</p>
                </div>
                <Badge
                  variant={
                    provider.healthy ? "success" : provider.enabled ? "outline" : "secondary"
                  }
                >
                  {provider.healthy ? "Healthy" : provider.enabled ? "待配置" : "Disabled"}
                </Badge>
              </div>
              <p className="mt-3 text-xs text-slate-400">{provider.message}</p>
              <p className="mt-2 text-xs text-slate-500">
                能力：{provider.capabilities.join(", ") || "-"}
              </p>
              <p className="mt-1 text-xs text-slate-500">
                场景：{provider.scenarios.join(", ") || "-"}
              </p>
              <div className="mt-3">
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => onTestProvider(provider.provider_id)}
                  disabled={testingProviderId === provider.provider_id}
                >
                  {testingProviderId === provider.provider_id ? "检测中..." : "测试连接"}
                </Button>
              </div>
            </div>
          ))}
        </div>
      </section>

      <section className="rounded-lg border border-white/10 bg-black p-5">
        <h3 className="mb-4 text-sm font-medium text-white">运行指标</h3>
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <Metric
            label="HTTP 请求数"
            value={metrics.scraper_http_requests_total ?? metrics.tmdb_http_requests_total ?? 0}
          />
          <Metric
            label="缓存命中"
            value={metrics.scraper_cache_hits_total ?? metrics.tmdb_cache_hits_total ?? 0}
          />
          <Metric
            label="缓存未命中"
            value={metrics.scraper_cache_misses_total ?? metrics.tmdb_cache_misses_total ?? 0}
          />
          <Metric
            label="命中率"
            value={`${(((metrics.scraper_hit_rate ?? metrics.tmdb_hit_rate ?? 0) as number) * 100).toFixed(1)}%`}
          />
          <Metric
            label="成功数"
            value={metrics.scraper_success_total ?? metrics.tmdb_success_total ?? 0}
          />
          <Metric
            label="失败数"
            value={metrics.scraper_failure_total ?? metrics.tmdb_failure_total ?? 0}
          />
        </div>
      </section>

      <section className="rounded-lg border border-white/10 bg-black p-5">
        <h3 className="mb-4 text-sm font-medium text-white">缓存与失败记录</h3>
        {cacheStats && (
          <div className="mb-4 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            <Metric label="总缓存条目" value={cacheStats.total_entries} />
            <Metric label="有结果条目" value={cacheStats.entries_with_result} />
            <Metric label="已过期条目" value={cacheStats.expired_entries} />
            <Metric label="总命中次数" value={cacheStats.total_hits} />
          </div>
        )}
        <div className="mb-4 flex flex-wrap gap-2">
          <Button
            variant="secondary"
            onClick={() => setConfirmAction("clear-expired")}
            disabled={actionLoading}
          >
            清除过期缓存
          </Button>
          <Button
            variant="destructive"
            onClick={() => setConfirmAction("clear-all")}
            disabled={actionLoading}
          >
            清除全部缓存
          </Button>
          <Button
            variant="outline"
            onClick={() => setConfirmAction("clear-failures")}
            disabled={actionLoading || failures.length === 0}
          >
            清除失败记录
          </Button>
        </div>
        {failures.length > 0 ? (
          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>名称</TableHead>
                  <TableHead>类型</TableHead>
                  <TableHead>尝试次数</TableHead>
                  <TableHead>错误</TableHead>
                  <TableHead>时间</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {failures.map((failure) => (
                  <TableRow key={failure.id}>
                    <TableCell className="max-w-[220px] truncate">{failure.item_name}</TableCell>
                    <TableCell>{failure.item_type}</TableCell>
                    <TableCell>{failure.attempts}</TableCell>
                    <TableCell className="text-muted-foreground max-w-[320px] truncate text-xs">
                      {failure.error}
                    </TableCell>
                    <TableCell className="text-muted-foreground text-xs whitespace-nowrap">
                      {new Date(failure.created_at).toLocaleString()}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        ) : (
          <p className="text-muted-foreground text-xs">暂无失败记录。</p>
        )}
      </section>

      <Modal
        open={confirmAction !== null}
        title={
          confirmAction === "clear-expired"
            ? "确认清除过期缓存"
            : confirmAction === "clear-all"
              ? "确认清除全部缓存"
              : "确认清除失败记录"
        }
        description="该操作不可逆，请确认后继续。"
        onClose={() => setConfirmAction(null)}
        showHeaderClose
        showFooterClose={false}
      >
        <div className="flex justify-end gap-2 pt-2">
          <Button variant="secondary" onClick={() => setConfirmAction(null)}>
            取消
          </Button>
          <Button variant="destructive" onClick={() => void onConfirmAction()}>
            确认
          </Button>
        </div>
      </Modal>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <label className="text-muted-foreground mb-1 block text-xs">{label}</label>
      {children}
    </div>
  );
}

function Metric({ label, value }: { label: string; value: number | string }) {
  return (
    <div className="rounded border border-white/5 bg-white/[0.02] px-4 py-3">
      <p className="text-xs text-slate-500">{label}</p>
      <p className="mt-1 text-xl font-medium text-white">{String(value)}</p>
    </div>
  );
}
