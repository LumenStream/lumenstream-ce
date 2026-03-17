import { motion } from "framer-motion";
import { useState, useMemo, useEffect } from "react";
import { Package, Calculator, Check, Star, Zap, Film, Tv, Loader2 } from "lucide-react";
import { GlowCard } from "@/components/effects/GlowCard";
import { AnimatedSection, TextReveal } from "@/components/effects/AnimatedSection";
import { Slider } from "@/components/ui/slider";
import { getPlans } from "@/lib/api/billing";
import type { Plan } from "@/lib/types/billing";

// Default features for plans since API doesn't provide them yet
const getPlanFeatures = (trafficGB: number) => {
  if (trafficGB < 0) return ["无限流量配额", "无限 4K 电影", "4K HDR 原画播放", "1对1 专属顾问"];
  const movies = Math.max(1, Math.round(trafficGB / 10));
  const trafficStr = `${trafficGB}GB 流量配额`;
  if (trafficGB < 200)
    return [trafficStr, `约 ${movies} 部 4K 电影`, "4K HDR 原画播放", "基础客服支持"];
  if (trafficGB < 800)
    return [trafficStr, `约 ${movies} 部 4K 电影`, "4K HDR 原画播放", "优先客服支持"];
  if (trafficGB < 1500)
    return [trafficStr, `约 ${movies} 部 4K 电影`, "4K HDR 原画播放", "专属客服通道"];
  return [trafficStr, `约 ${movies} 部 4K 电影`, "4K HDR 原画播放", "VIP 专属服务"];
};

export function PricingCompareSection() {
  const [apiPlans, setApiPlans] = useState<Plan[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    getPlans()
      .then((data) => {
        if (!cancelled) {
          // Sort by price
          const sorted = data.sort((a, b) => parseFloat(a.price) - parseFloat(b.price));
          console.log("[PricingCompareSection] loaded plans:", sorted);
          setApiPlans(sorted);
        }
      })
      .catch((err) => {
        console.error("Failed to load plans for comparison:", err);
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

  // Usage is now represented by movies and episodes per month
  const [movies, setMovies] = useState([10]); // movies per month
  const [episodes, setEpisodes] = useState([20]); // episodes per month

  // Calculate estimated traffic based on usage (Assume 10GB per movie, 2GB per episode in 4K)
  const estimatedTraffic = useMemo(() => {
    return movies[0] * 10 + episodes[0] * 2;
  }, [movies, episodes]);

  // Find recommended plan based on calculated traffic
  const recommendedPlan = useMemo(() => {
    if (apiPlans.length === 0) return null;

    // Find the smallest plan that covers the usage with some buffer (10%)
    const planWithBuffer = apiPlans.find((p) => {
      if (p.traffic_quota_bytes === -1) return true; // Unlimited plan
      const gb = p.traffic_quota_bytes / (1024 * 1024 * 1024);
      return gb >= estimatedTraffic * 1.1;
    });

    return planWithBuffer || apiPlans[apiPlans.length - 1];
  }, [estimatedTraffic, apiPlans]);

  // Get plans to display (recommended + adjacent)
  const displayPlans = useMemo(() => {
    if (apiPlans.length === 0 || !recommendedPlan) return [];

    const recIndex = apiPlans.findIndex((p) => p.id === recommendedPlan.id);
    const start = Math.max(0, recIndex - 1);
    const end = Math.min(apiPlans.length, start + 3);
    return apiPlans.slice(start, end);
  }, [recommendedPlan, apiPlans]);

  if (loading) {
    return (
      <section className="relative flex min-h-[400px] items-center justify-center px-4 py-24">
        <Loader2 className="h-8 w-8 animate-spin text-cyan-500" />
      </section>
    );
  }

  if (apiPlans.length === 0 && !loading) {
    return (
      <section className="relative px-4 py-24 text-center">
        <p className="text-muted-foreground">暂无可用套餐用于对比</p>
      </section>
    );
  }

  return (
    <section className="relative px-4 py-24">
      {/* Background */}
      <div className="pointer-events-none absolute inset-0 bg-gradient-to-b from-transparent via-cyan-500/5 to-transparent dark:via-cyan-900/5" />

      <div className="relative mx-auto max-w-6xl">
        {/* Section header */}
        <AnimatedSection className="mb-16 text-center">
          <motion.p
            className="mb-4 text-sm font-medium tracking-wider text-cyan-600 uppercase dark:text-cyan-400"
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
          >
            智能推荐
          </motion.p>
          <h2 className="text-foreground mb-4 text-4xl font-bold sm:text-5xl">
            <TextReveal text="按需选择，精准匹配" />
          </h2>
          <p className="text-muted-foreground mx-auto max-w-2xl text-lg">
            告诉我们您的观影习惯，我们为您推荐最合适的套餐（全套餐支持 4K 原画）
          </p>
        </AnimatedSection>

        {/* Interactive sliders */}
        <AnimatedSection className="mb-12">
          <GlowCard className="mx-auto max-w-2xl" glowColor="rgba(6, 182, 212, 0.2)">
            <div className="mb-8">
              <div className="mb-6 flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <Film className="h-5 w-5 text-cyan-600 dark:text-cyan-400" />
                  <span className="text-foreground font-medium">每月观看电影数量</span>
                </div>
                <div className="text-foreground text-2xl font-bold">
                  {movies[0]} <span className="text-muted-foreground text-base">部</span>
                </div>
              </div>
              <Slider
                value={movies}
                onValueChange={setMovies}
                max={100}
                min={0}
                step={5}
                className="mb-4"
              />
              <div className="text-muted-foreground flex justify-between text-sm">
                <span>0 部</span>
                <span>100 部</span>
              </div>
            </div>

            <div className="border-border/50 mb-6 border-t pt-8">
              <div className="mb-6 flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <Tv className="h-5 w-5 text-cyan-600 dark:text-cyan-400" />
                  <span className="text-foreground font-medium">每月观看剧集数量</span>
                </div>
                <div className="text-foreground text-2xl font-bold">
                  {episodes[0]} <span className="text-muted-foreground text-base">集</span>
                </div>
              </div>
              <Slider
                value={episodes}
                onValueChange={setEpisodes}
                max={300}
                min={0}
                step={10}
                className="mb-4"
              />
              <div className="text-muted-foreground flex justify-between text-sm">
                <span>0 集</span>
                <span>300 集</span>
              </div>
            </div>

            <div className="mt-8 rounded-lg bg-cyan-500/5 p-4 text-center">
              <p className="text-muted-foreground flex items-center justify-center gap-2 text-sm">
                <Calculator className="h-4 w-4" />
                预计每月消耗 4K 流量：
                <span className="font-semibold text-cyan-600 dark:text-cyan-400">
                  {estimatedTraffic} GB
                </span>
              </p>
            </div>
          </GlowCard>
        </AnimatedSection>

        {/* Recommended badge */}
        {recommendedPlan && (
          <AnimatedSection className="mb-6 text-center">
            <motion.div
              className="inline-flex items-center gap-2 rounded-full bg-cyan-500/10 px-4 py-2 text-sm font-medium text-cyan-600 dark:text-cyan-400"
              key={recommendedPlan.id}
              initial={{ scale: 0.9, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              transition={{ type: "spring", stiffness: 300 }}
            >
              <Star className="h-4 w-4" />
              根据您的观影习惯，推荐选择「{recommendedPlan.name}」
            </motion.div>
          </AnimatedSection>
        )}

        {/* Pricing cards */}
        <div className="grid gap-6 md:grid-cols-3">
          {displayPlans.map((plan, index) => {
            const isRecommended = plan.id === recommendedPlan?.id;
            const trafficGB =
              plan.traffic_quota_bytes === -1
                ? -1
                : plan.traffic_quota_bytes / (1024 * 1024 * 1024);
            const priceNum = parseFloat(plan.price);
            const pricePerGB = trafficGB > 0 && priceNum > 0 ? priceNum / trafficGB : 0;
            const features = getPlanFeatures(trafficGB);

            return (
              <AnimatedSection key={plan.id} delay={index * 0.1}>
                <GlowCard
                  className={`relative h-full ${isRecommended ? "border-cyan-500/50" : ""}`}
                  glowColor={isRecommended ? "rgba(6, 182, 212, 0.3)" : "rgba(59, 130, 246, 0.2)"}
                  hoverScale={1.02}
                >
                  {/* Recommended badge */}
                  {isRecommended && (
                    <div className="absolute -top-3 left-1/2 -translate-x-1/2">
                      <span className="flex items-center gap-1 rounded-full bg-cyan-500 px-3 py-1 text-xs font-medium text-white shadow-[0_0_10px_rgba(6,182,212,0.5)]">
                        <Zap className="h-3 w-3" />
                        推荐
                      </span>
                    </div>
                  )}

                  <div className="mb-4 flex items-center gap-3">
                    <div
                      className={`rounded-lg p-2 ${
                        isRecommended ? "bg-cyan-500/20" : "bg-blue-500/10"
                      }`}
                    >
                      <Package
                        className={`h-5 w-5 ${
                          isRecommended ? "text-cyan-600 dark:text-cyan-400" : "text-blue-500"
                        }`}
                      />
                    </div>
                    <h3 className="text-foreground font-semibold">{plan.name}</h3>
                  </div>

                  <div className="mb-4">
                    <div
                      className={`text-3xl font-bold ${
                        isRecommended ? "text-cyan-600 dark:text-cyan-400" : "text-foreground"
                      }`}
                    >
                      {priceNum === 0 ? "免费" : `¥${priceNum.toFixed(2)}`}
                      <span className="text-muted-foreground text-base font-normal">
                        /{plan.duration_days}天
                      </span>
                    </div>
                    {pricePerGB > 0 && (
                      <p className="text-muted-foreground mt-1 text-sm">
                        约 ¥{pricePerGB.toFixed(3)}/GB
                      </p>
                    )}
                  </div>

                  <ul className="space-y-3">
                    {features.map((feature) => (
                      <li key={feature} className="flex items-center gap-2 text-sm">
                        <Check
                          className={`h-4 w-4 shrink-0 ${
                            isRecommended ? "text-cyan-600 dark:text-cyan-400" : "text-blue-500"
                          }`}
                        />
                        <span className="text-foreground/80">{feature}</span>
                      </li>
                    ))}
                  </ul>

                  <motion.button
                    className={`mt-6 w-full rounded-lg py-2.5 text-sm font-medium transition-all duration-300 ${
                      isRecommended
                        ? "bg-cyan-600 text-white shadow-[0_0_15px_rgba(6,182,212,0.3)] hover:bg-cyan-500 hover:shadow-[0_0_20px_rgba(6,182,212,0.5)]"
                        : "bg-blue-500/10 text-blue-500 hover:bg-blue-500/20"
                    }`}
                    whileHover={{ scale: 1.02 }}
                    whileTap={{ scale: 0.98 }}
                    onClick={() => (window.location.href = "/app/store")}
                  >
                    {isRecommended ? "立即订阅" : "选择此套餐"}
                  </motion.button>
                </GlowCard>
              </AnimatedSection>
            );
          })}
        </div>
      </div>
    </section>
  );
}
