import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";

const badgeVariants = cva(
  "inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-medium transition-colors",
  {
    variants: {
      variant: {
        default: "border-transparent bg-primary text-primary-foreground",
        secondary: "border-transparent bg-secondary text-secondary-foreground",
        outline: "border-border text-foreground",
        glass:
          "border-white/10 bg-white/10 text-white/80 backdrop-blur-sm light:border-black/8 light:bg-black/[0.05] light:text-foreground/70",
        success:
          "border-emerald-500/30 bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/20",
        danger: "border-rose-500/30 bg-rose-500/15 text-rose-300 ring-1 ring-rose-500/20",
        warning: "border-amber-500/30 bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/20",
        info: "border-cyan-500/30 bg-cyan-500/15 text-cyan-300 ring-1 ring-cyan-500/20",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  }
);

export interface BadgeProps
  extends React.HTMLAttributes<HTMLDivElement>, VariantProps<typeof badgeVariants> {}

export function Badge({ className, variant, ...props }: BadgeProps) {
  return <div className={cn(badgeVariants({ variant }), className)} {...props} />;
}
