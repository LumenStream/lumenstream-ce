import React, { useEffect, useMemo, useState } from "react";

import { ErrorState, LoadingState } from "@/components/domain/DataState";
import { Modal } from "@/components/domain/Modal";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Textarea } from "@/components/ui/textarea";
import {
  cancelTaskRun,
  getTaskRunsWebSocketUrl,
  listTaskDefinitions,
  listTaskRuns,
  patchTaskDefinition,
  runTaskNow,
} from "@/lib/api/admin";
import { getAccessToken } from "@/lib/auth/token";
import type { ApiError } from "@/lib/api/client";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import { toast } from "@/lib/notifications/toast-store";
import type { AdminTaskDefinition, AdminTaskRun, TaskRunProgress } from "@/lib/types/admin";
import { formatDate } from "@/lib/utils";

interface TaskDraft {
  enabled: boolean;
  cron_expr: string;
  max_attempts: number;
  default_payload_text: string;
  run_payload_text: string;
}

function createDraft(task: AdminTaskDefinition): TaskDraft {
  return {
    enabled: task.enabled,
    cron_expr: task.cron_expr,
    max_attempts: task.max_attempts,
    default_payload_text: JSON.stringify(task.default_payload || {}, null, 2),
    run_payload_text: "{}",
  };
}

function parseJsonObject(raw: string, fieldName: string): Record<string, unknown> {
  const trimmed = raw.trim();
  if (!trimmed) {
    return {};
  }

  const value = JSON.parse(trimmed);
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${fieldName} 必须是 JSON 对象`);
  }
  return value as Record<string, unknown>;
}

function validateJsonObjectText(raw: string, fieldName: string): string | null {
  try {
    parseJsonObject(raw, fieldName);
    return null;
  } catch (cause) {
    if (cause instanceof Error) {
      return cause.message;
    }
    return `${fieldName} 格式无效`;
  }
}

function extractErrorMessage(cause: unknown, fallback: string): string {
  if (cause instanceof Error && cause.message) {
    return cause.message;
  }
  const apiError = cause as ApiError;
  if (apiError.message) {
    return apiError.message;
  }
  return fallback;
}

function runStatusVariant(status: string): "secondary" | "success" | "danger" {
  if (status === "failed") {
    return "danger";
  }
  if (status === "completed") {
    return "success";
  }
  return "secondary";
}

const MAINTENANCE_KINDS = new Set(["retry_dispatch", "cleanup_maintenance", "billing_expire"]);

function isMaintenanceKind(kind: string): boolean {
  return MAINTENANCE_KINDS.has(kind);
}

function clampProgressPercent(value: number): number {
  if (!Number.isFinite(value)) {
    return 0;
  }
  return Math.max(0, Math.min(100, value));
}

function extractProgress(run: AdminTaskRun): TaskRunProgress | null {
  const progress = run.progress;
  if (!progress || typeof progress !== "object") {
    return null;
  }
  const total =
    typeof progress.total === "number" && Number.isFinite(progress.total) ? progress.total : 0;
  const completed =
    typeof progress.completed === "number" && Number.isFinite(progress.completed)
      ? progress.completed
      : 0;
  const percent =
    typeof progress.percent === "number" && Number.isFinite(progress.percent)
      ? clampProgressPercent(progress.percent)
      : total > 0
        ? clampProgressPercent((completed / total) * 100)
        : 0;

  return {
    ...progress,
    total,
    completed,
    percent,
  };
}

function renderProgress(run: AdminTaskRun) {
  const progress = extractProgress(run);
  const percent =
    run.status === "completed"
      ? 100
      : run.status === "failed"
        ? progress?.percent || 0
        : progress?.percent || 0;
  const message = progress?.message || (run.status === "completed" ? "任务完成" : "进行中");
  const total = progress?.total ?? 0;
  const completed = progress?.completed ?? 0;
  const width = `${clampProgressPercent(percent)}%`;
  return (
    <div className="space-y-1">
      <div className="bg-border h-1.5 w-full overflow-hidden rounded">
        <div
          className={run.status === "failed" ? "h-full bg-red-500" : "h-full bg-cyan-500"}
          style={{ width }}
        />
      </div>
      <p className="text-[10px] text-slate-300">
        {Math.round(percent)}% · {message}
        {total > 0 ? ` (${completed}/${total})` : ""}
      </p>
    </div>
  );
}

function canCancelRun(status: string): boolean {
  return status === "queued" || status === "pending" || status === "running";
}

interface TaskRunSocketEvent {
  event: string;
  run: AdminTaskRun;
  emitted_at: string;
}

export function AdminJobsPanel() {
  const { ready } = useAuthSession({ requireAdmin: true });
  const [tasks, setTasks] = useState<AdminTaskDefinition[]>([]);
  const [runs, setRuns] = useState<AdminTaskRun[]>([]);
  const [drafts, setDrafts] = useState<Record<string, TaskDraft>>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [savingKey, setSavingKey] = useState<string | null>(null);
  const [runningKey, setRunningKey] = useState<string | null>(null);
  const [cancellingRunId, setCancellingRunId] = useState<string | null>(null);
  const [editingTaskKey, setEditingTaskKey] = useState<string | null>(null);
  const [hideMaintenance, setHideMaintenance] = useState(true);

  const latestRunByTask = useMemo(() => {
    const map: Record<string, AdminTaskRun> = {};
    runs.forEach((run) => {
      if (!map[run.kind]) {
        map[run.kind] = run;
      }
    });
    return map;
  }, [runs]);

  const activeRunByTask = useMemo(() => {
    const map: Record<string, AdminTaskRun> = {};
    runs.forEach((run) => {
      if (canCancelRun(run.status) && !map[run.kind]) {
        map[run.kind] = run;
      }
    });
    return map;
  }, [runs]);

  const editingTask = editingTaskKey
    ? tasks.find((task) => task.task_key === editingTaskKey) || null
    : null;
  const editingDraft = editingTask
    ? drafts[editingTask.task_key] || createDraft(editingTask)
    : null;
  const defaultPayloadError = editingDraft
    ? validateJsonObjectText(editingDraft.default_payload_text, "默认参数")
    : null;
  const runPayloadError = editingDraft
    ? validateJsonObjectText(editingDraft.run_payload_text, "运行覆盖参数")
    : null;

  const reload = React.useCallback(async () => {
    setLoading(true);
    try {
      const [taskList, runList] = await Promise.all([
        listTaskDefinitions(),
        listTaskRuns({
          limit: 100,
          exclude_kinds: hideMaintenance
            ? "retry_dispatch,cleanup_maintenance,billing_expire"
            : undefined,
        }),
      ]);
      setTasks(taskList);
      setRuns(runList);
      setDrafts((prev) => {
        const next: Record<string, TaskDraft> = {};
        taskList.forEach((task) => {
          next[task.task_key] = prev[task.task_key] || createDraft(task);
        });
        return next;
      });
      setError(null);
    } catch (cause) {
      const apiError = cause as ApiError;
      setError(apiError.message || "加载任务中心失败");
    } finally {
      setLoading(false);
    }
  }, [hideMaintenance]);

  useEffect(() => {
    if (!ready) {
      return;
    }
    void reload();
  }, [ready, reload]);

  useEffect(() => {
    if (!ready || typeof WebSocket === "undefined") {
      return;
    }

    const token = getAccessToken();
    if (!token) {
      return;
    }

    let ws: WebSocket | null = null;
    let closed = false;
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
    let reconnectDelay = 1000;

    const connect = () => {
      if (closed) {
        return;
      }
      ws = new WebSocket(getTaskRunsWebSocketUrl(token));
      ws.onopen = () => {
        reconnectDelay = 1000;
      };
      ws.onmessage = (event) => {
        try {
          const parsed = JSON.parse(event.data) as TaskRunSocketEvent;
          if (!parsed?.run?.id) {
            return;
          }
          if (hideMaintenance && isMaintenanceKind(parsed.run.kind)) {
            return;
          }
          setRuns((prev) => {
            const idx = prev.findIndex((run) => run.id === parsed.run.id);
            if (idx >= 0) {
              const next = [...prev];
              next[idx] = parsed.run;
              return next;
            }
            return [parsed.run, ...prev].slice(0, 100);
          });
        } catch {
          // Ignore malformed websocket events.
        }
      };
      ws.onerror = () => {
        ws?.close();
      };
      ws.onclose = () => {
        if (closed) {
          return;
        }
        reconnectTimer = setTimeout(connect, reconnectDelay);
        reconnectDelay = Math.min(reconnectDelay * 2, 30000);
      };
    };

    connect();

    return () => {
      closed = true;
      if (reconnectTimer) {
        clearTimeout(reconnectTimer);
      }
      if (ws) {
        ws.close();
      }
    };
  }, [ready, hideMaintenance]);

  useEffect(() => {
    if (editingTaskKey && !tasks.some((task) => task.task_key === editingTaskKey)) {
      setEditingTaskKey(null);
    }
  }, [editingTaskKey, tasks]);

  function updateDraft(taskKey: string, patch: Partial<TaskDraft>) {
    setDrafts((prev) => ({
      ...prev,
      [taskKey]: {
        ...(prev[taskKey] ||
          createDraft(
            tasks.find((task) => task.task_key === taskKey) || {
              task_key: taskKey,
              display_name: taskKey,
              enabled: false,
              cron_expr: "",
              default_payload: {},
              max_attempts: 1,
              created_at: "",
              updated_at: "",
            }
          )),
        ...patch,
      },
    }));
  }

  async function onSave(taskKey: string) {
    const draft = drafts[taskKey];
    if (!draft) {
      return;
    }

    setSavingKey(taskKey);
    try {
      const defaultPayload = parseJsonObject(draft.default_payload_text, "默认参数");
      parseJsonObject(draft.run_payload_text, "运行覆盖参数");
      await patchTaskDefinition(taskKey, {
        enabled: draft.enabled,
        cron_expr: draft.cron_expr.trim(),
        max_attempts: Math.max(1, draft.max_attempts),
        default_payload: defaultPayload,
      });
      toast.success(`已保存任务 ${taskKey}`);
      await reload();
    } catch (cause) {
      toast.error(`保存失败：${extractErrorMessage(cause, "未知错误")}`);
    } finally {
      setSavingKey(null);
    }
  }

  async function onRun(taskKey: string) {
    const draft = drafts[taskKey];
    if (!draft) {
      return;
    }
    if (activeRunByTask[taskKey]) {
      toast.error("同类型任务已有执行中/排队中的运行，请先取消或等待完成");
      return;
    }

    setRunningKey(taskKey);
    try {
      const overridePayload = parseJsonObject(draft.run_payload_text, "运行覆盖参数");
      const run = await runTaskNow(taskKey, overridePayload);
      toast.success(`已触发任务 ${taskKey}，运行 ID: ${run.id}`);
      await reload();
    } catch (cause) {
      toast.error(`触发失败：${extractErrorMessage(cause, "未知错误")}`);
    } finally {
      setRunningKey(null);
    }
  }

  async function onCancelRun(runId: string) {
    setCancellingRunId(runId);
    try {
      const updated = await cancelTaskRun(runId);
      setRuns((prev) => {
        const idx = prev.findIndex((item) => item.id === runId);
        if (idx < 0) {
          return prev;
        }
        const next = [...prev];
        next[idx] = updated;
        return next;
      });
      toast.success(`已取消任务 ${runId}`);
    } catch (cause) {
      toast.error(`取消失败：${extractErrorMessage(cause, "未知错误")}`);
    } finally {
      setCancellingRunId(null);
    }
  }

  if (!ready || loading) {
    return <LoadingState title="加载任务中心" />;
  }

  if (error) {
    return <ErrorState title="任务中心加载失败" description={error} />;
  }

  return (
    <div className="space-y-8">
      <section className="border-border/50 border-b pb-6">
        <h3 className="text-sm font-medium">任务定义</h3>
        <p className="text-muted-foreground mt-1 mb-4 text-xs">
          主列表仅展示关键状态，详细参数通过二级弹窗配置。
        </p>
        <div className="space-y-3">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>任务</TableHead>
                <TableHead>Cron</TableHead>
                <TableHead>状态</TableHead>
                <TableHead>任务进度</TableHead>
                <TableHead>操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {tasks.map((task) => {
                const latestRun = latestRunByTask[task.task_key];
                const activeRun = activeRunByTask[task.task_key];
                const progressRun = activeRun || latestRun;
                return (
                  <TableRow key={task.task_key}>
                    <TableCell>
                      <p className="font-semibold">{task.display_name}</p>
                      <p className="font-mono text-xs text-slate-300">{task.task_key}</p>
                    </TableCell>
                    <TableCell className="font-mono text-xs">{task.cron_expr}</TableCell>
                    <TableCell>
                      <Badge variant={task.enabled ? "success" : "secondary"}>
                        {task.enabled ? "已启用" : "已停用"}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      {progressRun ? (
                        <div className="space-y-1">
                          <Badge variant={runStatusVariant(progressRun.status)}>
                            {progressRun.status}
                          </Badge>
                          <div className="flex items-start gap-1">
                            <div className="min-w-[220px] flex-1">
                              <p className="mb-1 text-xs text-slate-300">
                                {formatDate(progressRun.created_at)}
                              </p>
                              {renderProgress(progressRun)}
                            </div>
                            {canCancelRun(progressRun.status) && (
                              <button
                                type="button"
                                aria-label={`取消任务 ${progressRun.id}`}
                                className="mt-[20px] h-4 w-4 shrink-0 rounded border border-slate-500/60 text-[10px] leading-none text-slate-300 transition-colors hover:border-red-400 hover:text-red-300 disabled:cursor-not-allowed disabled:opacity-50"
                                disabled={cancellingRunId === progressRun.id}
                                onClick={() => void onCancelRun(progressRun.id)}
                              >
                                {cancellingRunId === progressRun.id ? "…" : "x"}
                              </button>
                            )}
                          </div>
                        </div>
                      ) : (
                        <span className="text-xs text-slate-400">暂无</span>
                      )}
                    </TableCell>
                    <TableCell>
                      <div className="flex flex-wrap gap-2">
                        <Button
                          size="sm"
                          variant="secondary"
                          onClick={() => setEditingTaskKey(task.task_key)}
                        >
                          配置
                        </Button>
                        <Button
                          size="sm"
                          onClick={() => void onRun(task.task_key)}
                          disabled={runningKey === task.task_key || Boolean(activeRun)}
                        >
                          {runningKey === task.task_key
                            ? "触发中..."
                            : activeRun
                              ? "执行中..."
                              : "立即执行"}
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </div>
      </section>

      <section>
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-sm font-medium">运行记录</h3>
            <p className="text-muted-foreground mt-1 mb-4 text-xs">显示最近 100 条任务执行记录。</p>
          </div>
          <div className="flex items-center space-x-2">
            <Checkbox
              id="hideMaintenance"
              checked={hideMaintenance}
              onCheckedChange={(checked) => setHideMaintenance(!!checked)}
            />
            <label
              htmlFor="hideMaintenance"
              className="text-muted-foreground cursor-pointer text-xs leading-none font-medium peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
            >
              隐藏高频维护任务
            </label>
          </div>
        </div>
        <div className="space-y-3">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>运行 ID</TableHead>
                <TableHead>任务</TableHead>
                <TableHead>触发方式</TableHead>
                <TableHead>状态</TableHead>
                <TableHead>进度</TableHead>
                <TableHead>尝试</TableHead>
                <TableHead>创建时间</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {runs.map((run) => (
                <TableRow key={run.id}>
                  <TableCell className="font-mono text-[10px]">{run.id}</TableCell>
                  <TableCell>{run.kind}</TableCell>
                  <TableCell>{run.trigger_type || "manual"}</TableCell>
                  <TableCell>
                    <Badge variant={runStatusVariant(run.status)}>{run.status}</Badge>
                  </TableCell>
                  <TableCell className="min-w-[220px]">
                    <div className="flex items-start gap-1">
                      <div className="min-w-0 flex-1">{renderProgress(run)}</div>
                      {canCancelRun(run.status) && (
                        <button
                          type="button"
                          aria-label={`取消任务 ${run.id}`}
                          className="mt-0.5 h-4 w-4 shrink-0 rounded border border-slate-500/60 text-[10px] leading-none text-slate-300 transition-colors hover:border-red-400 hover:text-red-300 disabled:cursor-not-allowed disabled:opacity-50"
                          disabled={cancellingRunId === run.id}
                          onClick={() => void onCancelRun(run.id)}
                        >
                          {cancellingRunId === run.id ? "…" : "x"}
                        </button>
                      )}
                    </div>
                  </TableCell>
                  <TableCell>
                    {run.attempts}/{run.max_attempts}
                  </TableCell>
                  <TableCell>{formatDate(run.created_at)}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      </section>

      <Modal
        open={Boolean(editingTask && editingDraft)}
        title={editingTask ? `任务配置 · ${editingTask.display_name}` : "任务配置"}
        description={editingTask ? editingTask.task_key : undefined}
        onClose={() => setEditingTaskKey(null)}
        showHeaderClose
        showFooterClose={false}
      >
        {editingTask && editingDraft ? (
          <div className="space-y-4">
            <div className="grid gap-3 md:grid-cols-3">
              <div className="space-y-2">
                <p className="text-xs text-slate-300">启用状态</p>
                <div className="flex items-center space-x-2 pt-1">
                  <Switch
                    id="editEnabled"
                    checked={editingDraft.enabled}
                    onCheckedChange={(checked) =>
                      updateDraft(editingTask.task_key, { enabled: checked })
                    }
                  />
                  <label
                    htmlFor="editEnabled"
                    className="cursor-pointer text-sm leading-none font-medium"
                  >
                    启用任务
                  </label>
                </div>
              </div>
              <div className="space-y-1">
                <p className="text-xs text-slate-300">Cron 表达式</p>
                <Input
                  value={editingDraft.cron_expr}
                  onChange={(event) =>
                    updateDraft(editingTask.task_key, { cron_expr: event.target.value })
                  }
                  placeholder="例如：0 30 3 * * *"
                />
              </div>
              <div className="space-y-1">
                <p className="text-xs text-slate-300">失败最大重试次数</p>
                <Input
                  type="number"
                  min={1}
                  max={20}
                  value={editingDraft.max_attempts}
                  onChange={(event) =>
                    updateDraft(editingTask.task_key, {
                      max_attempts: Number(event.target.value) || 1,
                    })
                  }
                  placeholder="1 - 20"
                />
              </div>
            </div>

            <div className="grid gap-3 md:grid-cols-2">
              <div className="space-y-1">
                <p className="text-xs text-slate-300">默认参数（JSON）</p>
                <Textarea
                  className="min-h-[120px] font-mono text-xs"
                  value={editingDraft.default_payload_text}
                  onChange={(event) =>
                    updateDraft(editingTask.task_key, {
                      default_payload_text: event.target.value,
                    })
                  }
                />
              </div>
              <div className="space-y-1">
                <p className="text-xs text-slate-300">本次运行覆盖参数（JSON）</p>
                <Textarea
                  className="min-h-[120px] font-mono text-xs"
                  value={editingDraft.run_payload_text}
                  onChange={(event) =>
                    updateDraft(editingTask.task_key, {
                      run_payload_text: event.target.value,
                    })
                  }
                />
              </div>
            </div>

            <div className="flex flex-wrap gap-2">
              <Button
                variant="secondary"
                onClick={() => void onSave(editingTask.task_key)}
                disabled={
                  savingKey === editingTask.task_key ||
                  Boolean(defaultPayloadError) ||
                  Boolean(runPayloadError)
                }
              >
                {savingKey === editingTask.task_key ? "保存中..." : "保存配置"}
              </Button>
              <Button
                onClick={() => void onRun(editingTask.task_key)}
                disabled={runningKey === editingTask.task_key || Boolean(runPayloadError)}
              >
                {runningKey === editingTask.task_key ? "触发中..." : "立即执行"}
              </Button>
              <Button
                variant="outline"
                onClick={() => updateDraft(editingTask.task_key, createDraft(editingTask))}
              >
                还原默认
              </Button>
            </div>
          </div>
        ) : null}
      </Modal>
    </div>
  );
}
