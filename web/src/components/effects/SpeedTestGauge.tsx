import { motion, useInView } from "framer-motion";
import { useEffect, useId, useRef, useState } from "react";

interface SpeedTestGaugeProps {
  value: number;
  maxValue?: number;
  size?: number;
  className?: string;
}

const MIN_GAUGE_ANGLE = -135;
const MAX_GAUGE_ANGLE = 135;
const GAUGE_SWEEP_ANGLE = MAX_GAUGE_ANGLE - MIN_GAUGE_ANGLE;

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}

function toSafeMaxValue(maxValue: number) {
  return Number.isFinite(maxValue) && maxValue > 0 ? maxValue : 1000;
}

export function valueToGaugeAngle(value: number, maxValue: number) {
  const safeMaxValue = toSafeMaxValue(maxValue);
  const clampedValue = clamp(value, 0, safeMaxValue);
  return MIN_GAUGE_ANGLE + (clampedValue / safeMaxValue) * GAUGE_SWEEP_ANGLE;
}

export function buildJitterValue(targetValue: number, maxValue: number, noise: number) {
  const safeMaxValue = toSafeMaxValue(maxValue);
  const clampedTarget = clamp(targetValue, 0, safeMaxValue);
  const jitterRange = Math.max(safeMaxValue * 0.01, clampedTarget * 0.03);
  return clamp(clampedTarget + noise * jitterRange, 0, safeMaxValue);
}

function polarToCartesian(
  centerX: number,
  centerY: number,
  radius: number,
  angleInDegrees: number
) {
  const angleInRadians = ((angleInDegrees - 90) * Math.PI) / 180;
  return {
    x: centerX + radius * Math.cos(angleInRadians),
    y: centerY + radius * Math.sin(angleInRadians),
  };
}

export function createGaugeArcPath(
  centerX: number,
  centerY: number,
  radius: number,
  startAngle: number,
  endAngle: number
) {
  const start = polarToCartesian(centerX, centerY, radius, startAngle);
  const end = polarToCartesian(centerX, centerY, radius, endAngle);
  const largeArcFlag = endAngle - startAngle <= 180 ? "0" : "1";
  return ["M", start.x, start.y, "A", radius, radius, 0, largeArcFlag, 1, end.x, end.y].join(" ");
}

export function SpeedTestGauge({
  value: targetValue,
  maxValue = 1000,
  size = 240,
  className = "",
}: SpeedTestGaugeProps) {
  const ref = useRef<HTMLDivElement>(null);
  const isInView = useInView(ref, { once: true, margin: "-50px" });
  const uniqueId = useId().replace(/:/g, "");

  const safeMaxValue = toSafeMaxValue(maxValue);
  const [currentValue, setCurrentValue] = useState(0);

  const percentage = clamp(currentValue / safeMaxValue, 0, 1);
  const currentAngle = MIN_GAUGE_ANGLE + percentage * GAUGE_SWEEP_ANGLE;

  const speedGlowId = `${uniqueId}-speedGlow`;
  const speedGradientId = `${uniqueId}-speedGradient`;

  const cx = size / 2;
  const cy = size / 2;
  const radius = size * 0.42;
  const strokeWidth = size * 0.03;
  const trackPath = createGaugeArcPath(cx, cy, radius, MIN_GAUGE_ANGLE, MAX_GAUGE_ANGLE);

  // Position for the glowing dot at the end of the arc
  const dotPos = polarToCartesian(cx, cy, radius, currentAngle);

  useEffect(() => {
    if (!isInView) return;

    let cancelled = false;
    let frameId: number | undefined;
    let jitterTimer: number | undefined;

    const rampDurationMs = 1000;
    const rampStartTime = performance.now();
    setCurrentValue(0);

    const animateRamp = (now: number) => {
      if (cancelled) return;

      const progress = clamp((now - rampStartTime) / rampDurationMs, 0, 1);
      const eased = 1 - (1 - progress) ** 3;
      setCurrentValue(targetValue * eased);

      if (progress < 1) {
        frameId = window.requestAnimationFrame(animateRamp);
        return;
      }

      jitterTimer = window.setInterval(() => {
        if (cancelled) return;
        const noise = Math.random() * 2 - 1;
        setCurrentValue(buildJitterValue(targetValue, safeMaxValue, noise));
      }, 180);
    };

    frameId = window.requestAnimationFrame(animateRamp);

    return () => {
      cancelled = true;
      if (frameId !== undefined) {
        window.cancelAnimationFrame(frameId);
      }
      if (jitterTimer !== undefined) {
        window.clearInterval(jitterTimer);
      }
    };
  }, [isInView, targetValue, safeMaxValue]);

  return (
    <div ref={ref} className={className}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
        <defs>
          <filter id={speedGlowId} x="-50%" y="-50%" width="200%" height="200%">
            <feGaussianBlur stdDeviation="6" result="coloredBlur" />
            <feMerge>
              <feMergeNode in="coloredBlur" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>
          <linearGradient id={speedGradientId} x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stopColor="#0ea5e9" />
            <stop offset="50%" stopColor="#3b82f6" />
            <stop offset="100%" stopColor="#8b5cf6" />
          </linearGradient>
        </defs>

        {/* Outer dashed tech ring */}
        <circle
          cx={cx}
          cy={cy}
          r={radius + strokeWidth * 2}
          fill="none"
          stroke="currentColor"
          strokeWidth="1"
          strokeDasharray="4 8"
          className="text-cyan-500/20"
        />

        {/* Inner track background */}
        <path
          d={trackPath}
          fill="none"
          stroke="currentColor"
          strokeWidth={strokeWidth}
          strokeLinecap="round"
          className="text-slate-800/50"
        />

        {/* Active glowing track */}
        <motion.path
          d={trackPath}
          fill="none"
          stroke={`url(#${speedGradientId})`}
          strokeWidth={strokeWidth}
          strokeLinecap="round"
          filter={`url(#${speedGlowId})`}
          initial={{ pathLength: 0 }}
          animate={{ pathLength: percentage }}
          transition={{ duration: 0.18, ease: "easeOut" }}
        />

        {/* Moving dot at the end of the arc */}
        <motion.circle
          cx={cx}
          cy={cy}
          r={strokeWidth * 1.2}
          fill="#3b82f6"
          filter={`url(#${speedGlowId})`}
          animate={{ cx: dotPos.x, cy: dotPos.y }}
          transition={{ duration: 0.18, ease: "easeOut" }}
        />
        <motion.circle
          cx={cx}
          cy={cy}
          r={strokeWidth * 0.6}
          fill="#fff"
          animate={{ cx: dotPos.x, cy: dotPos.y }}
          transition={{ duration: 0.18, ease: "easeOut" }}
        />

        {/* Center rotating decorative rings */}
        <motion.circle
          cx={cx}
          cy={cy}
          r={radius * 0.6}
          fill="none"
          stroke="currentColor"
          strokeWidth="1"
          strokeDasharray="1 4"
          className="text-cyan-500/30"
          animate={{ rotate: 360 }}
          transition={{ duration: 20, repeat: Infinity, ease: "linear" }}
          style={{ originX: "50%", originY: "50%" }}
        />

        <motion.circle
          cx={cx}
          cy={cy}
          r={radius * 0.5}
          fill="none"
          stroke="currentColor"
          strokeWidth="1"
          strokeDasharray="10 10"
          className="text-blue-500/20"
          animate={{ rotate: -360 }}
          transition={{ duration: 15, repeat: Infinity, ease: "linear" }}
          style={{ originX: "50%", originY: "50%" }}
        />

        {/* Center data visualization */}
        <motion.g
          initial={{ opacity: 0, scale: 0.8 }}
          animate={{ opacity: isInView ? 1 : 0, scale: isInView ? 1 : 0.8 }}
          transition={{ duration: 0.5, delay: 0.2 }}
        >
          <text
            x={cx}
            y={cy}
            textAnchor="middle"
            dominantBaseline="middle"
            className="fill-foreground text-5xl font-bold tracking-tighter"
            style={{ textShadow: "0 0 20px rgba(6,182,212,0.5)" }}
          >
            {Math.round(currentValue)}
          </text>
          <text
            x={cx}
            y={cy + 35}
            textAnchor="middle"
            dominantBaseline="middle"
            className="fill-cyan-400 text-sm font-medium tracking-widest"
          >
            Mbps
          </text>
        </motion.g>

        {/* Pulsing energy rings when active */}
        {isInView && (
          <>
            {[0, 1].map((i) => (
              <motion.circle
                key={i}
                cx={cx}
                cy={cy}
                r={radius * 0.3}
                fill="none"
                stroke="currentColor"
                strokeWidth="1"
                className="text-cyan-500"
                initial={{ scale: 0, opacity: 0.8 }}
                animate={{
                  scale: [0, 2.5],
                  opacity: [0.5, 0],
                }}
                transition={{
                  duration: 2,
                  delay: i * 1,
                  repeat: Infinity,
                  ease: "easeOut",
                }}
              />
            ))}
          </>
        )}
      </svg>
    </div>
  );
}
