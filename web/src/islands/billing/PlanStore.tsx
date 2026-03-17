import { useEffect, useState } from "react";

import { PlanList } from "@/islands/billing/PlanList";
import { PurchaseConfirm } from "@/islands/billing/PurchaseConfirm";
import { RechargeFlow } from "@/islands/billing/RechargeFlow";
import { WalletDisplay } from "@/islands/billing/WalletDisplay";
import { getWallet } from "@/lib/api/billing";
import type { Plan, PurchaseResult } from "@/lib/types/billing";

export function PlanStore() {
  const [selectedPlan, setSelectedPlan] = useState<Plan | null>(null);
  const [balance, setBalance] = useState<string>("0.00");
  const [showRecharge, setShowRecharge] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    let cancelled = false;
    getWallet()
      .then((wallet) => {
        if (!cancelled) {
          setBalance(wallet.balance);
        }
      })
      .catch(() => {
        // Wallet display will show error state
      });
    return () => {
      cancelled = true;
    };
  }, [refreshKey]);

  function handleSelectPlan(plan: Plan) {
    setSelectedPlan(plan);
  }

  function handlePurchaseSuccess(result: PurchaseResult) {
    setBalance(result.wallet.balance);
    setRefreshKey((k) => k + 1);
  }

  function handleRechargeSuccess() {
    setShowRecharge(false);
    setRefreshKey((k) => k + 1);
  }

  return (
    <div className="space-y-6">
      <section className="shadow-card rounded-2xl border border-rose-900/40 bg-gradient-to-r from-rose-900/60 via-violet-900/50 to-blue-900/60 px-6 py-6">
        <div className="space-y-2">
          <h1 className="text-3xl font-semibold tracking-tight">套餐商店</h1>
          <p className="text-sm text-rose-100/85">选择适合您的套餐，享受高清流媒体服务。</p>
        </div>
      </section>

      <div className="grid gap-6 xl:grid-cols-4">
        <div className="xl:col-span-3">
          <PlanList key={`plans-${refreshKey}`} onSelectPlan={handleSelectPlan} />
        </div>
        <div className="space-y-4">
          <WalletDisplay key={`wallet-${refreshKey}`} onRecharge={() => setShowRecharge(true)} />
        </div>
      </div>

      <PurchaseConfirm
        plan={selectedPlan}
        balance={balance}
        onClose={() => setSelectedPlan(null)}
        onSuccess={handlePurchaseSuccess}
        onNeedRecharge={() => {
          setSelectedPlan(null);
          setShowRecharge(true);
        }}
      />

      <RechargeFlow
        open={showRecharge}
        onClose={() => setShowRecharge(false)}
        onSuccess={handleRechargeSuccess}
      />
    </div>
  );
}
