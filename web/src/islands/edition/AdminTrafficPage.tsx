import { useEffect, useState } from "react";

import { ErrorState, LoadingState } from "@/components/domain/DataState";
import { FeatureUnavailable } from "@/components/edition/FeatureUnavailable";
import { AdminTrafficPanel } from "@/islands/admin/AdminTrafficPanel";
import { getPublicSystemCapabilities } from "@/lib/api/system";
import type { ApiError } from "@/lib/api/client";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import type { AdminSystemCapabilities } from "@/lib/types/admin";

export function AdminTrafficPage() {
  const { ready } = useAuthSession({ requireAdmin: true });
  const [capabilities, setCapabilities] = useState<AdminSystemCapabilities | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!ready) return;
    let cancelled = false;
    setLoading(true);

    getPublicSystemCapabilities()
      .then((payload) => {
        if (!cancelled) {
          setCapabilities(payload);
          setError(null);
        }
      })
      .catch((cause) => {
        if (!cancelled) {
          setError((cause as ApiError).message || "加载版本能力失败");
        }
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
    return <LoadingState title="加载流量能力" />;
  }

  if (error) {
    return <ErrorState title="流量页加载失败" description={error} />;
  }

  if (!capabilities?.advanced_traffic_controls_enabled) {
    return (
      <FeatureUnavailable
        title="高级流量策略已切出社区版"
        description="CE 仍保留基础播放和多端推流，但按用户流量配额、统计重置与 Top 用量运营面板已迁移到商业版。"
        href="/admin/playback"
        linkLabel="转到推流与域名"
      />
    );
  }

  return <AdminTrafficPanel />;
}
