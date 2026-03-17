import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { Button } from "@/components/ui/button";
import { GlowCard } from "@/components/effects/GlowCard";
import { AnimatedSection } from "@/components/effects/AnimatedSection";
import { getPlans } from "@/lib/api/billing";
import type { Plan } from "@/lib/types/billing";
import type { ApiError } from "@/lib/api/client";
import { Check, Loader2, Sparkles } from "lucide-react";

function formatPrice(price: string): string {
  const amount = parseFloat(price);
  return amount === 0 ? "免费" : `¥${amount.toFixed(2)}`;
}

function formatTrafficBytes(bytes: number): string {
  if (bytes === -1) return "无限流量";
  const gb = bytes / (1024 * 1024 * 1024);
  return `${gb} GB`;
}

interface PricingCardProps {
  plan: Plan;
  index: number;
  isPopular?: boolean;
}

function PricingCard({ plan, index, isPopular }: PricingCardProps) {
  const glowColors: Record<string, string> = {
    basic: "rgba(148, 163, 184, 0.3)",
    standard: "rgba(59, 130, 246, 0.3)",
    premium: "rgba(6, 182, 212, 0.4)",
    annual: "rgba(14, 165, 233, 0.3)",
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 40 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{
        duration: 0.5,
        delay: index * 0.1,
        ease: [0.22, 1, 0.36, 1],
      }}
    >
      <GlowCard
        glowColor={glowColors[plan.code] || glowColors.basic}
        className={`relative flex h-full flex-col ${isPopular ? "ring-2 ring-cyan-500/50" : ""}`}
      >
        {isPopular && (
          <div className="absolute -top-3 left-1/2 -translate-x-1/2">
            <span className="flex items-center gap-1 rounded-full bg-gradient-to-r from-cyan-500 to-blue-500 px-3 py-1 text-xs font-medium text-white shadow-[0_0_10px_rgba(6,182,212,0.5)]">
              <Sparkles className="h-3 w-3" />
              最受欢迎
            </span>
          </div>
        )}

        <div className="mb-4">
          <h3 className="text-foreground text-xl font-semibold">{plan.name}</h3>
          <p className="text-muted-foreground text-sm">{plan.code.toUpperCase()}</p>
        </div>

        <div className="mb-6">
          <span className="text-foreground text-4xl font-bold">{formatPrice(plan.price)}</span>
          <span className="text-muted-foreground ml-2 text-sm">
            / {plan.duration_days === 365 ? "年" : `${plan.duration_days}天`}
          </span>
        </div>

        <ul className="mb-6 flex-1 space-y-3">
          <li className="text-muted-foreground flex items-center gap-3">
            <Check className="h-5 w-5 flex-shrink-0 text-cyan-500" />
            <span>{formatTrafficBytes(plan.traffic_quota_bytes)} 流量配额</span>
          </li>
          <li className="text-muted-foreground flex items-center gap-3">
            <Check className="h-5 w-5 flex-shrink-0 text-cyan-500" />
            <span>{plan.traffic_window_days} 天流量周期</span>
          </li>
          <li className="text-muted-foreground flex items-center gap-3">
            <Check className="h-5 w-5 flex-shrink-0 text-cyan-500" />
            <span>高清流媒体播放</span>
          </li>
          <li className="text-muted-foreground flex items-center gap-3">
            <Check className="h-5 w-5 flex-shrink-0 text-cyan-500" />
            <span>多设备同步</span>
          </li>
        </ul>

        <Button
          className={`w-full ${
            isPopular
              ? "bg-gradient-to-r from-cyan-600 to-cyan-500 text-white shadow-[0_0_15px_rgba(6,182,212,0.3)] transition-all duration-300 hover:from-cyan-500 hover:to-cyan-400 hover:shadow-[0_0_20px_rgba(6,182,212,0.5)]"
              : "bg-card/50 border-border/50 text-foreground hover:bg-accent/50 border backdrop-blur-sm"
          }`}
          onClick={() => (window.location.href = "/app/store")}
        >
          选择套餐
        </Button>
      </GlowCard>
    </motion.div>
  );
}

export function PricingSection() {
  const [plans, setPlans] = useState<Plan[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    getPlans()
      .then((data) => {
        if (!cancelled) {
          // Sort: premium first, then by price
          const sorted = data.sort((a, b) => {
            if (a.code === "premium") return -1;
            if (b.code === "premium") return 1;
            return parseFloat(a.price) - parseFloat(b.price);
          });
          setPlans(sorted);
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

  return (
    <section className="relative px-4 py-20">
      {/* Section header */}
      <AnimatedSection className="mb-12 text-center">
        <motion.span
          className="mb-4 inline-block rounded-full border border-cyan-500/30 bg-cyan-950/30 px-4 py-1 text-sm font-medium text-cyan-400 shadow-[0_0_10px_rgba(6,182,212,0.2)]"
          initial={{ opacity: 0, scale: 0.8 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ duration: 0.4 }}
        >
          灵活定价
        </motion.span>
        <h2 className="text-foreground mb-4 text-3xl font-bold sm:text-4xl">选择适合您的套餐</h2>
        <p className="text-muted-foreground mx-auto max-w-2xl">
          所有套餐均包含高清流媒体播放、智能缓存与多终端支持。 按需选择，随时升级。
        </p>
      </AnimatedSection>

      {/* Loading state */}
      {loading && (
        <div className="flex flex-col items-center justify-center py-12">
          <Loader2 className="mb-4 h-8 w-8 animate-spin text-cyan-500" />
          <p className="text-muted-foreground">正在加载套餐信息...</p>
        </div>
      )}

      {/* Error state */}
      {error && (
        <AnimatedSection className="rounded-xl border border-red-900/30 bg-red-950/20 p-6 text-center">
          <p className="text-red-400">{error}</p>
          <Button
            variant="outline"
            className="mt-4 border-red-900/50 text-red-400 hover:bg-red-950/30"
            onClick={() => window.location.reload()}
          >
            重试
          </Button>
        </AnimatedSection>
      )}

      {/* Plans grid */}
      {!loading && !error && plans.length > 0 && (
        <div className="mx-auto grid max-w-6xl gap-6 sm:grid-cols-2 lg:grid-cols-4">
          {plans.map((plan, index) => (
            <PricingCard
              key={plan.id}
              plan={plan}
              index={index}
              isPopular={plan.code === "premium"}
            />
          ))}
        </div>
      )}

      {/* Empty state */}
      {!loading && !error && plans.length === 0 && (
        <AnimatedSection className="border-border/50 bg-card/30 rounded-xl border p-8 text-center backdrop-blur-md">
          <p className="text-muted-foreground">暂无可用套餐，请稍后再试。</p>
        </AnimatedSection>
      )}

      {/* Bottom CTA */}
      <AnimatedSection delay={0.4} className="mt-12 text-center">
        <p className="text-muted-foreground mb-4">需要更多流量或自定义方案？</p>
        <Button
          variant="outline"
          className="border-border bg-card/50 text-foreground hover:bg-accent/50 backdrop-blur-sm"
          onClick={() => (window.location.href = "/app/store")}
        >
          查看完整商店
        </Button>
      </AnimatedSection>
    </section>
  );
}
