import React, { useCallback, useEffect, useState } from "react";
import {
  ChevronLeft,
  ChevronRight,
  Settings,
  Activity,
  AlertCircle,
  CheckCircle2,
  RotateCw,
} from "lucide-react";

import { EmptyState, ErrorState, LoadingState } from "@/components/domain/DataState";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  adminGetAgentSettings,
  adminListAgentProviders,
  adminGetRequest,
  getAdminRequestsWebSocketUrl,
  getRequestsWebSocketToken,
  adminListRequests,
  adminRetryRequest,
  adminReviewRequest,
  adminTestMoviePilot,
  adminUpdateAgentSettings,
} from "@/lib/api/requests";
import type { ApiError } from "@/lib/api/client";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import { toast } from "@/lib/notifications/toast-store";
import type {
  AgentProviderStatus,
  AgentRequest,
  AgentRequestDetail,
  AgentReviewRequest,
  AgentSettings,
  AgentWorkflowStepState,
} from "@/lib/types/requests";
import { cn, formatDate, formatRelativeTime } from "@/lib/utils";

const REQUEST_TYPE_LABELS: Record<string, string> = {
  intake: "智能受理",
  media_request: "求片 / 求剧",
  replace_source: "换源",
  feedback: "反馈",
  missing_episode: "缺集",
  missing_season: "漏季",
};

const REVIEW_ACTIONS: Array<{ value: AgentReviewRequest["action"]; label: string }> = [
  { value: "approve", label: "批准并重试" },
  { value: "manual_complete", label: "手动完成" },
  { value: "ignore", label: "忽略" },
  { value: "reject", label: "拒绝" },
];

function requestStatusVariant(
  status: string
): "glass" | "success" | "danger" | "secondary" | "outline" | "default" {
  if (status === "completed") return "success";
  if (status === "failed" || status === "rejected") return "danger";
  if (status === "review_required") return "secondary";
  return "outline";
}

function requestTypeLabel(type: string): string {
  return REQUEST_TYPE_LABELS[type] ?? type;
}

function hasObjectContent(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && Object.keys(value as object).length > 0;
}

function makeDefaultAgentSettings(): AgentSettings {
  return {
    enabled: true,
    auto_mode: "automatic",
    max_rounds: 10,
    question_timeout_minutes: 1440,
    missing_scan_enabled: true,
    missing_scan_cron: "0 */30 * * * *",
    auto_close_on_library_hit: true,
    review_required_on_parse_ambiguity: true,
    feedback_auto_route: true,
    llm: {
      enabled: false,
      base_url: "https://api.openai.com/v1",
      api_key: "",
      model: "gpt-4o-mini",
    },
    moviepilot: {
      enabled: true,
      base_url: "",
      username: "",
      password: "",
      timeout_seconds: 20,
      search_download_enabled: true,
      subscribe_fallback_enabled: true,
      filter: {
        min_seeders: 5,
        max_movie_size_gb: 35,
        max_episode_size_gb: 5,
        preferred_resource_pix: ["2160P", "4K", "1080P"],
        preferred_video_encode: ["X265", "H265", "X264"],
        preferred_resource_type: ["WEB-DL", "BluRay"],
        preferred_labels: ["中字", "中文"],
        excluded_keywords: ["CAM", "TS", "TC"],
      },
    },
  };
}

export function AdminRequestsPanel() {
  const { ready } = useAuthSession({ requireAdmin: true });
  const [requests, setRequests] = useState<AgentRequest[]>([]);
  const [detail, setDetail] = useState<AgentRequestDetail | null>(null);
  const [settings, setSettings] = useState<AgentSettings>(makeDefaultAgentSettings());
  const [providers, setProviders] = useState<AgentProviderStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [detailLoading, setDetailLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [statusFilter, setStatusFilter] = useState("");
  const [note, setNote] = useState("");
  const [acting, setActing] = useState<string | null>(null);
  const [savingSettings, setSavingSettings] = useState(false);
  const [testingMoviePilot, setTestingMoviePilot] = useState(false);
  const [activeTab, setActiveTab] = useState<"requests" | "settings">("requests");

  const onSelect = useCallback(async (requestId: string) => {
    setDetailLoading(true);
    setDetail(null);
    try {
      const payload = await adminGetRequest(requestId);
      setDetail(payload);
      setNote(payload.request.admin_note || "");
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(apiError.message || "加载请求详情失败");
    } finally {
      setDetailLoading(false);
    }
  }, []);

  const reload = useCallback(
    async (statusAdmin = statusFilter) => {
      setLoading(true);
      try {
        const [requestPayload, settingsPayload] = await Promise.all([
          adminListRequests({ limit: 200, status_admin: statusAdmin || undefined }),
          adminGetAgentSettings(),
        ]);
        const providerPayload = await adminListAgentProviders();
        setRequests(requestPayload);
        setSettings(settingsPayload);
        setProviders(providerPayload);
        setError(null);
      } catch (cause) {
        const apiError = cause as ApiError;
        setError(apiError.message || "加载 Agent 面板失败");
      } finally {
        setLoading(false);
      }
    },
    [statusFilter]
  );

  useEffect(() => {
    if (!ready) {
      return;
    }
    void reload("");
  }, [ready, reload]);

  useEffect(() => {
    if (!ready || typeof WebSocket === "undefined") {
      return;
    }
    const token = getRequestsWebSocketToken();
    if (!token) {
      return;
    }
    let ws: WebSocket | null = null;
    let closed = false;
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
    let reconnectDelay = 1000;

    const connect = () => {
      if (closed) return;
      ws = new WebSocket(getAdminRequestsWebSocketUrl(token));
      ws.onopen = () => {
        reconnectDelay = 1000;
      };
      ws.onmessage = (event) => {
        try {
          const parsed = JSON.parse(event.data) as { request_id?: string };
          void reload(statusFilter);
          if (parsed.request_id && detail?.request.id === parsed.request_id) {
            void onSelect(parsed.request_id);
          }
        } catch {
          // Ignore malformed websocket events.
        }
      };
      ws.onerror = () => ws?.close();
      ws.onclose = () => {
        if (closed) return;
        reconnectTimer = setTimeout(connect, reconnectDelay);
        reconnectDelay = Math.min(reconnectDelay * 2, 30000);
      };
    };

    connect();
    return () => {
      closed = true;
      if (reconnectTimer) clearTimeout(reconnectTimer);
      ws?.close();
    };
  }, [detail?.request.id, onSelect, ready, reload, statusFilter]);

  async function onReview(action: AgentReviewRequest["action"]) {
    if (!detail) return;
    setActing(action);
    try {
      const payload = await adminReviewRequest(detail.request.id, { action, note });
      setDetail(payload);
      setRequests((current) =>
        current.map((item) => (item.id === payload.request.id ? payload.request : item))
      );
      toast.success("操作已提交");
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(apiError.message || "操作失败");
    } finally {
      setActing(null);
    }
  }

  async function onRetry() {
    if (!detail) return;
    setActing("approve");
    try {
      const payload = await adminRetryRequest(detail.request.id);
      setDetail(payload);
      setRequests((current) =>
        current.map((item) => (item.id === payload.request.id ? payload.request : item))
      );
      toast.success("已重新触发 Agent");
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(apiError.message || "重试失败");
    } finally {
      setActing(null);
    }
  }

  async function onSaveSettings() {
    setSavingSettings(true);
    try {
      const payload = await adminUpdateAgentSettings(settings);
      setSettings(payload);
      toast.success("Agent 设置已保存");
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(apiError.message || "保存设置失败");
    } finally {
      setSavingSettings(false);
    }
  }

  async function onTestMoviePilot() {
    setTestingMoviePilot(true);
    try {
      const result = await adminTestMoviePilot(settings);
      toast.success(`MoviePilot 连接成功：${String(result.base_url || "")}`);
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(apiError.message || "MoviePilot 测试失败");
    } finally {
      setTestingMoviePilot(false);
    }
  }

  if (!ready || loading) {
    return <LoadingState title="加载求片 Agent 面板" />;
  }

  if (error) {
    return <ErrorState title="Agent 面板加载失败" description={error} />;
  }

  if (activeTab === "requests" && (detail || detailLoading)) {
    return (
      <div className="animate-in fade-in slide-in-from-bottom-4 mx-auto max-w-5xl space-y-6 duration-300">
        <Button
          variant="ghost"
          onClick={() => setDetail(null)}
          className="text-muted-foreground hover:text-foreground mb-2 -ml-4"
        >
          <ChevronLeft className="mr-1 h-4 w-4" /> 返回工单列表
        </Button>

        {detailLoading ? (
          <div className="py-12">
            <LoadingState title="加载工单详情" />
          </div>
        ) : detail ? (
          <div className="space-y-8">
            <div className="flex flex-col justify-between gap-4 sm:flex-row sm:items-start">
              <div>
                <h2 className="text-2xl font-bold tracking-tight">{detail.request.title}</h2>
                <div className="text-muted-foreground mt-3 flex flex-wrap items-center gap-3 text-sm">
                  <Badge variant="outline" className="font-normal">
                    {requestTypeLabel(detail.request.request_type)}
                  </Badge>
                  <span>·</span>
                  <span>{formatDate(detail.request.created_at)}</span>
                  <span>·</span>
                  <span>阶段: {detail.request.agent_stage}</span>
                  <span>·</span>
                  <Badge variant={requestStatusVariant(detail.request.status_admin)}>
                    {detail.request.status_admin}
                  </Badge>
                </div>
              </div>
              <Button variant="outline" onClick={() => void onRetry()} disabled={acting !== null}>
                <RotateCw className={cn("mr-2 h-4 w-4", acting === "approve" && "animate-spin")} />
                {acting === "approve" ? "处理中..." : "重新触发"}
              </Button>
            </div>

            <div className="grid grid-cols-2 gap-4 sm:grid-cols-3">
              <Metric label="用户状态" value={detail.request.status_user} />
              <Metric label="自动处理" value={detail.request.auto_handled ? "是" : "否"} />
              <Metric label="TMDB ID" value={String(detail.request.tmdb_id ?? "-")} />
            </div>

            <div className="grid gap-6 md:grid-cols-2">
              <div className="space-y-6">
                <div className="bg-card rounded-xl border p-5 shadow-sm">
                  <div className="mb-4 flex items-center justify-between">
                    <h3 className="font-semibold">工作流上下文</h3>
                    <Badge variant="secondary" className="font-normal">
                      {detail.workflow_kind}
                    </Badge>
                  </div>
                  <div className="grid gap-2 sm:grid-cols-2">
                    {detail.workflow_steps.map((step) => (
                      <div
                        key={step.step}
                        className={cn("rounded-lg border p-3", workflowStepTone(step))}
                      >
                        <p className="text-sm font-medium">{step.label}</p>
                        <p className="mt-1 text-[10px] font-bold tracking-wider uppercase opacity-80">
                          {step.status}
                        </p>
                      </div>
                    ))}
                  </div>

                  <div className="border-border mt-6 border-t pt-4">
                    <p className="text-muted-foreground text-xs tracking-wider uppercase">
                      Required Capabilities
                    </p>
                    <p className="mt-1 text-sm font-medium">
                      {detail.required_capabilities.join(", ") || "-"}
                    </p>
                  </div>
                </div>

                <div className="bg-card rounded-xl border p-5 shadow-sm">
                  <h3 className="mb-4 font-semibold">事件时间线</h3>
                  <div className="space-y-4">
                    {detail.events.map((eventItem) => (
                      <div
                        key={eventItem.id}
                        className="border-muted relative border-l-2 pb-4 pl-4 last:pb-0"
                      >
                        <div className="bg-primary/50 absolute top-1 -left-[5px] h-2 w-2 rounded-full" />
                        <div className="flex flex-col justify-between gap-1 sm:flex-row sm:items-center">
                          <p className="text-sm font-medium">{eventItem.summary}</p>
                          <span className="text-muted-foreground text-xs">
                            {formatDate(eventItem.created_at)}
                          </span>
                        </div>
                        <p className="text-muted-foreground mt-1 text-[11px] font-medium tracking-wider uppercase">
                          {eventItem.event_type}
                        </p>
                        {Object.keys(eventItem.detail || {}).length > 0 && (
                          <pre className="bg-muted/50 text-muted-foreground mt-2 overflow-x-auto rounded-lg p-3 text-xs">
                            {JSON.stringify(eventItem.detail, null, 2)}
                          </pre>
                        )}
                      </div>
                    ))}
                  </div>
                </div>
              </div>

              <div className="space-y-6">
                <div className="border-primary/20 bg-primary/5 rounded-xl border p-5 shadow-sm">
                  <h3 className="text-primary mb-4 flex items-center gap-2 font-semibold">
                    <AlertCircle className="h-4 w-4" /> 人工审核
                  </h3>

                  <label className="mb-4 block space-y-2">
                    <span className="text-sm font-medium">管理员备注</span>
                    <Textarea
                      className="bg-background min-h-[120px]"
                      value={note}
                      onChange={(event) => setNote(event.target.value)}
                      placeholder="写入管理员备注，审批时会一并保存，用户可见。"
                    />
                  </label>

                  <div className="flex flex-wrap gap-2">
                    {REVIEW_ACTIONS.map((action) => (
                      <Button
                        key={action.value}
                        variant={action.value === "reject" ? "destructive" : "default"}
                        disabled={acting !== null}
                        onClick={() => void onReview(action.value)}
                        className="flex-1 sm:flex-none"
                      >
                        {acting === action.value ? "提交中..." : action.label}
                      </Button>
                    ))}
                  </div>
                </div>

                {detail.manual_actions.length > 0 && (
                  <div className="bg-card rounded-xl border p-5 shadow-sm">
                    <h3 className="mb-4 font-semibold">推荐人工动作</h3>
                    <div className="grid gap-3">
                      {detail.manual_actions.map((action) => (
                        <div key={action.action} className="bg-muted/30 rounded-lg border p-4">
                          <p className="text-sm font-medium">{action.label}</p>
                          <p className="text-muted-foreground mt-1 text-sm">{action.description}</p>
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {(detail.request.agent_note || detail.request.content) && (
                  <div className="bg-card rounded-xl border p-5 shadow-sm">
                    <h3 className="mb-4 font-semibold">请求上下文</h3>
                    {detail.request.content && (
                      <div className="mb-4">
                        <p className="text-muted-foreground mb-1 text-xs tracking-wider uppercase">
                          用户说明
                        </p>
                        <p className="text-sm">{detail.request.content}</p>
                      </div>
                    )}
                    {detail.request.agent_note && (
                      <div>
                        <p className="text-muted-foreground mb-1 text-xs tracking-wider uppercase">
                          Agent 笔记
                        </p>
                        <p className="text-muted-foreground text-sm">{detail.request.agent_note}</p>
                      </div>
                    )}
                  </div>
                )}

                {hasObjectContent(detail.request.provider_result) && (
                  <div className="bg-card rounded-xl border p-5 shadow-sm">
                    <h3 className="mb-4 font-semibold">Agent 审计视图</h3>
                    <AuditSummary payload={detail.request.provider_result} />
                  </div>
                )}
              </div>
            </div>
          </div>
        ) : null}
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-6xl space-y-8">
      <div className="flex flex-col gap-4 border-b pb-6 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">求片 Agent 管理</h1>
          <p className="text-muted-foreground mt-1 text-sm">
            集中处理用户求片工单，维护自动化策略与阈值。
          </p>
        </div>
        <div className="bg-muted flex rounded-lg p-1">
          <button
            className={cn(
              "rounded-md px-4 py-2 text-sm font-medium transition-colors",
              activeTab === "requests"
                ? "bg-background text-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground"
            )}
            onClick={() => setActiveTab("requests")}
          >
            请求工单
          </button>
          <button
            className={cn(
              "rounded-md px-4 py-2 text-sm font-medium transition-colors",
              activeTab === "settings"
                ? "bg-background text-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground"
            )}
            onClick={() => setActiveTab("settings")}
          >
            系统设置
          </button>
        </div>
      </div>

      {activeTab === "requests" ? (
        <div className="animate-in fade-in space-y-4 duration-300">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <select
                className="border-input bg-background focus-visible:ring-ring h-9 w-[180px] rounded-md border px-3 py-1 text-sm shadow-sm transition-colors focus-visible:ring-1 focus-visible:outline-none"
                value={statusFilter}
                onChange={(event) => {
                  const value = event.target.value;
                  setStatusFilter(value);
                  void reload(value);
                }}
              >
                <option value="">全部状态</option>
                <option value="review_required">待人工处理</option>
                <option value="auto_processing">自动处理中</option>
                <option value="completed">已完成</option>
                <option value="failed">失败</option>
              </select>
            </div>
            <Button variant="outline" size="sm" onClick={() => void reload()} className="gap-2">
              <RotateCw className="h-3.5 w-3.5" /> 刷新
            </Button>
          </div>

          <div className="bg-card overflow-hidden rounded-xl border shadow-sm">
            {requests.length === 0 ? (
              <div className="py-16">
                <EmptyState title="暂无工单" description="当前筛选条件下没有可处理的请求。" />
              </div>
            ) : (
              <Table>
                <TableHeader className="bg-muted/30">
                  <TableRow className="hover:bg-transparent">
                    <TableHead className="h-11 w-[40%]">请求标题</TableHead>
                    <TableHead className="h-11">类型</TableHead>
                    <TableHead className="h-11">状态</TableHead>
                    <TableHead className="h-11">创建时间</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {requests.map((request) => (
                    <TableRow
                      key={request.id}
                      className="hover:bg-muted/50 group h-16 cursor-pointer transition-colors"
                      onClick={() => void onSelect(request.id)}
                    >
                      <TableCell>
                        <div className="text-foreground font-medium">{request.title}</div>
                        {(request.agent_note || request.content) && (
                          <div className="text-muted-foreground mt-1 max-w-[300px] truncate text-xs lg:max-w-[450px]">
                            {request.agent_note || request.content}
                          </div>
                        )}
                      </TableCell>
                      <TableCell>
                        <Badge variant="outline" className="bg-background/50 font-normal">
                          {requestTypeLabel(request.request_type)}
                        </Badge>
                      </TableCell>
                      <TableCell>
                        <Badge variant={requestStatusVariant(request.status_admin)}>
                          {request.status_admin}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-muted-foreground text-sm">
                        <div className="flex items-center justify-between">
                          {formatRelativeTime(request.created_at)}
                          <ChevronRight className="text-muted-foreground h-4 w-4 opacity-0 transition-opacity group-hover:opacity-100" />
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </div>
        </div>
      ) : (
        <div className="animate-in fade-in grid gap-8 duration-300 lg:grid-cols-3">
          <div className="space-y-6 lg:col-span-2">
            <div className="bg-card rounded-xl border p-6 shadow-sm">
              <div className="mb-6 flex items-start justify-between gap-4">
                <div>
                  <h3 className="flex items-center gap-2 text-lg font-semibold">
                    <Settings className="text-muted-foreground h-5 w-5" /> Agent 核心设置
                  </h3>
                  <p className="text-muted-foreground mt-1 text-sm">
                    维护自动化总开关、扫描计划和 MoviePilot 连接参数。
                  </p>
                </div>
                <div className="flex gap-2">
                  <Button
                    variant="outline"
                    onClick={() => void onTestMoviePilot()}
                    disabled={testingMoviePilot}
                  >
                    {testingMoviePilot ? "测试中..." : "测试连接"}
                  </Button>
                  <Button onClick={() => void onSaveSettings()} disabled={savingSettings}>
                    {savingSettings ? "保存中..." : "保存设置"}
                  </Button>
                </div>
              </div>

              <div className="grid gap-6 sm:grid-cols-2">
                <div className="space-y-4">
                  <h4 className="text-muted-foreground text-sm font-medium tracking-wider uppercase">
                    行为策略
                  </h4>
                  <div className="space-y-3">
                    <SettingSwitch
                      label="启用 Agent 自动处理"
                      checked={settings.enabled}
                      onChange={(checked) =>
                        setSettings((current) => ({ ...current, enabled: checked }))
                      }
                    />
                    <SettingSwitch
                      label="启用缺集扫描"
                      checked={settings.missing_scan_enabled}
                      onChange={(checked) =>
                        setSettings((current) => ({ ...current, missing_scan_enabled: checked }))
                      }
                    />
                    <SettingSwitch
                      label="命中库内自动关闭工单"
                      checked={settings.auto_close_on_library_hit}
                      onChange={(checked) =>
                        setSettings((current) => ({
                          ...current,
                          auto_close_on_library_hit: checked,
                        }))
                      }
                    />
                    <SettingSwitch
                      label="解析歧义时转交人工"
                      checked={settings.review_required_on_parse_ambiguity}
                      onChange={(checked) =>
                        setSettings((current) => ({
                          ...current,
                          review_required_on_parse_ambiguity: checked,
                        }))
                      }
                    />
                  </div>

                  <div className="pt-2">
                    <label className="block space-y-1.5 pb-4">
                      <span className="text-sm font-medium">最大自动轮次</span>
                      <Input
                        type="number"
                        min={1}
                        max={20}
                        value={String(settings.max_rounds)}
                        onChange={(event) =>
                          setSettings((current) => ({
                            ...current,
                            max_rounds: Number(event.target.value || 10),
                          }))
                        }
                      />
                    </label>
                    <label className="block space-y-1.5 pb-4">
                      <span className="text-sm font-medium">提问等待超时（分钟）</span>
                      <Input
                        type="number"
                        min={1}
                        value={String(settings.question_timeout_minutes)}
                        onChange={(event) =>
                          setSettings((current) => ({
                            ...current,
                            question_timeout_minutes: Number(event.target.value || 1440),
                          }))
                        }
                      />
                    </label>
                    <label className="block space-y-1.5">
                      <span className="text-sm font-medium">缺集扫描 Cron 表达式</span>
                      <Input
                        value={settings.missing_scan_cron}
                        onChange={(event) =>
                          setSettings((current) => ({
                            ...current,
                            missing_scan_cron: event.target.value,
                          }))
                        }
                      />
                    </label>
                  </div>
                </div>

                <div className="space-y-4">
                  <h4 className="text-muted-foreground text-sm font-medium tracking-wider uppercase">
                    LLM 意图识别 (OpenAI 兼容)
                  </h4>
                  <div className="space-y-4">
                    <SettingSwitch
                      label="启用 LLM 解析"
                      checked={settings.llm.enabled}
                      onChange={(checked) =>
                        setSettings((current) => ({
                          ...current,
                          llm: { ...current.llm, enabled: checked },
                        }))
                      }
                    />
                    <label className="block space-y-1.5">
                      <span className="text-sm font-medium">API Base URL</span>
                      <Input
                        value={settings.llm.base_url}
                        placeholder="https://api.openai.com/v1"
                        onChange={(event) =>
                          setSettings((current) => ({
                            ...current,
                            llm: { ...current.llm, base_url: event.target.value },
                          }))
                        }
                      />
                    </label>
                    <label className="block space-y-1.5">
                      <span className="text-sm font-medium">API Key</span>
                      <Input
                        type="password"
                        value={settings.llm.api_key}
                        placeholder="sk-..."
                        onChange={(event) =>
                          setSettings((current) => ({
                            ...current,
                            llm: { ...current.llm, api_key: event.target.value },
                          }))
                        }
                      />
                    </label>
                    <label className="block space-y-1.5">
                      <span className="text-sm font-medium">模型 (Model)</span>
                      <Input
                        value={settings.llm.model}
                        placeholder="gpt-4o-mini"
                        onChange={(event) =>
                          setSettings((current) => ({
                            ...current,
                            llm: { ...current.llm, model: event.target.value },
                          }))
                        }
                      />
                    </label>
                  </div>
                </div>

                <div className="space-y-4">
                  <h4 className="text-muted-foreground text-sm font-medium tracking-wider uppercase">
                    MoviePilot 集成
                  </h4>
                  <div className="space-y-4">
                    <SettingSwitch
                      label="启用 MoviePilot Provider"
                      checked={settings.moviepilot.enabled}
                      onChange={(checked) =>
                        setSettings((current) => ({
                          ...current,
                          moviepilot: { ...current.moviepilot, enabled: checked },
                        }))
                      }
                    />
                    <p className="text-muted-foreground text-xs leading-5">
                      关闭后会保留连接参数，但连接测试和 Agent 工作流都会将 MoviePilot 视为禁用。
                    </p>
                    <label className="block space-y-1.5">
                      <span className="text-sm font-medium">Base URL</span>
                      <Input
                        value={settings.moviepilot.base_url}
                        placeholder="http://moviepilot:3000"
                        onChange={(event) =>
                          setSettings((current) => ({
                            ...current,
                            moviepilot: { ...current.moviepilot, base_url: event.target.value },
                          }))
                        }
                      />
                    </label>
                    <label className="block space-y-1.5">
                      <span className="text-sm font-medium">用户名</span>
                      <Input
                        value={settings.moviepilot.username}
                        onChange={(event) =>
                          setSettings((current) => ({
                            ...current,
                            moviepilot: { ...current.moviepilot, username: event.target.value },
                          }))
                        }
                      />
                    </label>
                    <label className="block space-y-1.5">
                      <span className="text-sm font-medium">密码 / API Key</span>
                      <Input
                        type="password"
                        value={settings.moviepilot.password}
                        onChange={(event) =>
                          setSettings((current) => ({
                            ...current,
                            moviepilot: { ...current.moviepilot, password: event.target.value },
                          }))
                        }
                      />
                    </label>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div className="space-y-6">
            <div className="bg-card rounded-xl border p-6 shadow-sm">
              <div className="mb-5">
                <h3 className="flex items-center gap-2 text-lg font-semibold">
                  <Activity className="text-muted-foreground h-5 w-5" /> Provider 健康状态
                </h3>
                <p className="text-muted-foreground mt-1 text-sm">
                  当前 Agent 运行依赖的能力提供方状态。
                </p>
              </div>
              <div className="grid gap-4">
                {providers.length === 0 ? (
                  <p className="text-muted-foreground py-4 text-center text-sm">无提供方数据</p>
                ) : (
                  providers.map((provider) => (
                    <div key={provider.provider_id} className="bg-muted/30 rounded-lg border p-4">
                      <div className="mb-2 flex items-center justify-between gap-3">
                        <div>
                          <p className="text-sm font-semibold">{provider.display_name}</p>
                          <p className="text-muted-foreground text-xs">{provider.provider_kind}</p>
                        </div>
                        <Badge variant={provider.healthy ? "success" : "secondary"}>
                          {provider.healthy ? (
                            <span className="flex items-center gap-1">
                              <CheckCircle2 className="h-3 w-3" /> 健康
                            </span>
                          ) : (
                            "降级"
                          )}
                        </Badge>
                      </div>
                      <p className="text-muted-foreground text-sm">{provider.message}</p>
                      {provider.capabilities.length > 0 && (
                        <div className="mt-3 flex flex-wrap gap-1">
                          {provider.capabilities.map((cap) => (
                            <span
                              key={cap}
                              className="bg-background text-muted-foreground rounded border px-1.5 py-0.5 text-[10px] font-medium"
                            >
                              {cap}
                            </span>
                          ))}
                        </div>
                      )}
                    </div>
                  ))
                )}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function workflowStepTone(step: AgentWorkflowStepState): string {
  switch (step.status) {
    case "completed":
      return "border-emerald-500/20 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400";
    case "active":
      return "border-cyan-500/20 bg-cyan-500/10 text-cyan-600 dark:text-cyan-400";
    case "blocked":
      return "border-amber-500/20 bg-amber-500/10 text-amber-600 dark:text-amber-400";
    case "failed":
      return "border-rose-500/20 bg-rose-500/10 text-rose-600 dark:text-rose-400";
    default:
      return "border-border bg-muted/50 text-muted-foreground";
  }
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-background/50 rounded-xl border p-4 shadow-sm">
      <p className="text-muted-foreground text-xs font-medium">{label}</p>
      <p className="mt-1.5 text-base font-semibold">{value}</p>
    </div>
  );
}

function AuditSummary({ payload }: { payload: Record<string, unknown> }) {
  const sections = (
    [
      ["意图识别", payload.recognized_intent] as [string, unknown],
      ["精确搜索参数", payload.exact_query] as [string, unknown],
      ["执行计划", payload.agent_plan] as [string, unknown],
      ["已选资源", payload.selected_results] as [string, unknown],
      ["订阅结果", payload.subscription] as [string, unknown],
    ] satisfies Array<[string, unknown]>
  ).filter(([, value]) => {
    if (Array.isArray(value)) return value.length > 0;
    return hasObjectContent(value);
  });

  return (
    <div className="space-y-4">
      {sections.length === 0 ? (
        <pre className="bg-muted/50 overflow-x-auto rounded-lg p-3 text-xs break-all whitespace-pre-wrap">
          {JSON.stringify(payload, null, 2)}
        </pre>
      ) : (
        sections.map(([label, value]) => (
          <div key={label}>
            <p className="text-muted-foreground mb-2 text-xs font-medium tracking-wider uppercase">
              {label}
            </p>
            <pre className="bg-muted/50 overflow-x-auto rounded-lg p-3 text-xs break-all whitespace-pre-wrap">
              {JSON.stringify(value, null, 2)}
            </pre>
          </div>
        ))
      )}
    </div>
  );
}

function SettingSwitch({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <div className="bg-background hover:bg-muted/50 flex items-center justify-between rounded-lg border px-4 py-3 shadow-sm transition-colors">
      <label className="cursor-pointer text-sm font-medium" onClick={() => onChange(!checked)}>
        {label}
      </label>
      <Switch checked={checked} onCheckedChange={onChange} />
    </div>
  );
}
