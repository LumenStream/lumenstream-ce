import React, { useCallback, useEffect, useMemo, useState } from "react";
import {
  ChevronLeft,
  ChevronRight,
  Plus,
  RefreshCw,
  XCircle,
  Info,
  AlertCircle,
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
import { cn, formatDate, formatRelativeTime, formatDuration } from "@/lib/utils";
import { mapToUserStage } from "@/lib/utils/workflow-display";

const REQUEST_TYPE_OPTIONS: Array<{ value: AgentCreateRequest["request_type"]; label: string }> = [
  { value: "media_request", label: "求片 / 求剧" },
  { value: "feedback", label: "反馈" },
  { value: "missing_episode", label: "缺集" },
  { value: "missing_season", label: "漏季" },
];

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
  return REQUEST_TYPE_OPTIONS.find((item) => item.value === type)?.label ?? type;
}

function parseNumberList(raw: string): number[] {
  return raw
    .split(",")
    .map((part) => Number(part.trim()))
    .filter((part) => Number.isFinite(part) && part > 0)
    .map((part) => Math.floor(part));
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
    request_type: "media_request" as AgentCreateRequest["request_type"],
    title: "",
    content: "",
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

  useEffect(() => {
    const shouldAutoExpand = ["missing_episode", "missing_season"].includes(form.request_type);
    setShowAdvanced(shouldAutoExpand);
  }, [form.request_type]);

  async function onSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!form.title.trim()) {
      toast.warning("标题不能为空");
      return;
    }
    setSubmitting(true);
    try {
      const created = await createMyRequest({
        request_type: form.request_type,
        title: form.title.trim(),
        content: form.content.trim(),
        media_type: form.media_type.trim(),
        tmdb_id: form.tmdb_id.trim() ? Number(form.tmdb_id.trim()) : null,
        season_numbers: parseNumberList(form.season_numbers),
        episode_numbers: parseNumberList(form.episode_numbers),
      });
      setRequests((current) => [created.request, ...current]);
      setForm({
        request_type: "media_request",
        title: "",
        content: "",
        media_type: "unknown",
        tmdb_id: "",
        season_numbers: "",
        episode_numbers: "",
      });
      setShowForm(false);
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
                      <Info className="text-muted-foreground h-4 w-4" /> 请求说明
                    </h3>
                    <div className="mb-5">
                      <p className="text-sm leading-relaxed whitespace-pre-wrap">
                        {detail.request.content}
                      </p>
                    </div>
                  </div>
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
          <p className="text-muted-foreground mt-1 text-sm">统一提交求片、缺集、漏季和问题反馈。</p>
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
            <h2 className="text-lg font-semibold">填写请求详情</h2>
            <p className="text-muted-foreground text-sm">
              提供尽可能详细的信息，以便系统能更快地自动处理。
            </p>
          </div>
          <form onSubmit={(event) => void onSubmit(event)} className="space-y-6">
            <div className="grid gap-6 md:grid-cols-2">
              <div className="space-y-2">
                <label className="text-sm font-medium">请求类型</label>
                <select
                  className="border-input bg-background ring-offset-background placeholder:text-muted-foreground focus-visible:ring-ring flex h-10 w-full rounded-md border px-3 py-2 text-sm file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
                  value={form.request_type}
                  onChange={(e) =>
                    setForm((f) => ({
                      ...f,
                      request_type: e.target.value as AgentCreateRequest["request_type"],
                    }))
                  }
                >
                  {REQUEST_TYPE_OPTIONS.map((opt) => (
                    <option key={opt.value} value={opt.value}>
                      {opt.label}
                    </option>
                  ))}
                </select>
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium">
                  标题 <span className="text-rose-500">*</span>
                </label>
                <Input
                  value={form.title}
                  onChange={(e) => setForm((f) => ({ ...f, title: e.target.value }))}
                  placeholder="例如：基地 第二季缺集"
                  required
                />
              </div>
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
                    onChange={(e) => setForm((f) => ({ ...f, media_type: e.target.value }))}
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
                    onChange={(e) => setForm((f) => ({ ...f, tmdb_id: e.target.value }))}
                    placeholder="可选"
                  />
                </div>
                <div className="space-y-2">
                  <label className="text-muted-foreground text-xs font-medium">季号</label>
                  <Input
                    className="h-9"
                    value={form.season_numbers}
                    onChange={(e) => setForm((f) => ({ ...f, season_numbers: e.target.value }))}
                    placeholder="如: 1,2"
                  />
                </div>
                <div className="space-y-2">
                  <label className="text-muted-foreground text-xs font-medium">集号</label>
                  <Input
                    className="h-9"
                    value={form.episode_numbers}
                    onChange={(e) => setForm((f) => ({ ...f, episode_numbers: e.target.value }))}
                    placeholder="如: 5,6"
                  />
                </div>
              </div>
            )}

            <div className="space-y-2">
              <label className="text-sm font-medium">补充说明</label>
              <Textarea
                className="min-h-[100px] resize-y"
                value={form.content}
                onChange={(e) => setForm((f) => ({ ...f, content: e.target.value }))}
                placeholder="例如：已完结但库里只有前四集；或者希望优先 4K / 中文字幕。"
              />
            </div>

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
              description="提交第一个求片或反馈后，这里会展示实时状态。"
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
