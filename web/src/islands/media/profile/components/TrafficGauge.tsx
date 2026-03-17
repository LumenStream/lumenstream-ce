import { motion, useInView } from "framer-motion";
import { useId, useRef } from "react";
import { cn } from "@/lib/utils";

interface TrafficGaugeProps {
  usedBytes: number;
  quotaBytes: number | null;
  className?: string;
}

function toGaugePoint(cx: number, cy: number, r: number, angleDeg: number) {
  const rad = (angleDeg * Math.PI) / 180;
  return { x: cx + r * Math.cos(rad), y: cy - r * Math.sin(rad) };
}

function createTopSemiArc(cx: number, cy: number, r: number) {
  return `M ${cx - r} ${cy} A ${r} ${r} 0 0 1 ${cx + r} ${cy}`;
}

export function TrafficGauge({ usedBytes, quotaBytes, className }: TrafficGaugeProps) {
  const ref = useRef(null);
  const gradientId = useId();
  const isInView = useInView(ref, { once: true, margin: "-50px" });

  const unlimited = quotaBytes === null || quotaBytes <= 0;
  const percentage = unlimited ? 0 : Math.min(Math.max(usedBytes / quotaBytes, 0), 1);
  const displayPercent = unlimited
    ? null
    : Math.min(100, Math.max(0, Number((percentage * 100).toFixed(1))));

  const width = 320;
  const height = 196;
  const strokeWidth = 14;
  const cx = width / 2;
  const cy = 166;
  const radius = 116;

  const trackPath = createTopSemiArc(cx, cy, radius);
  const needleAngle = 180 - percentage * 180;
  const needleTip = toGaugePoint(cx, cy, radius, needleAngle);
  const needleTail = toGaugePoint(cx, cy, 26, needleAngle + 180);

  const stopColor1 = percentage >= 0.9 ? "#ef4444" : percentage >= 0.7 ? "#f59e0b" : "#10b981";
  const stopColor2 = percentage >= 0.9 ? "#dc2626" : percentage >= 0.7 ? "#d97706" : "#059669";

  return (
    <div ref={ref} className={cn("flex w-full max-w-[360px] flex-col items-center", className)}>
      <svg
        width={width}
        height={height}
        viewBox={`0 0 ${width} ${height}`}
        className="w-full overflow-visible"
      >
        <defs>
          <linearGradient id={gradientId} x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stopColor={stopColor1} />
            <stop offset="100%" stopColor={stopColor2} />
          </linearGradient>
        </defs>

        {Array.from({ length: 5 }, (_, i) => i).map((tick) => {
          const ratio = tick / 4;
          const angle = 180 - ratio * 180;
          const outer = toGaugePoint(cx, cy, radius + 8, angle);
          const inner = toGaugePoint(cx, cy, radius - 6, angle);
          return (
            <line
              key={tick}
              x1={outer.x}
              y1={outer.y}
              x2={inner.x}
              y2={inner.y}
              stroke="currentColor"
              strokeWidth="2"
              className="light:text-black/[0.18] text-white/[0.15]"
            />
          );
        })}

        <path
          d={trackPath}
          fill="none"
          stroke="currentColor"
          strokeWidth={strokeWidth}
          strokeLinecap="round"
          className="light:text-black/[0.08] text-white/[0.07]"
        />

        <path
          d={trackPath}
          fill="none"
          stroke="currentColor"
          strokeWidth={strokeWidth - 6}
          strokeLinecap="round"
          className="light:text-black/[0.05] text-white/[0.03]"
        />

        {!unlimited && (
          <motion.path
            d={trackPath}
            fill="none"
            stroke={`url(#${gradientId})`}
            strokeWidth={strokeWidth}
            strokeLinecap="round"
            initial={{ pathLength: 0 }}
            animate={isInView ? { pathLength: percentage } : { pathLength: 0 }}
            transition={{ duration: 1.2, ease: "easeOut", delay: 0.15 }}
          />
        )}

        {!unlimited && (
          <>
            <motion.line
              x1={needleTail.x}
              y1={needleTail.y}
              x2={needleTip.x}
              y2={needleTip.y}
              stroke="currentColor"
              strokeWidth="3"
              strokeLinecap="round"
              className={cn(
                percentage >= 0.9
                  ? "text-red-500"
                  : percentage >= 0.7
                    ? "text-amber-500"
                    : "text-emerald-500"
              )}
              initial={{ opacity: 0, scale: 0.92 }}
              animate={isInView ? { opacity: 1, scale: 1 } : { opacity: 0, scale: 0.92 }}
              transition={{ duration: 0.35, delay: 0.9 }}
            />
            <motion.circle
              cx={needleTip.x}
              cy={needleTip.y}
              r="6"
              fill={`url(#${gradientId})`}
              initial={{ opacity: 0, scale: 0.5 }}
              animate={isInView ? { opacity: 1, scale: 1 } : { opacity: 0, scale: 0.5 }}
              transition={{ duration: 0.35, delay: 1.05 }}
            />
          </>
        )}

        <circle
          cx={cx}
          cy={cy}
          r="7"
          className="fill-foreground/90"
          stroke="currentColor"
          strokeWidth="2"
        />

        <motion.text
          x={cx}
          y={cy - 38}
          textAnchor="middle"
          className="fill-foreground text-[34px] font-semibold tracking-tight"
          initial={{ opacity: 0 }}
          animate={isInView ? { opacity: 1 } : { opacity: 0 }}
          transition={{ duration: 0.35, delay: 0.7 }}
        >
          {unlimited ? "∞" : `${displayPercent}%`}
        </motion.text>

        <motion.text
          x={cx}
          y={cy - 14}
          textAnchor="middle"
          className="fill-muted-foreground text-xs"
          initial={{ opacity: 0 }}
          animate={isInView ? { opacity: 1 } : { opacity: 0 }}
          transition={{ duration: 0.35, delay: 0.8 }}
        >
          {unlimited ? "不限额" : "已使用"}
        </motion.text>
      </svg>
    </div>
  );
}
