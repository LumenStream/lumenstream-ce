import { motion } from "framer-motion";
import { Film, Sparkles, FolderOpen, RefreshCw } from "lucide-react";
import { GlowCard } from "@/components/effects/GlowCard";
import {
  AnimatedSection,
  StaggerContainer,
  StaggerItem,
  TextReveal,
} from "@/components/effects/AnimatedSection";
import { CountUpNumber } from "@/components/effects/CountUpNumber";

const stats = [
  {
    icon: Film,
    value: 10000,
    suffix: "+",
    label: "视频内容",
    description: "海量影视资源持续更新",
    color: "rgba(6, 182, 212, 0.3)", // cyan-500
  },
  {
    icon: Sparkles,
    value: 4,
    suffix: "K HDR",
    label: "超清画质",
    description: "支持 4K HDR 高清播放",
    color: "rgba(14, 165, 233, 0.3)", // sky-500
  },
  {
    icon: FolderOpen,
    value: 100,
    suffix: "+",
    label: "内容分类",
    description: "电影、剧集、综艺、动漫",
    color: "rgba(59, 130, 246, 0.3)", // blue-500
  },
  {
    icon: RefreshCw,
    value: 24,
    suffix: "/7",
    label: "持续更新",
    description: "全天候自动同步新内容",
    color: "rgba(148, 163, 184, 0.3)", // slate-400
  },
];

export function ContentStatsSection() {
  return (
    <section className="relative px-4 py-24">
      {/* Background gradient */}
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
            内容资源
          </motion.p>
          <h2 className="text-foreground mb-4 text-4xl font-bold sm:text-5xl">
            <TextReveal text="海量内容，应有尽有" />
          </h2>
          <p className="text-muted-foreground mx-auto max-w-2xl text-lg">
            覆盖全品类影视资源，从经典老片到最新热播，一站式满足您的观影需求
          </p>
        </AnimatedSection>

        {/* Stats grid */}
        <StaggerContainer className="grid gap-6 sm:grid-cols-2 lg:grid-cols-4">
          {stats.map((stat) => (
            <StaggerItem key={stat.label}>
              <GlowCard className="h-full text-center" glowColor={stat.color} hoverScale={1.03}>
                <div className="mb-4 flex justify-center">
                  <div
                    className="rounded-xl p-3"
                    style={{
                      background: stat.color.replace("0.3", "0.15"),
                    }}
                  >
                    <stat.icon
                      className="h-6 w-6"
                      style={{
                        color: stat.color.replace("0.3", "1"),
                      }}
                    />
                  </div>
                </div>
                <div className="text-foreground mb-2 text-4xl font-bold">
                  <CountUpNumber value={stat.value} suffix={stat.suffix} duration={2} />
                </div>
                <div className="text-foreground/80 mb-1 text-lg font-medium">{stat.label}</div>
                <p className="text-muted-foreground text-sm">{stat.description}</p>
              </GlowCard>
            </StaggerItem>
          ))}
        </StaggerContainer>
      </div>
    </section>
  );
}
