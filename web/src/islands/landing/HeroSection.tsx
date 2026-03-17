import { motion, useScroll, useTransform } from "framer-motion";
import { useState, useEffect, useRef } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { TextReveal } from "@/components/effects/AnimatedSection";
import { getAuthSession } from "@/lib/auth/token";
import { isMockFeatureEnabled, isMockMode } from "@/lib/mock/mode";
import { enableMockExperience } from "@/lib/mock/session";
import { Play, Zap, Shield, Globe } from "lucide-react";
import { ThemeToggle } from "@/islands/navigation/ThemeToggle";

export function HeroSection() {
  const mockFeatureEnabled = isMockFeatureEnabled();
  const [hasSession, setHasSession] = useState<boolean | null>(null);
  const [mockEnabled, setMockEnabled] = useState<boolean | null>(null);
  const [mousePosition, setMousePosition] = useState({ x: 0, y: 0 });
  const containerRef = useRef<HTMLElement>(null);

  const { scrollYProgress } = useScroll({
    target: containerRef,
    offset: ["start start", "end start"],
  });

  const yBackground = useTransform(scrollYProgress, [0, 1], ["0%", "50%"]);
  const yContent = useTransform(scrollYProgress, [0, 1], ["0%", "20%"]);
  const opacity = useTransform(scrollYProgress, [0, 0.8], [1, 0]);

  useEffect(() => {
    setHasSession(Boolean(getAuthSession()));
    setMockEnabled(mockFeatureEnabled && isMockMode());
  }, [mockFeatureEnabled]);

  const handleMouseMove = (e: React.MouseEvent) => {
    const rect = e.currentTarget.getBoundingClientRect();
    setMousePosition({
      x: (e.clientX - rect.left - rect.width / 2) / 20,
      y: (e.clientY - rect.top - rect.height / 2) / 20,
    });
  };

  async function startDemo() {
    await enableMockExperience("demo-admin");
    window.location.href = "/app/home";
  }

  const features = [
    { icon: Zap, text: "800Mbps 极速" },
    { icon: Shield, text: "稳定可靠" },
    { icon: Globe, text: "全球加速" },
    { icon: Play, text: "海量资源" },
  ];

  return (
    <section
      ref={containerRef}
      className="relative flex min-h-[80vh] flex-col items-center justify-center px-4 py-20 text-center"
      onMouseMove={handleMouseMove}
      onMouseLeave={() => setMousePosition({ x: 0, y: 0 })}
    >
      {/* Theme toggle - fixed position */}
      <div className="absolute top-6 right-6 z-20">
        <ThemeToggle />
      </div>

      {/* Floating orbs */}
      <motion.div
        className="pointer-events-none absolute inset-0 overflow-hidden"
        style={{ y: yBackground, opacity }}
      >
        <motion.div
          className="absolute top-20 -left-32 h-64 w-64 rounded-full bg-cyan-500/10 blur-3xl dark:bg-cyan-600/10"
          animate={{
            x: mousePosition.x * 2,
            y: mousePosition.y * 2,
          }}
          transition={{ type: "spring", stiffness: 50, damping: 30 }}
        />
        <motion.div
          className="absolute -right-32 bottom-20 h-96 w-96 rounded-full bg-blue-500/10 blur-3xl dark:bg-blue-600/10"
          animate={{
            x: mousePosition.x * -2,
            y: mousePosition.y * -2,
          }}
          transition={{ type: "spring", stiffness: 50, damping: 30 }}
        />
        <motion.div
          className="absolute top-1/3 left-1/2 h-48 w-48 -translate-x-1/2 rounded-full bg-slate-500/5 blur-3xl dark:bg-slate-600/5"
          animate={{
            x: mousePosition.x * 1.5,
            y: mousePosition.y * 1.5,
          }}
          transition={{ type: "spring", stiffness: 50, damping: 30 }}
        />
      </motion.div>

      <motion.div
        style={{ y: yContent, opacity }}
        className="relative z-10 flex flex-col items-center"
      >
        {/* Badge */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
        >
          <Badge
            variant="outline"
            className="mb-6 border-cyan-500/30 bg-cyan-500/10 px-4 py-1.5 text-xs font-medium text-cyan-600 backdrop-blur-sm dark:text-cyan-400"
          >
            <span className="mr-2 inline-block h-1.5 w-1.5 animate-pulse rounded-full bg-cyan-500" />
            LumenStream Media Server v2.0
          </Badge>
        </motion.div>

        {/* Main heading */}
        <motion.h1
          className="mb-6 max-w-4xl text-5xl font-bold tracking-tight sm:text-6xl lg:text-7xl"
          initial={{ opacity: 0, y: 30 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6, delay: 0.1 }}
        >
          <span className="bg-gradient-to-r from-slate-200 via-cyan-400 to-blue-500 bg-clip-text text-transparent drop-shadow-[0_0_15px_rgba(34,211,238,0.3)] dark:from-white dark:via-cyan-300 dark:to-blue-400">
            海量资源
          </span>
          <br />
          <span className="text-foreground drop-shadow-sm">
            <TextReveal text="极速体验" delay={0.3} />
          </span>
        </motion.h1>

        {/* Subtitle */}
        <motion.p
          className="text-muted-foreground mb-8 max-w-2xl text-lg sm:text-xl"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.4 }}
        >
          10000+ 部高清影视资源，800Mbps 极速播放，按量计费超高性价比， 让您畅享无忧的观影体验。
        </motion.p>

        {/* Feature pills */}
        <motion.div
          className="mb-10 flex flex-wrap justify-center gap-3"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.5 }}
        >
          {features.map((feature, index) => (
            <motion.div
              key={feature.text}
              className="border-border/50 bg-card/30 text-foreground/80 flex items-center gap-2 rounded-full border px-4 py-2 text-sm backdrop-blur-md"
              initial={{ opacity: 0, scale: 0.8 }}
              animate={{ opacity: 1, scale: 1 }}
              transition={{ duration: 0.3, delay: 0.5 + index * 0.1 }}
              whileHover={{
                scale: 1.05,
                borderColor: "rgba(34, 211, 238, 0.5)",
                backgroundColor: "rgba(34, 211, 238, 0.05)",
              }}
            >
              <feature.icon className="h-4 w-4 text-cyan-500" />
              {feature.text}
            </motion.div>
          ))}
        </motion.div>

        {/* CTA Buttons */}
        {hasSession !== null && (
          <motion.div
            className="flex flex-wrap justify-center gap-4"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.7 }}
          >
            <motion.div whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }}>
              <Button
                size="lg"
                className="relative overflow-hidden border border-cyan-500/50 bg-cyan-950/50 px-8 text-cyan-50 shadow-[0_0_20px_rgba(6,182,212,0.2)] backdrop-blur-md transition-all duration-300 hover:border-cyan-400 hover:bg-cyan-900/60 hover:shadow-[0_0_25px_rgba(6,182,212,0.4)]"
                onClick={() => (window.location.href = hasSession ? "/app/home" : "/login")}
              >
                <div className="absolute inset-0 translate-x-[-100%] animate-[shimmer_2s_infinite] bg-gradient-to-r from-transparent via-cyan-400/10 to-transparent" />
                {hasSession ? "继续使用" : "立即开始"}
              </Button>
            </motion.div>

            {mockFeatureEnabled ? (
              <motion.div whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }}>
                <Button
                  variant="outline"
                  size="lg"
                  className="border-slate-700/50 bg-slate-900/40 px-8 text-slate-300 backdrop-blur-md hover:border-cyan-900/50 hover:bg-slate-800/60 hover:text-cyan-400"
                  onClick={() => void startDemo()}
                >
                  <Zap className="mr-2 h-4 w-4 text-cyan-500" />
                  Mock 演示
                </Button>
              </motion.div>
            ) : null}
          </motion.div>
        )}

        {/* Status indicators */}
        <motion.div
          className="mt-8 flex flex-wrap justify-center gap-3"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.5, delay: 0.9 }}
        >
          {hasSession && (
            <Badge
              variant="outline"
              className="border-green-500/30 bg-green-500/10 text-green-600 dark:text-green-400"
            >
              已登录
            </Badge>
          )}
          {mockFeatureEnabled && mockEnabled && (
            <Badge
              variant="outline"
              className="border-amber-500/30 bg-amber-500/10 text-amber-600 dark:text-amber-400"
            >
              Mock 模式
            </Badge>
          )}
        </motion.div>
      </motion.div>
    </section>
  );
}
