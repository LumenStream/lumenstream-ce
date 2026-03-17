import { useEffect, useState } from "react";

import { ErrorState, LoadingState } from "@/components/domain/DataState";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { getPlans } from "@/lib/api/billing";
import type { ApiError } from "@/lib/api/client";
import type { Plan } from "@/lib/types/billing";

function formatPrice(price: string): string {
  const amount = parseFloat(price);
  return `¥${amount.toFixed(2)}`;
}

function formatTrafficBytes(bytes: number): string {
  const gb = bytes / (1024 * 1024 * 1024);
  return `${gb.toFixed(0)} GB`;
}

interface PlanListProps {
  onSelectPlan?: (plan: Plan) => void;
}

export function PlanList({ onSelectPlan }: PlanListProps) {
  const [plans, setPlans] = useState<Plan[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    getPlans()
      .then((data) => {
        if (!cancelled) {
          setPlans(data);
          setError(null);
        }
      })
      .catch((cause) => {
        if (!cancelled) {
          const apiError = cause as ApiError;
          setError(apiError.message || "加载套餐失败");
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
  }, []);

  if (loading) {
    return <LoadingState title="加载套餐" description="正在获取可用套餐列表。" />;
  }

  if (error) {
    return <ErrorState title="套餐加载失败" description={error} />;
  }

  if (plans.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>暂无可用套餐</CardTitle>
          <CardDescription>请稍后再试。</CardDescription>
        </CardHeader>
      </Card>
    );
  }

  return (
    <div className="space-y-4">
      <h2 className="text-xl font-semibold">选择套餐</h2>
      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        {plans.map((plan) => (
          <Card key={plan.id} className="flex flex-col">
            <CardHeader>
              <CardTitle className="text-lg">{plan.name}</CardTitle>
              <CardDescription>{plan.code}</CardDescription>
            </CardHeader>
            <CardContent className="flex flex-1 flex-col justify-between space-y-4">
              <div className="space-y-3">
                <p className="text-2xl font-semibold">
                  {formatPrice(plan.price)}
                  <span className="text-muted-foreground ml-1 text-sm font-normal">
                    / {plan.duration_days === 365 ? "年" : `${plan.duration_days}天`}
                  </span>
                </p>
                <ul className="text-muted-foreground space-y-1 text-sm">
                  <li className="flex items-center gap-2">
                    <span className="text-green-500">✓</span>
                    {formatTrafficBytes(plan.traffic_quota_bytes)} 流量配额
                  </li>
                  <li className="flex items-center gap-2">
                    <span className="text-green-500">✓</span>
                    {plan.traffic_window_days} 天流量周期
                  </li>
                </ul>
              </div>
              {onSelectPlan ? (
                <Button
                  className="w-full"
                  variant={plan.code === "premium" ? "default" : "secondary"}
                  onClick={() => onSelectPlan(plan)}
                >
                  选择此套餐
                </Button>
              ) : null}
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}
