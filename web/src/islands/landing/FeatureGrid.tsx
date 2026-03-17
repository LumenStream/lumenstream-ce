import { motion } from "framer-motion";
import { GlowCard } from "@/components/effects/GlowCard";
import {
  AnimatedSection,
  StaggerContainer,
  StaggerItem,
} from "@/components/effects/AnimatedSection";
import { Cloud, HardDrive, Monitor, Shield, Smartphone, Zap, Users, Video } from "lucide-react";

const features = [
  {
    icon: Cloud,
    title: "云端存储集成",
    description: "无缝连接 Google Drive，自动同步媒体库，支持多账号轮询与智能负载均衡。",
    color: "rgba(14, 165, 233, 0.3)", // sky-500
  },
  {
    icon: Zap,
    title: "智能缓存加速",
    description: "基于 S3 的多层缓存架构，热点内容自动预加载，实现秒级播放启动。",
    color: "rgba(6, 182, 212, 0.3)", // cyan-500
  },
  {
    icon: Monitor,
    title: "多终端适配",
    description: "支持 Infuse、VLC、PotPlayer 等主流播放器，兼容 Web、移动端与电视端。",
    color: "rgba(59, 130, 246, 0.3)", // blue-500
  },
  {
    icon: Shield,
    title: "企业级安全",
    description: "Token 级权限控制，会话审计日志，支持 Admin 单一管理角色。",
    color: "rgba(16, 185, 129, 0.3)", // emerald-500
  },
  {
    icon: Video,
    title: "全格式支持",
    description: "支持 4K HDR、Dolby Vision，无需转码直接播放，保留原始画质。",
    color: "rgba(14, 165, 233, 0.3)", // sky-500
  },
  {
    icon: Smartphone,
    title: "移动端优化",
    description: "响应式界面设计，触屏手势操作，支持 iOS/Android 原生播放器唤起。",
    color: "rgba(6, 182, 212, 0.3)", // cyan-500
  },
  {
    icon: Users,
    title: "多用户管理",
    description: "独立用户空间，个性化推荐，家庭成员账号隔离与内容分级。",
    color: "rgba(59, 130, 246, 0.3)", // blue-500
  },
  {
    icon: HardDrive,
    title: "弹性扩容",
    description: "按需付费的流量套餐，支持月付/年付，随时升级不中断服务。",
    color: "rgba(148, 163, 184, 0.3)", // slate-400
  },
];

export function FeatureGrid() {
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
          核心功能
        </motion.span>
        <h2 className="text-foreground mb-4 text-3xl font-bold sm:text-4xl">
          为什么选择 LumenStream
        </h2>
        <p className="text-muted-foreground mx-auto max-w-2xl">
          从存储到播放，从安全到体验，我们为您提供完整的流媒体解决方案。
        </p>
      </AnimatedSection>

      {/* Feature grid */}
      <StaggerContainer
        className="mx-auto grid max-w-6xl gap-6 sm:grid-cols-2 lg:grid-cols-4"
        staggerDelay={0.1}
      >
        {features.map((feature) => (
          <StaggerItem key={feature.title}>
            <GlowCard glowColor={feature.color} className="h-full">
              <div className="mb-4 inline-flex rounded-lg border border-slate-700/50 bg-gradient-to-br from-slate-800 to-slate-900 p-3">
                <feature.icon
                  className="h-6 w-6"
                  style={{
                    color: feature.color.replace("0.3", "0.9"),
                  }}
                />
              </div>
              <h3 className="text-foreground mb-2 text-lg font-semibold">{feature.title}</h3>
              <p className="text-muted-foreground text-sm leading-relaxed">{feature.description}</p>
            </GlowCard>
          </StaggerItem>
        ))}
      </StaggerContainer>
    </section>
  );
}
