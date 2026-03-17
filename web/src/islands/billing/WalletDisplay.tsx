import { useEffect, useState } from "react";

import { ErrorState, LoadingState } from "@/components/domain/DataState";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { getWallet } from "@/lib/api/billing";
import type { ApiError } from "@/lib/api/client";
import type { Wallet } from "@/lib/types/billing";

function formatBalance(balance: string): string {
  const amount = Number.parseFloat(balance);
  return `¥${amount.toFixed(2)}`;
}

interface WalletDisplayProps {
  onRecharge?: () => void;
}

export function WalletDisplay({ onRecharge }: WalletDisplayProps) {
  const [wallet, setWallet] = useState<Wallet | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    getWallet()
      .then((data) => {
        if (!cancelled) {
          setWallet(data);
          setError(null);
        }
      })
      .catch((cause) => {
        if (!cancelled) {
          const apiError = cause as ApiError;
          setError(apiError.message || "加载钱包失败");
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
    return <LoadingState title="加载钱包" description="正在获取余额信息。" />;
  }

  if (error || !wallet) {
    return <ErrorState title="钱包加载失败" description={error || "未知错误"} />;
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>账户余额</CardTitle>
        <CardDescription>当前可用于购买套餐的余额。</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <p className="text-center text-4xl font-semibold tracking-tight">
          {formatBalance(wallet.balance)}
        </p>
        {onRecharge ? (
          <button
            type="button"
            onClick={onRecharge}
            className="w-full rounded-md bg-rose-700 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-rose-600"
          >
            充值余额
          </button>
        ) : null}
      </CardContent>
    </Card>
  );
}
