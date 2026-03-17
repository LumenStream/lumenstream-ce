import { Button, buttonVariants } from "@/components/ui/button";
import { StaggerContainer, StaggerItem } from "@/components/effects/AnimatedSection";
import { TrafficGauge } from "../components/TrafficGauge";
import { formatBytes, formatBalance } from "../utils";
import type { Wallet } from "@/lib/types/billing";
import type { MyTrafficUsageMediaSummary } from "@/lib/types/edition-commercial";

interface BillingTrafficSectionProps {
  trafficUsage: MyTrafficUsageMediaSummary | null;
  wallet: Wallet | null;
  activeTrafficMultiplier: number;
  onRechargeOpen: () => void;
}

export function BillingTrafficSection({
  trafficUsage,
  wallet,
  activeTrafficMultiplier,
  onRechargeOpen,
}: BillingTrafficSectionProps) {
  const usageWindowDays = trafficUsage?.window_days ?? 30;
  const usedBytes = trafficUsage?.used_bytes ?? 0;
  const realUsedBytes = trafficUsage?.real_used_bytes ?? usedBytes;
  const quotaBytes = trafficUsage?.quota_bytes ?? null;
  const remainingBytes = trafficUsage?.remaining_bytes ?? null;

  return (
    <div className="space-y-6">
      {/* Hero metrics row */}
      <div className="flex flex-col gap-8 pb-8 md:flex-row md:items-center md:gap-12">
        <div className="w-full max-w-[240px]">
          <TrafficGauge usedBytes={usedBytes} quotaBytes={quotaBytes} className="mx-auto" />
        </div>
        <div className="flex flex-col justify-center">
          <p className="text-muted-foreground text-xs tracking-wide uppercase">账户余额</p>
          <p className="mt-1 text-4xl font-semibold tracking-tight">
            {wallet ? formatBalance(wallet.balance) : "-"}
          </p>
          <div className="mt-5 flex gap-3">
            <Button variant="default" onClick={onRechargeOpen}>
              充值余额
            </Button>
            <a href="/app/plans" className={buttonVariants({ variant: "outline" })}>
              浏览套餐
            </a>
          </div>
        </div>
      </div>

      {/* Stat metric tiles */}
      <div className="border-border/50 mb-5 border-y py-6">
        <StaggerContainer className="grid grid-cols-2 gap-x-6 gap-y-8 sm:grid-cols-3">
          {[
            { label: "统计窗口", value: usageWindowDays, suffix: "天" },
            { label: "计费流量", value: formatBytes(usedBytes) },
            { label: "真实流量", value: formatBytes(realUsedBytes) },
            { label: "流量配额", value: quotaBytes === null ? "不限额" : formatBytes(quotaBytes) },
            {
              label: "剩余可用",
              value: remainingBytes === null ? "-" : formatBytes(remainingBytes),
            },
            { label: "线路倍率", value: `x${activeTrafficMultiplier.toFixed(2)}` },
          ].map((m) => (
            <StaggerItem key={m.label}>
              <div>
                <p className="text-muted-foreground text-xs tracking-wide">{m.label}</p>
                <p className="mt-1 text-2xl font-semibold tracking-tight">
                  {m.value}
                  {m.suffix && (
                    <span className="text-muted-foreground ml-1 text-sm font-normal">
                      {m.suffix}
                    </span>
                  )}
                </p>
              </div>
            </StaggerItem>
          ))}
        </StaggerContainer>
      </div>
    </div>
  );
}
