import { useEffect, useState } from "react";

import { FeatureUnavailable } from "@/components/edition/FeatureUnavailable";
import { ErrorState, LoadingState } from "@/components/domain/DataState";
import { AdminBillingPanel } from "@/islands/admin/AdminBillingPanel";
import { getPublicSystemCapabilities } from "@/lib/api/system";
import type { ApiError } from "@/lib/api/client";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import type { AdminSystemCapabilities } from "@/lib/types/admin";

export function AdminBillingPage() {
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
    return <LoadingState title="加载账单能力" />;
  }

  if (error) {
    return <ErrorState title="账单页加载失败" description={error} />;
  }

  if (!capabilities?.billing_enabled) {
    return (
      <FeatureUnavailable
        title="账单与订阅已切出社区版"
        description="CE 保留媒体服务核心、Agent 与推流域名能力；账单、钱包、订阅和充值链路已迁移到商业版。"
        href="/admin/overview"
        linkLabel="返回系统总览"
      />
    );
  }

  return <AdminBillingPanel />;
}
