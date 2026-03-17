import { useEffect, useState } from "react";

import { ErrorState, LoadingState } from "@/components/domain/DataState";
import { Badge } from "@/components/ui/badge";
import { getSystemCapabilities, getSystemFlags, getSystemSummary } from "@/lib/api/admin";
import type { ApiError } from "@/lib/api/client";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import type {
  AdminSystemCapabilities,
  AdminSystemFlags,
  AdminSystemSummary,
} from "@/lib/types/admin";

interface OverviewState {
  summary: AdminSystemSummary;
  flags: AdminSystemFlags;
  capabilities: AdminSystemCapabilities;
}

export function AdminOverviewPanel() {
  const { ready } = useAuthSession({ requireAdmin: true });
  const [state, setState] = useState<OverviewState | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!ready) {
      return;
    }

    let cancelled = false;
    setLoading(true);

    Promise.all([getSystemSummary(), getSystemFlags(), getSystemCapabilities()])
      .then(([summary, flags, capabilities]) => {
        if (cancelled) {
          return;
        }
        setState({ summary, flags, capabilities });
        setError(null);
      })
      .catch((cause) => {
        if (cancelled) {
          return;
        }
        const apiError = cause as ApiError;
        setError(apiError.message || "加载总览失败");
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [ready]);

  if (!ready || loading) {
    return <LoadingState title="加载系统总览" />;
  }

  if (error || !state) {
    return <ErrorState title="总览加载失败" description={error || "未知错误"} />;
  }

  const { summary, flags, capabilities } = state;

  return (
    <div className="space-y-8">
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <Metric label="媒体总量" value={summary.media_items_total} />
        <Metric label="用户总量" value={summary.users_total} />
        <Metric label="活跃播放会话" value={summary.active_playback_sessions} />
        <Metric label="活跃鉴权会话" value={summary.active_auth_sessions} />
      </div>

      <section className="border-border/50 border-b pb-6">
        <h3 className="text-sm font-medium">系统开关</h3>
        <p className="text-muted-foreground mt-1 mb-4 text-xs">
          当前配置从 `/admin/system/flags` 与 `/admin/system/capabilities` 拉取。
        </p>
        <div className="space-y-2 text-sm">
          <div className="flex flex-wrap gap-2">
            <Badge variant={flags.strm_only_streaming ? "success" : "outline"}>
              STRM Only: {String(flags.strm_only_streaming)}
            </Badge>
            <Badge variant={flags.transcoding_enabled ? "danger" : "success"}>
              Transcoding: {String(flags.transcoding_enabled)}
            </Badge>
            <Badge variant={flags.scraper_enabled ? "secondary" : "outline"}>
              Scraper: {String(flags.scraper_enabled)}
            </Badge>
          </div>
          <p className="text-muted-foreground text-xs">
            版本：{capabilities.edition.toUpperCase()} · 支持能力：
            {capabilities.supported_stream_features.join(", ")}
          </p>
        </div>
      </section>

      <section>
        <h3 className="text-sm font-medium">作业状态分布</h3>
        <p className="text-muted-foreground mt-1 mb-4 text-xs">用于快速判断任务堆积与失败趋势。</p>
        <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
          {Object.entries(summary.jobs_by_status).map(([status, count]) => (
            <div key={status} className="border-border bg-card/50 rounded-lg border px-3 py-2">
              <p className="text-muted-foreground text-xs tracking-wide uppercase">{status}</p>
              <p className="text-foreground/90 mt-1 text-xl font-semibold">{count}</p>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: number }) {
  return (
    <div className="light:bg-black/[0.02] light:border-black/[0.06] rounded-xl border border-white/[0.04] bg-white/[0.02] px-4 py-3">
      <p className="text-muted-foreground text-xs">{label}</p>
      <p className="text-foreground mt-1 text-3xl font-semibold">{value}</p>
    </div>
  );
}
