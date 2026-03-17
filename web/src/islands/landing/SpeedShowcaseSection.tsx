import { motion, useScroll, useTransform } from "framer-motion";
import { useRef } from "react";
import { Zap, Globe } from "lucide-react";
import { GlowCard } from "@/components/effects/GlowCard";
import {
  AnimatedSection,
  StaggerContainer,
  StaggerItem,
  TextReveal,
} from "@/components/effects/AnimatedSection";
import { SpeedTestGauge } from "@/components/effects/SpeedTestGauge";

const metrics = [
  {
    icon: Zap,
    value: "99.9",
    unit: "%",
    label: "服务可用性",
    description: "稳定可靠，全年无休",
    color: "rgba(6, 182, 212, 0.3)", // cyan-500
  },
  {
    icon: Globe,
    value: "全球",
    unit: "CDN",
    label: "加速网络",
    description: "多节点覆盖，就近访问",
    color: "rgba(14, 165, 233, 0.3)", // sky-500
  },
];

export function SpeedShowcaseSection() {
  const containerRef = useRef<HTMLElement>(null);

  const { scrollYProgress } = useScroll({
    target: containerRef,
    offset: ["start end", "end start"],
  });

  const yBg = useTransform(scrollYProgress, [0, 1], ["-20%", "20%"]);
  const yContent = useTransform(scrollYProgress, [0, 1], ["10%", "-10%"]);

  return (
    <section ref={containerRef} className="relative px-4 py-24">
      {/* Background effects */}
      <motion.div className="pointer-events-none absolute inset-0" style={{ y: yBg }}>
        <div className="absolute top-1/2 left-1/2 h-[500px] w-[500px] -translate-x-1/2 -translate-y-1/2 rounded-full bg-cyan-500/5 blur-3xl" />
      </motion.div>

      <motion.div className="relative mx-auto max-w-6xl" style={{ y: yContent }}>
        {/* Section header */}
        <AnimatedSection className="mb-16 text-center">
          <motion.p
            className="mb-4 text-sm font-medium tracking-wider text-cyan-500 uppercase"
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
          >
            极致性能
          </motion.p>
          <h2 className="text-foreground mb-4 text-4xl font-bold sm:text-5xl">
            <TextReveal text="极致速度，畅享无阻" />
          </h2>
          <p className="text-muted-foreground mx-auto max-w-2xl text-lg">
            基于全球 CDN 加速网络，提供超高速流媒体传输，让您享受丝滑的观影体验
          </p>
        </AnimatedSection>

        {/* Speed gauge */}
        <AnimatedSection className="mb-16 flex justify-center">
          <motion.div
            className="relative"
            initial={{ scale: 0.8, opacity: 0 }}
            whileInView={{ scale: 1, opacity: 1 }}
            viewport={{ once: true }}
            transition={{ duration: 0.8, ease: "easeOut" }}
          >
            <SpeedTestGauge value={800} maxValue={1000} size={320} />
            {/* Decorative rings */}
            <div className="pointer-events-none absolute inset-0 flex items-center justify-center">
              <motion.div
                className="absolute h-[380px] w-[380px] rounded-full border border-cyan-500/20"
                initial={{ scale: 0.8, opacity: 0 }}
                whileInView={{ scale: 1, opacity: 1 }}
                viewport={{ once: true }}
                transition={{ duration: 1, delay: 0.5 }}
              />
              <motion.div
                className="absolute h-[440px] w-[440px] rounded-full border border-cyan-500/10"
                initial={{ scale: 0.8, opacity: 0 }}
                whileInView={{ scale: 1, opacity: 1 }}
                viewport={{ once: true }}
                transition={{ duration: 1, delay: 0.7 }}
              />
            </div>
          </motion.div>
        </AnimatedSection>

        {/* Metrics grid */}
        <StaggerContainer className="mx-auto grid max-w-2xl gap-6 sm:grid-cols-2">
          {metrics.map((metric) => (
            <StaggerItem key={metric.label}>
              <GlowCard className="text-center" glowColor={metric.color} hoverScale={1.03}>
                <div className="mb-4 flex justify-center">
                  <div
                    className="rounded-xl p-3"
                    style={{
                      background: metric.color.replace("0.3", "0.15"),
                    }}
                  >
                    <metric.icon
                      className="h-6 w-6"
                      style={{
                        color: metric.color.replace("0.3", "1"),
                      }}
                    />
                  </div>
                </div>
                <div className="text-foreground mb-2 text-3xl font-bold">
                  {metric.value}
                  <span className="text-muted-foreground ml-1 text-xl">{metric.unit}</span>
                </div>
                <div className="text-foreground/80 mb-1 text-lg font-medium">{metric.label}</div>
                <p className="text-muted-foreground text-sm">{metric.description}</p>
              </GlowCard>
            </StaggerItem>
          ))}
        </StaggerContainer>
      </motion.div>
    </section>
  );
}
