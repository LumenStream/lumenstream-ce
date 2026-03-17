import { useState } from "react";

import { Modal } from "@/components/domain/Modal";
import { Button } from "@/components/ui/button";
import { purchasePlan } from "@/lib/api/billing";
import type { ApiError } from "@/lib/api/client";
import { toast } from "@/lib/notifications/toast-store";
import type { Plan, PurchaseResult } from "@/lib/types/billing";

function formatPrice(price: string): string {
  const amount = parseFloat(price);
  return `¥${amount.toFixed(2)}`;
}

function formatDate(isoString: string): string {
  const date = new Date(isoString);
  return date.toLocaleDateString("zh-CN", {
    year: "numeric",
    month: "long",
    day: "numeric",
  });
}

function formatTrafficBytes(bytes: number): string {
  const gb = bytes / (1024 * 1024 * 1024);
  return `${gb.toFixed(0)} GB`;
}

interface PurchaseConfirmProps {
  plan: Plan | null;
  balance: string;
  onClose: () => void;
  onSuccess?: (result: PurchaseResult) => void;
  onNeedRecharge?: () => void;
}

export function PurchaseConfirm({
  plan,
  balance,
  onClose,
  onSuccess,
  onNeedRecharge,
}: PurchaseConfirmProps) {
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<PurchaseResult | null>(null);

  if (!plan) {
    return null;
  }

  const balanceNum = parseFloat(balance);
  const priceNum = parseFloat(plan.price);
  const canAfford = balanceNum >= priceNum;

  async function handlePurchase() {
    if (!plan) return;

    setLoading(true);

    try {
      const purchaseResult = await purchasePlan(plan.id);
      setResult(purchaseResult);
      onSuccess?.(purchaseResult);
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(apiError.message || "购买失败");
    } finally {
      setLoading(false);
    }
  }

  function handleClose() {
    setResult(null);
    onClose();
  }

  // Show success result
  if (result) {
    return (
      <Modal open={true} title="购买成功" onClose={handleClose}>
        <div className="space-y-4 text-center">
          <div className="text-4xl">🎉</div>
          <p className="text-lg font-medium">成功购买 {result.subscription.plan_name}</p>
          <p className="text-muted-foreground text-sm">
            有效期至：{formatDate(result.subscription.expires_at)}
          </p>
          <p className="text-muted-foreground text-sm">
            剩余余额：{formatPrice(result.wallet.balance)}
          </p>
          <Button className="w-full" onClick={handleClose}>
            完成
          </Button>
        </div>
      </Modal>
    );
  }

  return (
    <Modal
      open={true}
      title="确认购买"
      description={`您正在购买「${plan.name}」`}
      onClose={handleClose}
    >
      <div className="space-y-4">
        <div className="border-border bg-background/50 space-y-3 rounded-lg border p-4">
          <div className="flex justify-between text-sm">
            <span className="text-muted-foreground">套餐名称</span>
            <span className="font-medium">{plan.name}</span>
          </div>
          <div className="flex justify-between text-sm">
            <span className="text-muted-foreground">有效期</span>
            <span>{plan.duration_days === 365 ? "1 年" : `${plan.duration_days} 天`}</span>
          </div>
          <div className="flex justify-between text-sm">
            <span className="text-muted-foreground">流量配额</span>
            <span>{formatTrafficBytes(plan.traffic_quota_bytes)}</span>
          </div>
          <div className="border-border flex justify-between border-t pt-3">
            <span className="text-muted-foreground">套餐价格</span>
            <span className="text-lg font-semibold">{formatPrice(plan.price)}</span>
          </div>
        </div>

        <div className="border-border bg-background/50 space-y-2 rounded-lg border p-4">
          <div className="flex justify-between text-sm">
            <span className="text-muted-foreground">当前余额</span>
            <span className={canAfford ? "text-green-500" : "text-red-400"}>
              {formatPrice(balance)}
            </span>
          </div>
          {!canAfford && (
            <p className="text-xs text-red-400">
              余额不足，还需充值 {formatPrice((priceNum - balanceNum).toFixed(2))}
            </p>
          )}
        </div>

        <div className="flex gap-2">
          {canAfford ? (
            <Button className="flex-1" onClick={handlePurchase} disabled={loading}>
              {loading ? "处理中..." : "确认购买"}
            </Button>
          ) : (
            <Button className="flex-1" onClick={onNeedRecharge}>
              去充值
            </Button>
          )}
          <Button variant="secondary" onClick={handleClose}>
            取消
          </Button>
        </div>
      </div>
    </Modal>
  );
}
