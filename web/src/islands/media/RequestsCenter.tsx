import React, { useCallback, useEffect, useMemo, useState } from "react";
import {
  ChevronLeft,
  ChevronRight,
  Plus,
  RefreshCw,
  XCircle,
  Info,
  AlertCircle,
  Sparkles,
} from "lucide-react";

import { EmptyState, ErrorState, LoadingState } from "@/components/domain/DataState";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
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
  createMyRequest,
  getMyRequest,
  listMyRequests,
  resubmitMyRequest,
} from "@/lib/api/requests";
import type { ApiError } from "@/lib/api/client";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import { toast } from "@/lib/notifications/toast-store";
import type {
  AgentCreateRequest,
  AgentRequest,
  AgentRequestDetail,
  AgentWorkflowStepState,
} from "@/lib/types/requests";
import { cn, formatDate, formatDuration, formatRelativeTime } from "@/lib/utils";
import { mapToUserStage } from "@/lib/utils/workflow-display";

const REQUEST_TYPE_LABELS: Record<string, string> = {
  intake: "智能受理",
  media_request: "求片 / 求剧",
  feedback: "反馈",
  missing_episode: "缺集",
  missing_season: "漏季",
};

function statusVariant(
  status: string
): "glass" | "success" | "danger" | "secondary" | "outline" | "default" {
  if (status === "success") return "success";
  if (status === "failed") return "danger";
  if (status === "action_required") return "secondary";
  return "outline";
}

function statusLabel(status: string): string {
  switch (status) {
    case "success":
      return "已完成";
    case "failed":
      return "失败";
    case "action_required":
      return "需人工处理";
    case "closed":
      return "已关闭";
    default:
      return "处理中";
  }
}

function requestTypeLabel(type: string): string {
  return REQUEST_TYPE_LABELS[type] ?? type;
}

function parseNumberList(raw: string): number[] {
  return raw
    .split(",")
    .map((part) => Number(part.trim()))
    .filter((part) => Number.isFinite(part) && part > 0)
    .map((part) => Math.floor(part));
}

function hasObjectContent(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && Object.keys(value as object).length > 0;
}

export function RequestsCenter() {
  const { ready } = useAuthSession();
  const [requests, setRequests] = useState<AgentRequest[]>([]);
  const [detail, setDetail] = useState<AgentRequestDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [detailLoading, setDetailLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [resubmittingId, setResubmittingId] = useState<string | null>(null);

  const [showForm, setShowForm] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);

  const [form, setForm] = useState({
    raw_text: "",
    media_type: "unknown",
    tmdb_id: "",
    season_numbers: "",
    episode_numbers: "",
  });

  const onSelect = useCallback(async (requestId: string) => {
    setDetailLoading(true);
    setDetail(null);
    try {
      const payload = await getMyRequest(requestId);
      setDetail(payload);
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(apiError.message || "加载详情失败");
    } finally {
      setDetailLoading(false);
    }
  }, []);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const payload = await listMyRequests({ limit: 50 });
      setRequests(payload);
      setError(null);
    } catch (cause) {
      const apiError = cause as ApiError;
      setError(apiError.message || "加载请求失败");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!ready) return;
    void reload();
  }, [ready, reload]);

  async function onSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!form.raw_text.trim()) {
      toast.warning("请先告诉 Agent 你想做什么");
      return;
    }
    setSubmitting(true);
    try {
      const payload: AgentCreateRequest = {
        request_type: "intake",
        title: form.raw_text.trim(),
        content: form.raw_text.trim(),
        media_type: form.media_type.trim(),
        tmdb_id: form.tmdb_id.trim() ? Number(form.tmdb_id.trim()) : null,
        season_numbers: parseNumberList(form.season_numbers),
        episode_numbers: parseNumberList(form.episode_numbers),
      };
      const created = await createMyRequest(payload);
      setRequests((current) => [created.request, ...current]);
      setForm({
        raw_text: "",
        media_type: "unknown",
        tmdb_id: "",
        season_numbers: "",
        episode_numbers: "",
      });
      setShowForm(false);
      setShowAdvanced(false);
      toast.success("请求已提交");
      void onSelect(created.request.id);
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(apiError.message || "提交失败");
    } finally {
      setSubmitting(false);
    }
  }

  async function onResubmit(requestId: string) {
    setResubmittingId(requestId);
    try {
      const payload = await resubmitMyRequest(requestId);
      setRequests((current) =>
        current.map((item) => (item.id === payload.request.id ? payload.request : item))
      );
      setDetail(payload);
      toast.success("已重新触发处理");
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(apiError.message || "重试失败");
    } finally {
      setResubmittingId(null);
    }
  }

  const summary = useMemo(
    () => ({
      total: requests.length,
      processing: requests.filter((item) => item.status_user === "processing").length,
      done: requests.filter((item) => item.status_user === "success").length,
    }),
    [requests]
  );

  if (!ready || loading) {
    return <LoadingState title="加载求片中心" description="正在读取你当前的请求历史。" />;
  }

  if (error) {
    return <ErrorState title="请求中心加载失败" description={error} />;
  }

  if (detail || detailLoading) {
    const audit = detail?.request.provider_result;
    return (
      <div className="animate-in fade-in slide-in-from-bottom-4 mx-auto max-w-5xl space-y-6 duration-300">
        <Button
          variant="ghost"
          onClick={() => setDetail(null)}
          className="text-muted-foreground hover:text-foreground mb-2 -ml-4"
        >
          <ChevronLeft className="mr-1 h-4 w-4" /> 返回列表
        </Button>

        {detailLoading ? (
          <div className="py-12">
            <LoadingState title="加载请求详情" />
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
                  <Badge variant={statusVariant(detail.request.status_user)}>
                    {statusLabel(detail.request.status_user)}
                  </Badge>
                  {detail.request.status_user === "processing" && (
                    <>
                      <span>·</span>
                      <span className="flex items-center gap-1">
                        <RefreshCw className="h-3 w-3 animate-spin" /> 已处理{" "}
                        {formatDuration(detail.request.created_at)}
                      </span>
                    </>
                  )}
                </div>
              </div>
              {(detail.request.status_user === "failed" ||
                detail.request.status_user === "action_required") && (
                <Button
                  variant="default"
                  disabled={resubmittingId === detail.request.id}
                  onClick={() => void onResubmit(detail.request.id)}
                >
                  <RefreshCw
                    className={cn(
                      "mr-2 h-4 w-4",
                      resubmittingId === detail.request.id && "animate-spin"
                    )}
                  />
                  {resubmittingId === detail.request.id ? "重新触发中..." : "重新触发"}
                </Button>
              )}
            </div>

            <div className="grid grid-cols-3 gap-4">
              <InfoStat label="自动处理" value={detail.request.auto_handled ? "是" : "否"} />
              <InfoStat label="TMDB ID" value={String(detail.request.tmdb_id ?? "-")} />
              <InfoStat
                label="季 / 集"
                value={`${detail.request.season_numbers.join(",") || "-"} / ${detail.request.episode_numbers.join(",") || "-"}`}
              />
            </div>

            <div className="grid gap-6 md:grid-cols-2">
              <div className="space-y-6">
                <div className="bg-card rounded-xl border p-5 shadow-sm">
                  <div className="mb-4 flex items-center justify-between">
                    <h3 className="font-semibold">处理进度</h3>
                  </div>
                  <WorkflowProgress steps={detail.workflow_steps} />
                </div>

                <div className="bg-card rounded-xl border p-5 shadow-sm">
                  <h3 className="mb-4 font-semibold">处理时间线</h3>
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
                      </div>
                    ))}
                  </div>
                </div>
              </div>

              <div className="space-y-6">
                {detail.request.content && (
                  <div className="bg-card rounded-xl border p-5 shadow-sm">
                    <h3 className="mb-4 flex items-center gap-2 font-semibold">
                      <Info className="text-muted-foreground h-4 w-4" /> 原始提交内容
                    </h3>
                    <p className="text-sm leading-relaxed whitespace-pre-wrap">
                      {detail.request.content}
                    </p>
                  </div>
                )}

                {hasObjectContent(audit) && (
                  <AuditInsightCard title="Agent 审计视图" payload={audit} />
                )}

                {detail.request.admin_note && (
                  <div className="rounded-xl border border-amber-500/30 bg-amber-500/5 p-5 shadow-sm">
                    <h3 className="mb-3 flex items-center gap-2 font-semibold text-amber-700 dark:text-amber-500">
                      <AlertCircle className="h-4 w-4" /> 管理员备注
                    </h3>
                    <p className="text-sm leading-relaxed whitespace-pre-wrap text-amber-800 dark:text-amber-200">
                      {detail.request.admin_note}
                    </p>
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
    <div className="mx-auto max-w-6xl space-y-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">求片与反馈</h1>
          <p className="text-muted-foreground mt-1 text-sm">
            直接用自然语言告诉 Agent 你的诉求，它会自动识别意图并处理。
          </p>
        </div>
        <div className="flex items-center gap-6">
          <div className="mr-4 hidden items-center gap-4 text-sm sm:flex">
            <div className="flex flex-col items-center">
              <span className="text-lg font-semibold">{summary.total}</span>
              <span className="text-muted-foreground text-xs">总工单</span>
            </div>
            <div className="bg-border h-8 w-px"></div>
            <div className="flex flex-col items-center">
              <span className="text-lg font-semibold text-cyan-600 dark:text-cyan-400">
                {summary.processing}
              </span>
              <span className="text-muted-foreground text-xs">处理中</span>
            </div>
            <div className="bg-border h-8 w-px"></div>
            <div className="flex flex-col items-center">
              <span className="text-lg font-semibold text-emerald-600 dark:text-emerald-400">
                {summary.done}
              </span>
              <span className="text-muted-foreground text-xs">已完成</span>
            </div>
          </div>
          <Button onClick={() => setShowForm(!showForm)} className="gap-2 transition-all">
            {showForm ? <XCircle className="h-4 w-4" /> : <Plus className="h-4 w-4" />}
            {showForm ? "取消新建" : "新建请求"}
          </Button>
        </div>
      </div>

      {showForm && (
        <div className="animate-in fade-in slide-in-from-top-4 bg-card rounded-xl border p-6 shadow-sm duration-300">
          <div className="mb-6">
            <h2 className="text-lg font-semibold">告诉 Agent 你想做什么</h2>
            <p className="text-muted-foreground text-sm">
              例如：JOJO的奇妙冒险的第五季资源能换奈飞的资源么。 或 基地第二季缺第5集。
            </p>
          </div>
          <form onSubmit={(event) => void onSubmit(event)} className="space-y-6">
            <div className="space-y-2">
              <label className="text-sm font-medium">
                需求描述 <span className="text-rose-500">*</span>
              </label>
              <Textarea
                className="min-h-[120px] resize-y"
                value={form.raw_text}
                onChange={(e) => setForm((current) => ({ ...current, raw_text: e.target.value }))}
                placeholder="直接描述你的需求，Agent 会自动判断是求片、换源、补集、漏季还是普通反馈。"
                required
              />
            </div>

            <div className="rounded-xl border border-cyan-500/20 bg-cyan-500/5 p-4 text-sm">
              <div className="mb-2 flex items-center gap-2 font-medium text-cyan-700 dark:text-cyan-300">
                <Sparkles className="h-4 w-4" /> Agent 会自动完成这些步骤
              </div>
              <ul className="text-muted-foreground list-disc space-y-1 pl-5">
                <li>识别你是在求片、换源、补集、漏季还是提交反馈</li>
                <li>自动匹配 TMDB 元数据并拼接 MoviePilot 精确搜索</li>
                <li>展示识别结果、搜索参数、候选资源和最终动作</li>
              </ul>
            </div>

            <div className="space-y-2">
              <button
                type="button"
                className="text-muted-foreground hover:text-foreground flex items-center gap-1 text-sm font-medium transition-colors"
                onClick={() => setShowAdvanced(!showAdvanced)}
              >
                {showAdvanced ? "收起高级选项" : "展开高级选项"}
              </button>
            </div>

            {showAdvanced && (
              <div className="animate-in fade-in slide-in-from-top-2 bg-muted/30 grid gap-6 rounded-xl border p-5 md:grid-cols-2 lg:grid-cols-4">
                <div className="space-y-2">
                  <label className="text-muted-foreground text-xs font-medium">媒体类型</label>
                  <select
                    className="border-input bg-background focus-visible:ring-ring flex h-9 w-full rounded-md border px-3 py-1 text-sm shadow-sm transition-colors focus-visible:ring-1 focus-visible:outline-none"
                    value={form.media_type}
                    onChange={(e) =>
                      setForm((current) => ({ ...current, media_type: e.target.value }))
                    }
                  >
                    <option value="unknown">未指定</option>
                    <option value="movie">电影</option>
                    <option value="series">剧集</option>
                  </select>
                </div>
                <div className="space-y-2">
                  <label className="text-muted-foreground text-xs font-medium">TMDB ID</label>
                  <Input
                    className="h-9"
                    value={form.tmdb_id}
                    onChange={(e) =>
                      setForm((current) => ({ ...current, tmdb_id: e.target.value }))
                    }
                    placeholder="可选"
                  />
                </div>
                <div className="space-y-2">
                  <label className="text-muted-foreground text-xs font-medium">季号</label>
                  <Input
                    className="h-9"
                    value={form.season_numbers}
                    onChange={(e) =>
                      setForm((current) => ({ ...current, season_numbers: e.target.value }))
                    }
                    placeholder="如: 1,2"
                  />
                </div>
                <div className="space-y-2">
                  <label className="text-muted-foreground text-xs font-medium">集号</label>
                  <Input
                    className="h-9"
                    value={form.episode_numbers}
                    onChange={(e) =>
                      setForm((current) => ({ ...current, episode_numbers: e.target.value }))
                    }
                    placeholder="如: 5,6"
                  />
                </div>
              </div>
            )}

            <div className="flex justify-end gap-3 pt-2">
              <Button type="button" variant="ghost" onClick={() => setShowForm(false)}>
                取消
              </Button>
              <Button type="submit" disabled={submitting}>
                {submitting ? "提交中..." : "提交请求"}
              </Button>
            </div>
          </form>
        </div>
      )}

      <div className="bg-card overflow-hidden rounded-xl border shadow-sm">
        {requests.length === 0 ? (
          <div className="py-16">
            <EmptyState
              title="还没有请求"
              description="提交第一个请求后，这里会展示 Agent 的实时处理状态。"
            />
          </div>
        ) : (
          <Table>
            <TableHeader className="bg-muted/30">
              <TableRow className="hover:bg-transparent">
                <TableHead className="h-11 w-[45%]">请求标题</TableHead>
                <TableHead className="h-11">类型</TableHead>
                <TableHead className="h-11">状态</TableHead>
                <TableHead className="h-11">更新时间</TableHead>
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
                    {request.content && (
                      <div className="text-muted-foreground mt-1 max-w-[350px] truncate text-xs lg:max-w-[500px]">
                        {request.content}
                      </div>
                    )}
                  </TableCell>
                  <TableCell>
                    <Badge variant="outline" className="bg-background/50 font-normal">
                      {requestTypeLabel(request.request_type)}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <Badge variant={statusVariant(request.status_user)}>
                      {statusLabel(request.status_user)}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-muted-foreground text-sm">
                    <div className="flex items-center gap-4">
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
  );
}

function WorkflowProgress({ steps }: { steps: AgentWorkflowStepState[] }) {
  const stage = mapToUserStage(steps);

  return (
    <div className="space-y-4">
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium">{stage.label}</span>
          <span className="text-muted-foreground text-xs font-medium">{stage.progress}%</span>
        </div>
        <div className="bg-muted h-2 w-full overflow-hidden rounded-full">
          <div
            className="h-full bg-cyan-500 transition-all duration-500 ease-in-out"
            style={{ width: `${stage.progress}%` }}
          />
        </div>
        <p className="text-muted-foreground text-xs">{stage.description}</p>
      </div>
    </div>
  );
}

function InfoStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-background/50 rounded-xl border p-4 shadow-sm">
      <p className="text-muted-foreground text-xs font-medium">{label}</p>
      <p className="mt-1.5 text-base font-semibold">{value}</p>
    </div>
  );
}

function AuditInsightCard({ title, payload }: { title: string; payload: Record<string, unknown> }) {
  const recognizedIntent = hasObjectContent(payload.recognized_intent)
    ? (payload.recognized_intent as Record<string, unknown>)
    : null;
  const exactQuery = hasObjectContent(payload.exact_query)
    ? (payload.exact_query as Record<string, unknown>)
    : null;
  const agentPlan = hasObjectContent(payload.agent_plan)
    ? (payload.agent_plan as Record<string, unknown>)
    : null;

  return (
    <div className="bg-card rounded-xl border p-5 shadow-sm">
      <h3 className="mb-4 font-semibold">{title}</h3>
      <div className="space-y-4">
        {recognizedIntent && <AuditBlock label="意图识别" value={recognizedIntent} />}
        {exactQuery && <AuditBlock label="精确搜索参数" value={exactQuery} />}
        {agentPlan && <AuditBlock label="执行计划" value={agentPlan} />}
        {Array.isArray(payload.selected_results) && payload.selected_results.length > 0 && (
          <AuditBlock label="已选资源" value={payload.selected_results} />
        )}
        {hasObjectContent(payload.subscription) && (
          <AuditBlock label="订阅结果" value={payload.subscription as Record<string, unknown>} />
        )}
        {!recognizedIntent && !exactQuery && !agentPlan && (
          <AuditBlock label="原始审计数据" value={payload} />
        )}
      </div>
    </div>
  );
}

function AuditBlock({ label, value }: { label: string; value: unknown }) {
  return (
    <div>
      <p className="text-muted-foreground mb-2 text-xs font-medium tracking-wider uppercase">
        {label}
      </p>
      <pre className="bg-muted/50 overflow-x-auto rounded-lg p-3 text-xs break-all whitespace-pre-wrap">
        {JSON.stringify(value, null, 2)}
      </pre>
    </div>
  );
}
