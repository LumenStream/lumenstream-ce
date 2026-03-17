import { motion, useSpring, useTransform, useInView } from "framer-motion";
import { useRef, useEffect } from "react";

interface CountUpNumberProps {
  value: number;
  duration?: number;
  suffix?: string;
  prefix?: string;
  className?: string;
  decimals?: number;
}

export function CountUpNumber({
  value,
  duration = 2,
  suffix = "",
  prefix = "",
  className,
  decimals = 0,
}: CountUpNumberProps) {
  const ref = useRef(null);
  const isInView = useInView(ref, { once: true, margin: "-100px" });

  const spring = useSpring(0, {
    duration: duration * 1000,
    bounce: 0,
  });

  const display = useTransform(spring, (current) => {
    return `${prefix}${current.toFixed(decimals)}${suffix}`;
  });

  useEffect(() => {
    if (isInView) {
      spring.set(value);
    }
  }, [isInView, spring, value]);

  return (
    <motion.span ref={ref} className={className}>
      {display}
    </motion.span>
  );
}
