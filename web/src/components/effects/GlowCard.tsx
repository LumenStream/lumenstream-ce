import { motion, useMotionValue, useSpring, useTransform } from "framer-motion";
import { useState } from "react";
import { cn } from "@/lib/utils";

interface GlowCardProps {
  children: React.ReactNode;
  className?: string;
  glowColor?: string;
  hoverScale?: number;
  tilt?: boolean;
}

export function GlowCard({
  children,
  className,
  glowColor = "rgba(6, 182, 212, 0.3)", // cyan-500
  hoverScale = 1.02,
  tilt = true,
}: GlowCardProps) {
  const [mousePosition, setMousePosition] = useState({ x: 0, y: 0 });
  const [isHovered, setIsHovered] = useState(false);

  // 3D Tilt logic
  const x = useMotionValue(0);
  const y = useMotionValue(0);

  const mouseXSpring = useSpring(x, { stiffness: 300, damping: 30 });
  const mouseYSpring = useSpring(y, { stiffness: 300, damping: 30 });

  const rotateX = useTransform(mouseYSpring, [-0.5, 0.5], ["5deg", "-5deg"]);
  const rotateY = useTransform(mouseXSpring, [-0.5, 0.5], ["-5deg", "5deg"]);

  const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const width = rect.width;
    const height = rect.height;
    const mouseX = e.clientX - rect.left;
    const mouseY = e.clientY - rect.top;

    setMousePosition({ x: mouseX, y: mouseY });

    if (tilt) {
      x.set(mouseX / width - 0.5);
      y.set(mouseY / height - 0.5);
    }
  };

  const handleMouseLeave = () => {
    setIsHovered(false);
    if (tilt) {
      x.set(0);
      y.set(0);
    }
  };

  return (
    <div className="h-full w-full" style={{ perspective: tilt ? 1200 : "none" }}>
      <motion.div
        className={cn(
          "border-border/50 bg-card/30 relative h-full overflow-hidden rounded-xl border p-6 backdrop-blur-md",
          className
        )}
        onMouseMove={handleMouseMove}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={handleMouseLeave}
        whileHover={{ scale: hoverScale }}
        transition={{ type: "spring", stiffness: 400, damping: 30 }}
        style={{
          rotateX: tilt ? rotateX : 0,
          rotateY: tilt ? rotateY : 0,
          transformStyle: tilt ? "preserve-3d" : "flat",
          boxShadow: isHovered
            ? `0 0 30px ${glowColor}, 0 0 60px ${glowColor.replace("0.3", "0.1")}`
            : "0 4px 20px rgba(0, 0, 0, 0.2)",
        }}
      >
        {/* Glow effect overlay */}
        <div
          className="pointer-events-none absolute inset-0 opacity-0 transition-opacity duration-300"
          style={{
            opacity: isHovered ? 0.15 : 0,
            background: `radial-gradient(300px circle at ${mousePosition.x}px ${mousePosition.y}px, ${glowColor}, transparent 50%)`,
          }}
        />

        {/* Border glow */}
        <div
          className="pointer-events-none absolute inset-0 rounded-xl transition-opacity duration-300"
          style={{
            opacity: isHovered ? 1 : 0,
            boxShadow: `inset 0 0 20px ${glowColor.replace("0.3", "0.2")}`,
          }}
        />

        <div
          className="relative z-10 h-full"
          style={{
            transform: tilt && isHovered ? "translateZ(30px)" : "translateZ(0px)",
            transition: "transform 0.3s ease",
          }}
        >
          {children}
        </div>
      </motion.div>
    </div>
  );
}
