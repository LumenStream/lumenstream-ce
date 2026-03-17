import { motion, useInView } from "framer-motion";
import { useRef } from "react";

interface SpeedGaugeProps {
  value: number;
  maxValue?: number;
  size?: number;
  className?: string;
}

export function SpeedGauge({ value, maxValue = 1000, size = 280, className }: SpeedGaugeProps) {
  const ref = useRef(null);
  const isInView = useInView(ref, { once: true, margin: "-50px" });

  const percentage = Math.min(value / maxValue, 1);
  const angle = -135 + percentage * 270; // -135 to 135 degrees

  const cx = size / 2;
  const cy = size / 2;
  const radius = size * 0.38;
  const strokeWidth = size * 0.035;

  // Arc path for the gauge track
  const createArc = (startAngle: number, endAngle: number) => {
    const start = polarToCartesian(cx, cy, radius, endAngle);
    const end = polarToCartesian(cx, cy, radius, startAngle);
    const largeArcFlag = endAngle - startAngle <= 180 ? "0" : "1";
    return `M ${start.x} ${start.y} A ${radius} ${radius} 0 ${largeArcFlag} 0 ${end.x} ${end.y}`;
  };

  const polarToCartesian = (
    centerX: number,
    centerY: number,
    r: number,
    angleInDegrees: number
  ) => {
    const angleInRadians = ((angleInDegrees - 90) * Math.PI) / 180;
    return {
      x: centerX + r * Math.cos(angleInRadians),
      y: centerY + r * Math.sin(angleInRadians),
    };
  };

  const trackPath = createArc(-135, 135);

  // Tick marks
  const ticks = [];
  for (let i = 0; i <= 10; i++) {
    const tickAngle = -135 + i * 27;
    const innerRadius = radius - strokeWidth * 2;
    const outerRadius = radius + strokeWidth;
    const inner = polarToCartesian(cx, cy, innerRadius, tickAngle);
    const outer = polarToCartesian(cx, cy, outerRadius, tickAngle);
    ticks.push(
      <line
        key={i}
        x1={inner.x}
        y1={inner.y}
        x2={outer.x}
        y2={outer.y}
        stroke="currentColor"
        strokeWidth={i % 5 === 0 ? 2 : 1}
        className={i % 5 === 0 ? "text-slate-400" : "text-slate-600"}
      />
    );
  }

  // Needle
  const needleLength = radius - strokeWidth * 3;
  const needleEnd = polarToCartesian(cx, cy, needleLength, angle);

  return (
    <div ref={ref} className={className}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
        {/* Glow filter */}
        <defs>
          <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
            <feGaussianBlur stdDeviation="3" result="coloredBlur" />
            <feMerge>
              <feMergeNode in="coloredBlur" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>
          <linearGradient id="gaugeGradient" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stopColor="#3b82f6" />
            <stop offset="50%" stopColor="#8b5cf6" />
            <stop offset="100%" stopColor="#f43f5e" />
          </linearGradient>
        </defs>

        {/* Background track */}
        <path
          d={trackPath}
          fill="none"
          stroke="currentColor"
          strokeWidth={strokeWidth}
          strokeLinecap="round"
          className="text-slate-800"
        />

        {/* Animated progress arc */}
        <motion.path
          d={trackPath}
          fill="none"
          stroke="url(#gaugeGradient)"
          strokeWidth={strokeWidth}
          strokeLinecap="round"
          filter="url(#glow)"
          initial={{ pathLength: 0 }}
          animate={isInView ? { pathLength: percentage } : { pathLength: 0 }}
          transition={{ duration: 2, ease: "easeOut", delay: 0.3 }}
        />

        {/* Tick marks */}
        {ticks}

        {/* Center circle */}
        <circle cx={cx} cy={cy} r={strokeWidth * 2} className="fill-slate-700" />

        {/* Needle */}
        <motion.line
          x1={cx}
          y1={cy}
          x2={needleEnd.x}
          y2={needleEnd.y}
          stroke="currentColor"
          strokeWidth={3}
          strokeLinecap="round"
          className="text-rose-500"
          filter="url(#glow)"
          initial={{ rotate: -135, originX: cx, originY: cy }}
          animate={
            isInView
              ? { rotate: angle, originX: cx, originY: cy }
              : { rotate: -135, originX: cx, originY: cy }
          }
          transition={{ duration: 2, ease: "easeOut", delay: 0.3 }}
          style={{ transformOrigin: `${cx}px ${cy}px` }}
        />

        {/* Center dot */}
        <circle cx={cx} cy={cy} r={strokeWidth} className="fill-rose-500" />

        {/* Value display */}
        <motion.text
          x={cx}
          y={cy + radius * 0.5}
          textAnchor="middle"
          className="fill-white text-3xl font-bold"
          initial={{ opacity: 0 }}
          animate={isInView ? { opacity: 1 } : { opacity: 0 }}
          transition={{ duration: 0.5, delay: 1.5 }}
        >
          {value}+
        </motion.text>
        <motion.text
          x={cx}
          y={cy + radius * 0.5 + 24}
          textAnchor="middle"
          className="fill-slate-400 text-sm"
          initial={{ opacity: 0 }}
          animate={isInView ? { opacity: 1 } : { opacity: 0 }}
          transition={{ duration: 0.5, delay: 1.7 }}
        >
          Mbps
        </motion.text>
      </svg>
    </div>
  );
}
