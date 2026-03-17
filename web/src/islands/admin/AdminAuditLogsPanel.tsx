import React, { useEffect, useState } from "react";

import { ErrorState, LoadingState } from "@/components/domain/DataState";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { buildAuditExportUrl, buildMockAuditExportCsv, listAuditLogs } from "@/lib/api/admin";
import type { ApiError } from "@/lib/api/client";
import { getPublicSystemCapabilities } from "@/lib/api/system";
import { getAccessToken } from "@/lib/auth/token";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import { isMockFeatureEnabled, isMockMode } from "@/lib/mock/mode";
import { toast } from "@/lib/notifications/toast-store";
import type { AdminSystemCapabilities, AuditLogEntry } from "@/lib/types/admin";
import { formatDate } from "@/lib/utils";

function downloadCsv(filename: string, content: string): void {
  const blob = new Blob([content], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}

export function AdminAuditLogsPanel() {
  const mockFeatureEnabled = isMockFeatureEnabled();
  const { ready } = useAuthSession({ requireAdmin: true });
  const [capabilities, setCapabilities] = useState<AdminSystemCapabilities | null>(null);
  const [logs, setLogs] = useState<AuditLogEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [limit, setLimit] = useState(200);

  const reload = React.useCallback(
    async (nextLimit: number, options?: { preserveData?: boolean }) => {
      if (options?.preserveData) {
        setRefreshing(true);
      } else {
        setLoading(true);
      }
      try {
        const payload = await listAuditLogs(nextLimit);
        setLogs(payload);
        setError(null);
        if (options?.preserveData) {
          toast.info(`已刷新审计日志（limit=${nextLimit}）`);
        }
      } catch (cause) {
        const apiError = cause as ApiError;
        setError(apiError.message || "加载审计日志失败");
      } finally {
        if (options?.preserveData) {
          setRefreshing(false);
        } else {
          setLoading(false);
        }
      }
    },
    []
  );

  useEffect(() => {
    if (!ready) {
      return;
    }
    void getPublicSystemCapabilities()
      .then(setCapabilities)
      .catch(() => setCapabilities(null));
    void reload(200);
  }, [ready, reload]);

  function onExport() {
    if (!capabilities?.audit_log_export_enabled) {
      toast.info("CSV 导出仅在商业版提供。");
      return;
    }

    const normalizedLimit = Math.max(1, Math.min(limit, 5000));
    setExporting(true);

    try {
      if (isMockMode()) {
        const csv = buildMockAuditExportCsv(normalizedLimit);
        downloadCsv("admin-audit-logs-mock.csv", csv);
        toast.success(`已导出 mock CSV（limit=${normalizedLimit}）`);
        return;
      }

      const token = getAccessToken();
      if (!token) {
        toast.error("导出失败：缺少访问令牌。");
        return;
      }

      const url = `${buildAuditExportUrl(normalizedLimit)}&api_key=${encodeURIComponent(token)}`;
      const opened = window.open(url, "_blank", "noopener,noreferrer");
      if (opened) {
        toast.success("已打开导出链接。");
      } else {
        toast.warning("导出链接被浏览器拦截。");
      }
    } finally {
      setExporting(false);
    }
  }

  if (!ready || loading) {
    return <LoadingState title="加载审计日志" />;
  }

  if (error) {
    return <ErrorState title="审计日志加载失败" description={error} />;
  }

  return (
    <div className="space-y-8">
      <section>
        <h3 className="text-sm font-medium">审计日志查询</h3>
        <p className="text-muted-foreground mt-1 mb-4 text-xs">
          {capabilities?.audit_log_export_enabled
            ? mockFeatureEnabled
              ? "支持导出 CSV。Mock 模式下会导出本地生成的演示数据。"
              : "支持导出 CSV。"
            : "社区版保留审计查询，CSV 导出已迁移到商业版。"}
        </p>
        <div className="space-y-3">
          <div className="flex flex-wrap items-center gap-2">
            <Input
              type="number"
              className="w-40"
              min={1}
              max={1000}
              value={limit}
              onChange={(event) => setLimit(Number(event.target.value || 100))}
            />
            <Button
              onClick={() => void reload(limit, { preserveData: true })}
              disabled={refreshing}
            >
              {refreshing ? "刷新中..." : "刷新"}
            </Button>
            <Button
              variant="secondary"
              onClick={onExport}
              disabled={exporting || !capabilities?.audit_log_export_enabled}
            >
              {exporting ? "导出中..." : "导出 CSV"}
            </Button>
          </div>
          <Table className="min-w-[760px]">
            <TableHeader>
              <TableRow>
                <TableHead>时间</TableHead>
                <TableHead>操作人</TableHead>
                <TableHead>动作</TableHead>
                <TableHead>对象</TableHead>
                <TableHead>详情</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {logs.map((entry) => (
                <TableRow key={entry.id}>
                  <TableCell>{formatDate(entry.created_at)}</TableCell>
                  <TableCell>{entry.actor_username || "system"}</TableCell>
                  <TableCell>{entry.action}</TableCell>
                  <TableCell>
                    {entry.target_type}
                    {entry.target_id ? `/${entry.target_id}` : ""}
                  </TableCell>
                  <TableCell className="text-muted-foreground max-w-md overflow-hidden font-mono text-[10px] text-ellipsis whitespace-nowrap">
                    {JSON.stringify(entry.detail)}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      </section>
    </div>
  );
}
