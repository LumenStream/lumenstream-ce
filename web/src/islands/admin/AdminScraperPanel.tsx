import React, { useEffect, useMemo, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { ChevronDown, ChevronUp } from "lucide-react";

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
import {
  SCRAPER_DEFAULT_ROUTE_KEYS,
  getScraperDefaultRouteLabel,
} from "@/lib/admin/scraper-policy";
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
    SCRAPER_DEFAULT_ROUTE_KEYS.forEach((routeKey) => {
      nextScenarioInputs[routeKey] = (payload.scraper?.default_routes?.[routeKey] ?? []).join(", ");
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
    const defaultRoutes = Object.fromEntries(
      SCRAPER_DEFAULT_ROUTE_KEYS.map((routeKey) => [
        routeKey,
        scenarioInputs[routeKey]
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
          default_routes: defaultRoutes as WebAppSettings["scraper"]["default_routes"],
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
    <div className="space-y-8">
      <section className="rounded-xl border border-white/10 bg-black/60 p-6 shadow-sm backdrop-blur-sm">
        <div className="flex flex-col gap-6 xl:flex-row xl:items-start xl:justify-between">
          <div className="space-y-3">
            <h3 className="text-2xl font-semibold text-white">刮削框架配置</h3>
            <p className="max-w-2xl text-sm leading-relaxed text-slate-400">
              统一维护 provider 顺序、默认场景链、连接健康度与运行状态。
              媒体库级别的刮削配置已迁移到媒体库管理页面。
            </p>
            <div className="flex flex-wrap gap-2 pt-1">
              <Badge variant={flags?.scraper_enabled ? "success" : "outline"} className="px-3">
                {flags?.scraper_enabled ? "已启用" : "已禁用"}
              </Badge>
              <Badge variant="outline" className="px-3">
                {providers.length} 个 Provider
              </Badge>
              <Badge variant="outline" className="px-3">
                默认：{providersInput || "tmdb"}
              </Badge>
            </div>
          </div>

          <div className="flex flex-col gap-3 xl:min-w-[300px]">
            <div className="flex gap-3 text-sm">
              <div className="flex-1 rounded-lg border border-white/10 bg-white/5 px-4 py-3 shadow-inner">
                <div className="mb-1 text-xs font-medium text-slate-400">健康节点</div>
                <div className="text-2xl font-semibold text-emerald-400">
                  {providers.filter((provider) => provider.healthy).length}
                </div>
              </div>
              <div className="flex-1 rounded-lg border border-white/10 bg-white/5 px-4 py-3 shadow-inner">
                <div className="mb-1 text-xs font-medium text-slate-400">失败记录</div>
                <div className="text-2xl font-semibold text-rose-400">{failures.length}</div>
              </div>
            </div>
            <div className="flex gap-2">
              <Button
                variant={flags?.scraper_enabled ? "secondary" : "default"}
                className="flex-1"
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
                className="border-primary/30 bg-primary/10 text-primary hover:bg-primary/20 inline-flex flex-1 items-center justify-center rounded-md border px-4 py-2 text-sm font-medium transition-colors"
              >
                管理媒体库
              </a>
            </div>
          </div>
        </div>
      </section>

      <div className="grid gap-6 xl:grid-cols-[1fr_360px] 2xl:grid-cols-[1.3fr_0.7fr]">
        <div className="space-y-5">
          <div className="flex items-center justify-between px-1">
            <h2 className="text-lg font-medium text-white">全局策略</h2>
            <div className="flex items-center gap-2">
              <Button onClick={onSave} disabled={saving} size="sm">
                {saving ? "保存中..." : "保存全部配置"}
              </Button>
              <Button variant="outline" size="sm" onClick={onTriggerFill} disabled={triggerLoading}>
                {triggerLoading ? "触发中..." : "触发补齐"}
              </Button>
            </div>
          </div>

          <CollapsibleCard
            title="基础策略与默认提供者"
            description="配置默认的刮削策略和全局 Provider 优先级"
            defaultOpen={true}
          >
            <div className="grid gap-6 sm:grid-cols-[1fr_1.5fr]">
              <Field label="默认策略">
                <Input
                  value={defaultStrategy}
                  onChange={(event) => setDefaultStrategy(event.target.value)}
                  placeholder="例如: primary_with_fallback"
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
            </div>
          </CollapsibleCard>

          <CollapsibleCard
            title="TMDB 配置"
            description="The Movie Database 核心刮削源"
            defaultOpen={true}
          >
            <div className="space-y-4">
              <Field label="API Key">
                <Input
                  type="password"
                  value={tmdbApiKey}
                  onChange={(event) => setTmdbApiKey(event.target.value)}
                  placeholder="输入 TMDB API Key"
                />
              </Field>
              <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                <Field label="语言">
                  <Input
                    value={tmdbLanguage}
                    onChange={(event) => setTmdbLanguage(event.target.value)}
                  />
                </Field>
                <Field label="超时（秒）">
                  <Input
                    type="number"
                    value={timeoutSeconds}
                    onChange={(event) => setTimeoutSeconds(event.target.value)}
                  />
                </Field>
                <Field label="请求间隔（ms）">
                  <Input
                    type="number"
                    value={requestIntervalMs}
                    onChange={(event) => setRequestIntervalMs(event.target.value)}
                  />
                </Field>
                <Field label="缓存 TTL（秒）">
                  <Input
                    type="number"
                    value={cacheTtlSeconds}
                    onChange={(event) => setCacheTtlSeconds(event.target.value)}
                  />
                </Field>
                <Field label="重试次数">
                  <Input
                    type="number"
                    value={retryAttempts}
                    onChange={(event) => setRetryAttempts(event.target.value)}
                  />
                </Field>
                <Field label="退避（ms）">
                  <Input
                    type="number"
                    value={retryBackoffMs}
                    onChange={(event) => setRetryBackoffMs(event.target.value)}
                  />
                </Field>
              </div>
            </div>
          </CollapsibleCard>

          <CollapsibleCard
            title="TVDB 配置"
            description="TheTVDB 剧集刮削源"
            headerAction={
              <div className="flex items-center gap-2">
                <span className="text-xs font-medium text-slate-400">启用</span>
                <Switch checked={tvdbEnabled} onCheckedChange={setTvdbEnabled} />
              </div>
            }
          >
            <div className="space-y-4">
              <Field label="Base URL">
                <Input
                  value={tvdbBaseUrl}
                  onChange={(event) => setTvdbBaseUrl(event.target.value)}
                />
              </Field>
              <div className="grid gap-4 sm:grid-cols-2">
                <Field label="API Key">
                  <Input
                    type="password"
                    value={tvdbApiKey}
                    onChange={(event) => setTvdbApiKey(event.target.value)}
                    placeholder="输入 TVDB API Key"
                  />
                </Field>
                <Field label="PIN">
                  <Input
                    type="password"
                    value={tvdbPin}
                    onChange={(event) => setTvdbPin(event.target.value)}
                    placeholder="输入 TVDB PIN"
                  />
                </Field>
              </div>
              <div className="max-w-[200px]">
                <Field label="超时（秒）">
                  <Input
                    type="number"
                    value={tvdbTimeoutSeconds}
                    onChange={(event) => setTvdbTimeoutSeconds(event.target.value)}
                  />
                </Field>
              </div>
            </div>
          </CollapsibleCard>

          <CollapsibleCard
            title="Bangumi 配置"
            description="番组计划 (bgm.tv) 动漫刮削源"
            headerAction={
              <div className="flex items-center gap-2">
                <span className="text-xs font-medium text-slate-400">启用</span>
                <Switch checked={bangumiEnabled} onCheckedChange={setBangumiEnabled} />
              </div>
            }
          >
            <div className="space-y-4">
              <Field label="Base URL">
                <Input
                  value={bangumiBaseUrl}
                  onChange={(event) => setBangumiBaseUrl(event.target.value)}
                />
              </Field>
              <div className="grid gap-4 sm:grid-cols-2">
                <Field label="Access Token">
                  <Input
                    type="password"
                    value={bangumiAccessToken}
                    onChange={(event) => setBangumiAccessToken(event.target.value)}
                    placeholder="输入 Bangumi Access Token"
                  />
                </Field>
                <Field label="超时（秒）">
                  <Input
                    type="number"
                    value={bangumiTimeoutSeconds}
                    onChange={(event) => setBangumiTimeoutSeconds(event.target.value)}
                  />
                </Field>
              </div>
              <Field label="User-Agent">
                <Input
                  value={bangumiUserAgent}
                  onChange={(event) => setBangumiUserAgent(event.target.value)}
                />
              </Field>
            </div>
          </CollapsibleCard>
        </div>

        <div className="space-y-5">
          <div className="mb-2 px-1">
            <h2 className="text-lg font-medium text-white">场景路由</h2>
            <p className="mt-1 text-xs text-slate-500">
              默认按电影、电视剧、图像三类区分 Provider 链路，拖拽排序调整优先级。
            </p>
          </div>

          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-1">
            {SCRAPER_DEFAULT_ROUTE_KEYS.map((scenarioKey) => {
              const currentChainRaw = scenarioInputs[scenarioKey] ?? "";
              const currentChain = currentChainRaw
                .split(",")
                .map((s) => s.trim())
                .filter(Boolean);

              return (
                <div
                  key={scenarioKey}
                  className="rounded-lg border border-white/10 bg-black/40 p-4 transition-colors hover:bg-black/60"
                >
                  <Field label={getScraperDefaultRouteLabel(scenarioKey)}>
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
        </div>
      </div>

      <div className="grid gap-6 xl:grid-cols-2">
        <SectionCard
          title="Provider 健康状态"
          description="当前已注册 Provider 的能力、启用状态和检查结果。"
        >
          <div className="grid gap-4 sm:grid-cols-2">
            {providers.map((provider) => (
              <div
                key={provider.provider_id}
                className="flex flex-col justify-between rounded-lg border border-white/10 bg-white/[0.02] p-4 transition-colors hover:bg-white/[0.04]"
              >
                <div>
                  <div className="mb-2 flex items-start justify-between gap-3">
                    <div>
                      <p className="text-sm font-medium text-white">{provider.display_name}</p>
                      <p className="text-xs text-slate-500">{provider.provider_kind}</p>
                    </div>
                    <Badge
                      variant={
                        provider.healthy ? "success" : provider.enabled ? "outline" : "secondary"
                      }
                    >
                      {provider.healthy ? "Healthy" : provider.enabled ? "待配置" : "Disabled"}
                    </Badge>
                  </div>
                  <p className="line-clamp-2 min-h-[32px] text-xs text-slate-400">
                    {provider.message}
                  </p>

                  <div className="mt-4 space-y-1.5">
                    <p className="flex justify-between text-xs text-slate-500">
                      <span>能力:</span>
                      <span
                        className="max-w-[140px] truncate text-right text-slate-300"
                        title={provider.capabilities.join(", ")}
                      >
                        {provider.capabilities.join(", ") || "-"}
                      </span>
                    </p>
                    <p className="flex justify-between text-xs text-slate-500">
                      <span>场景:</span>
                      <span
                        className="max-w-[140px] truncate text-right text-slate-300"
                        title={provider.scenarios.join(", ")}
                      >
                        {provider.scenarios.join(", ") || "-"}
                      </span>
                    </p>
                  </div>
                </div>
                <div className="mt-4 border-t border-white/10 pt-4">
                  <Button
                    variant="secondary"
                    size="sm"
                    className="w-full"
                    onClick={() => onTestProvider(provider.provider_id)}
                    disabled={testingProviderId === provider.provider_id}
                  >
                    {testingProviderId === provider.provider_id ? "检测中..." : "测试连接"}
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </SectionCard>

        <SectionCard title="运行指标" description="刮削系统 API 调用的实时统计数据。">
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
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
        </SectionCard>
      </div>

      <SectionCard title="缓存与失败记录" description="管理刮削缓存和查看近期错误。">
        {cacheStats && (
          <div className="mb-6 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            <Metric label="总缓存条目" value={cacheStats.total_entries} />
            <Metric label="有结果条目" value={cacheStats.entries_with_result} />
            <Metric label="已过期条目" value={cacheStats.expired_entries} />
            <Metric label="总命中次数" value={cacheStats.total_hits} />
          </div>
        )}
        <div className="mb-4 flex flex-wrap items-center gap-3 border-b border-white/10 pb-4">
          <Button
            variant="secondary"
            size="sm"
            onClick={() => setConfirmAction("clear-expired")}
            disabled={actionLoading}
          >
            清除过期缓存
          </Button>
          <Button
            variant="destructive"
            size="sm"
            onClick={() => setConfirmAction("clear-all")}
            disabled={actionLoading}
          >
            清除全部缓存
          </Button>
          <div className="flex-1" />
          <Button
            variant="outline"
            size="sm"
            onClick={() => setConfirmAction("clear-failures")}
            disabled={actionLoading || failures.length === 0}
          >
            清除失败记录
          </Button>
        </div>

        {failures.length > 0 ? (
          <div className="overflow-x-auto rounded-md border border-white/10">
            <Table>
              <TableHeader className="bg-white/5">
                <TableRow className="border-white/10 hover:bg-transparent">
                  <TableHead className="font-medium text-slate-300">名称</TableHead>
                  <TableHead className="font-medium text-slate-300">类型</TableHead>
                  <TableHead className="font-medium text-slate-300">尝试次数</TableHead>
                  <TableHead className="font-medium text-slate-300">错误</TableHead>
                  <TableHead className="text-right font-medium text-slate-300">时间</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {failures.map((failure) => (
                  <TableRow key={failure.id} className="border-white/10 hover:bg-white/[0.02]">
                    <TableCell className="max-w-[220px] truncate font-medium">
                      {failure.item_name}
                    </TableCell>
                    <TableCell>
                      <Badge variant="outline" className="py-0 text-[10px] font-normal">
                        {failure.item_type}
                      </Badge>
                    </TableCell>
                    <TableCell>{failure.attempts}</TableCell>
                    <TableCell className="max-w-[320px] truncate text-xs text-slate-400">
                      {failure.error}
                    </TableCell>
                    <TableCell className="text-right text-xs whitespace-nowrap text-slate-500">
                      {new Date(failure.created_at).toLocaleString()}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        ) : (
          <div className="rounded-lg border border-dashed border-white/10 bg-white/[0.01] py-8 text-center">
            <p className="text-sm text-slate-500">暂无失败记录</p>
          </div>
        )}
      </SectionCard>

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

function CollapsibleCard({
  title,
  description,
  defaultOpen = false,
  headerAction,
  children,
}: {
  title: string;
  description?: string;
  defaultOpen?: boolean;
  headerAction?: React.ReactNode;
  children: React.ReactNode;
}) {
  const [isOpen, setIsOpen] = useState(defaultOpen);

  return (
    <div className="overflow-hidden rounded-lg border border-white/10 bg-black shadow-sm">
      <div
        className="flex cursor-pointer items-center justify-between p-5 transition-colors select-none hover:bg-white/[0.02]"
        onClick={() => setIsOpen(!isOpen)}
      >
        <div className="flex-1 pr-4">
          <div className="flex items-center gap-3">
            <h3 className="text-sm font-semibold text-white">{title}</h3>
            {headerAction && <div onClick={(e) => e.stopPropagation()}>{headerAction}</div>}
          </div>
          {description && <p className="mt-1.5 text-xs text-slate-500">{description}</p>}
        </div>
        <div className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-md text-slate-500 transition-colors hover:bg-white/10">
          {isOpen ? <ChevronUp size={18} /> : <ChevronDown size={18} />}
        </div>
      </div>
      <AnimatePresence initial={false}>
        {isOpen && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.2, ease: "easeInOut" }}
          >
            <div className="border-t border-white/5 p-5 pt-0">
              <div className="pt-4">{children}</div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

function SectionCard({
  title,
  description,
  headerAction,
  children,
  className,
}: {
  title: string;
  description?: string;
  headerAction?: React.ReactNode;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <section
      className={`rounded-lg border border-white/10 bg-black p-5 shadow-sm ${className || ""}`}
    >
      <div className="mb-5 flex items-start justify-between gap-4">
        <div>
          <h3 className="text-sm font-semibold text-white">{title}</h3>
          {description && <p className="mt-1.5 text-xs text-slate-500">{description}</p>}
        </div>
        {headerAction && <div>{headerAction}</div>}
      </div>
      {children}
    </section>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-1.5">
      <label className="block text-xs font-medium text-slate-400">{label}</label>
      {children}
    </div>
  );
}

function Metric({ label, value }: { label: string; value: number | string }) {
  return (
    <div className="rounded-lg border border-white/5 bg-white/[0.02] px-4 py-3 transition-colors hover:bg-white/[0.04]">
      <p className="text-xs font-medium text-slate-500">{label}</p>
      <p className="mt-2 text-2xl font-semibold text-white">{String(value)}</p>
    </div>
  );
}
