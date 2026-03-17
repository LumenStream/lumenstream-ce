import { motion, useInView, useMotionValue, useSpring, useTransform } from "framer-motion";
import { useRef, useState } from "react";
import { Database, Play, Users, Server, Check, X } from "lucide-react";
import { AnimatedSection, TextReveal } from "@/components/effects/AnimatedSection";
import { GlowCard } from "@/components/effects/GlowCard";

const comparisonData = [
  { feature: "存储成本", traditional: "高昂服务器费用", ls: "按需付费" },
  { feature: "带宽压力", traditional: "集中式瓶颈", ls: "分布式加速" },
  { feature: "扩展性", traditional: "需要硬件升级", ls: "弹性伸缩" },
  { feature: "维护成本", traditional: "专人运维", ls: "零运维" },
];

export function ArchitectureSection() {
  const ref = useRef<HTMLDivElement>(null);
  const isInView = useInView(ref, { once: true, margin: "-100px" });

  const [isHovered, setIsHovered] = useState(false);

  // 3D Tilt logic
  const x = useMotionValue(0);
  const y = useMotionValue(0);

  const mouseXSpring = useSpring(x, { stiffness: 300, damping: 30 });
  const mouseYSpring = useSpring(y, { stiffness: 300, damping: 30 });

  const rotateX = useTransform(mouseYSpring, [-0.5, 0.5], ["5deg", "-5deg"]);
  const rotateY = useTransform(mouseXSpring, [-0.5, 0.5], ["-5deg", "5deg"]);

  const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!ref.current) return;
    const rect = ref.current.getBoundingClientRect();
    const width = rect.width;
    const height = rect.height;
    const mouseX = e.clientX - rect.left;
    const mouseY = e.clientY - rect.top;

    x.set(mouseX / width - 0.5);
    y.set(mouseY / height - 0.5);
  };

  const handleMouseLeave = () => {
    setIsHovered(false);
    x.set(0);
    y.set(0);
  };

  return (
    <section className="relative px-4 py-24">
      {/* Background */}
      <div className="pointer-events-none absolute inset-0 bg-gradient-to-b from-transparent via-blue-500/5 to-transparent dark:via-blue-950/5" />

      <div className="relative mx-auto max-w-6xl">
        {/* Section header */}
        <AnimatedSection className="mb-16 text-center">
          <motion.p
            className="mb-4 text-sm font-medium tracking-wider text-blue-600 uppercase dark:text-blue-400"
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
          >
            创新架构
          </motion.p>
          <h2 className="text-foreground mb-4 text-4xl font-bold sm:text-5xl">
            <TextReveal text="播放分离，智能调度" />
          </h2>
          <p className="text-muted-foreground mx-auto max-w-2xl text-lg">
            索引与播放分离的创新架构，实现资源的高效利用与极致的播放体验
          </p>
        </AnimatedSection>

        {/* Architecture diagram */}
        <div className="perspective-1000 mb-24">
          <motion.div
            ref={ref}
            onMouseMove={handleMouseMove}
            onMouseEnter={() => setIsHovered(true)}
            onMouseLeave={handleMouseLeave}
            className="mx-auto max-w-4xl"
            style={{
              rotateX,
              rotateY,
              transformStyle: "preserve-3d",
            }}
          >
            <svg
              viewBox="0 0 800 400"
              className="h-full w-full"
              style={{
                height: "auto",
                filter: isHovered ? "drop-shadow(0 0 30px rgba(14, 165, 233, 0.2))" : "none",
                transition: "filter 0.3s ease",
              }}
            >
              <defs>
                <filter id="archGlow" x="-50%" y="-50%" width="200%" height="200%">
                  <feGaussianBlur stdDeviation="4" result="coloredBlur" />
                  <feMerge>
                    <feMergeNode in="coloredBlur" />
                    <feMergeNode in="SourceGraphic" />
                  </feMerge>
                </filter>
                <linearGradient id="lineGradient1" x1="0" y1="0" x2="1" y2="0">
                  <stop offset="0%" stopColor="#0ea5e9" stopOpacity="0.2" />
                  <stop offset="100%" stopColor="#0ea5e9" stopOpacity="1" />
                </linearGradient>
                <linearGradient id="lineGradient2" x1="0" y1="0" x2="1" y2="0">
                  <stop offset="0%" stopColor="#0ea5e9" stopOpacity="1" />
                  <stop offset="100%" stopColor="#3b82f6" stopOpacity="0.5" />
                </linearGradient>
                <linearGradient id="lineGradient3" x1="0" y1="1" x2="1" y2="0">
                  <stop offset="0%" stopColor="#06b6d4" stopOpacity="1" />
                  <stop offset="100%" stopColor="#3b82f6" stopOpacity="0.5" />
                </linearGradient>
                <linearGradient id="lineGradient4" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#0ea5e9" stopOpacity="1" />
                  <stop offset="100%" stopColor="#06b6d4" stopOpacity="0.5" />
                </linearGradient>
              </defs>

              {/* Connection lines */}
              <motion.path
                d="M 165 130 L 295 130"
                stroke="url(#lineGradient1)"
                strokeWidth="2"
                fill="none"
                strokeDasharray="8 4"
                initial={{ pathLength: 0, opacity: 0 }}
                animate={isInView ? { pathLength: 1, opacity: 1 } : {}}
                transition={{ duration: 0.8, delay: 0.5 }}
              />
              <motion.path
                d="M 505 130 L 575 130"
                stroke="url(#lineGradient2)"
                strokeWidth="2"
                fill="none"
                strokeDasharray="8 4"
                initial={{ pathLength: 0, opacity: 0 }}
                animate={isInView ? { pathLength: 1, opacity: 1 } : {}}
                transition={{ duration: 0.8, delay: 1.3 }}
              />
              <motion.path
                d="M 400 170 L 400 225"
                stroke="url(#lineGradient4)"
                strokeWidth="2"
                fill="none"
                strokeDasharray="8 4"
                initial={{ pathLength: 0, opacity: 0 }}
                animate={isInView ? { pathLength: 1, opacity: 1 } : {}}
                transition={{ duration: 0.8, delay: 1.3 }}
              />
              <motion.path
                d="M 445 270 L 670 270 Q 680 270 680 260 L 680 175"
                stroke="url(#lineGradient3)"
                strokeWidth="2"
                fill="none"
                strokeDasharray="8 4"
                initial={{ pathLength: 0, opacity: 0 }}
                animate={isInView ? { pathLength: 1, opacity: 1 } : {}}
                transition={{ duration: 0.8, delay: 2.1 }}
              />

              {/* User node */}
              <motion.g
                initial={{ opacity: 0, scale: 0.5 }}
                animate={isInView ? { opacity: 1, scale: 1 } : {}}
                transition={{ duration: 0.5, delay: 0 }}
              >
                <circle
                  cx="120"
                  cy="130"
                  r="45"
                  className="fill-card"
                  stroke="#0ea5e9"
                  strokeWidth="2"
                />
                <Users
                  x="96"
                  y="106"
                  width="48"
                  height="48"
                  className="text-cyan-500 dark:text-cyan-400"
                />
                <text x="120" y="195" textAnchor="middle" className="fill-muted-foreground text-sm">
                  用户
                </text>
              </motion.g>

              {/* Index service node */}
              <motion.g
                initial={{ opacity: 0, scale: 0.5 }}
                animate={isInView ? { opacity: 1, scale: 1 } : {}}
                transition={{ duration: 0.5, delay: 0.3 }}
              >
                <rect
                  x="300"
                  y="90"
                  width="200"
                  height="80"
                  rx="12"
                  className="fill-card"
                  stroke="#0ea5e9"
                  strokeWidth="2"
                />
                <Database
                  x="320"
                  y="114"
                  width="32"
                  height="32"
                  className="text-cyan-500 dark:text-cyan-400"
                />
                <text
                  x="365"
                  y="126"
                  textAnchor="start"
                  className="fill-foreground text-base font-medium"
                >
                  索引服务
                </text>
                <text x="365" y="146" textAnchor="start" className="fill-muted-foreground text-xs">
                  元数据 · 搜索 · 分类
                </text>
              </motion.g>

              {/* Play service node */}
              <motion.g
                initial={{ opacity: 0, scale: 0.5 }}
                animate={isInView ? { opacity: 1, scale: 1 } : {}}
                transition={{ duration: 0.5, delay: 0.6 }}
              >
                <rect
                  x="580"
                  y="90"
                  width="200"
                  height="80"
                  rx="12"
                  className="fill-card"
                  stroke="#3b82f6"
                  strokeWidth="2"
                />
                <Play
                  x="600"
                  y="114"
                  width="32"
                  height="32"
                  className="text-blue-500 dark:text-blue-400"
                />
                <text
                  x="645"
                  y="126"
                  textAnchor="start"
                  className="fill-foreground text-base font-medium"
                >
                  播放服务
                </text>
                <text x="645" y="146" textAnchor="start" className="fill-muted-foreground text-xs">
                  CDN · 转码 · 流媒体
                </text>
              </motion.g>

              {/* Storage node */}
              <motion.g
                initial={{ opacity: 0, scale: 0.5 }}
                animate={isInView ? { opacity: 1, scale: 1 } : {}}
                transition={{ duration: 0.5, delay: 0.9 }}
              >
                <circle
                  cx="400"
                  cy="270"
                  r="40"
                  className="fill-card"
                  stroke="#06b6d4"
                  strokeWidth="2"
                />
                <Server
                  x="380"
                  y="250"
                  width="40"
                  height="40"
                  className="text-cyan-500 dark:text-cyan-400"
                />
                <text x="400" y="335" textAnchor="middle" className="fill-muted-foreground text-sm">
                  存储集群
                </text>
              </motion.g>

              {/* Arrow indicators */}
              <motion.polygon
                points="300,130 290,125 290,135"
                fill="#0ea5e9"
                initial={{ opacity: 0 }}
                animate={isInView ? { opacity: 1 } : {}}
                transition={{ delay: 1.3 }}
              />
              <motion.polygon
                points="400,230 395,220 405,220"
                fill="#06b6d4"
                initial={{ opacity: 0 }}
                animate={isInView ? { opacity: 1 } : {}}
                transition={{ delay: 2.1 }}
              />
              <motion.polygon
                points="580,130 570,125 570,135"
                fill="#3b82f6"
                initial={{ opacity: 0 }}
                animate={isInView ? { opacity: 1 } : {}}
                transition={{ delay: 2.1 }}
              />
              <motion.polygon
                points="680,170 675,180 685,180"
                fill="#3b82f6"
                initial={{ opacity: 0 }}
                animate={isInView ? { opacity: 1 } : {}}
                transition={{ delay: 2.9 }}
              />
            </svg>
          </motion.div>
        </div>

        {/* Comparison table */}
        <AnimatedSection>
          <div className="grid gap-6 md:grid-cols-2">
            {/* Traditional */}
            <GlowCard glowColor="rgba(100, 116, 139, 0.3)">
              <div className="mb-4 flex items-center gap-3">
                <div className="bg-muted rounded-lg p-2">
                  <Server className="text-muted-foreground h-5 w-5" />
                </div>
                <h3 className="text-muted-foreground text-lg font-semibold">传统方案</h3>
              </div>
              <ul className="space-y-3">
                {comparisonData.map((item) => (
                  <li key={item.feature} className="flex items-start gap-3">
                    <X className="text-muted-foreground/50 mt-0.5 h-4 w-4 shrink-0" />
                    <div>
                      <span className="text-muted-foreground text-sm">{item.feature}：</span>
                      <span className="text-muted-foreground/70 text-sm">{item.traditional}</span>
                    </div>
                  </li>
                ))}
              </ul>
            </GlowCard>

            {/* LumenStream */}
            <GlowCard glowColor="rgba(6, 182, 212, 0.3)">
              <div className="mb-4 flex items-center gap-3">
                <div className="rounded-lg bg-cyan-500/20 p-2">
                  <Play className="h-5 w-5 text-cyan-600 dark:text-cyan-400" />
                </div>
                <h3 className="text-foreground text-lg font-semibold">LumenStream 方案</h3>
              </div>
              <ul className="space-y-3">
                {comparisonData.map((item) => (
                  <li key={item.feature} className="flex items-start gap-3">
                    <Check className="mt-0.5 h-4 w-4 shrink-0 text-cyan-500 dark:text-cyan-400" />
                    <div>
                      <span className="text-muted-foreground text-sm">{item.feature}：</span>
                      <span className="text-foreground text-sm">{item.ls}</span>
                    </div>
                  </li>
                ))}
              </ul>
            </GlowCard>
          </div>
        </AnimatedSection>
      </div>
    </section>
  );
}
