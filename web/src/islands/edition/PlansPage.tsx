import { useEffect, useState } from "react";

import { ErrorState, LoadingState } from "@/components/domain/DataState";
import { FeatureUnavailable } from "@/components/edition/FeatureUnavailable";
import { PlanStore } from "@/islands/billing/PlanStore";
import { getPublicSystemCapabilities } from "@/lib/api/system";
import type { ApiError } from "@/lib/api/client";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import type { AdminSystemCapabilities } from "@/lib/types/admin";

export function PlansPage() {
  const { ready } = useAuthSession();
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
    return <LoadingState title="加载套餐商店" />;
  }

  if (error) {
    return <ErrorState title="套餐页加载失败" description={error} />;
  }

  if (!capabilities?.billing_enabled) {
    return (
      <FeatureUnavailable
        title="套餐商店已切出社区版"
        description="社区版保留媒体服务、刮削、Agent 和多端推流；在线充值、钱包与套餐订阅功能已迁移到商业版。"
        href="/app/profile"
        linkLabel="返回账户中心"
      />
    );
  }

  return <PlanStore />;
}
