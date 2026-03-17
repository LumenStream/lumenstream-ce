import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";

const cardVariants = cva("", {
  variants: {
    variant: {
      default: "text-card-foreground",
      surface:
        "text-card-foreground rounded-2xl border border-white/[0.04] bg-white/[0.02] backdrop-blur-sm light:border-black/[0.06] light:bg-black/[0.02]",
      elevated:
        "text-card-foreground rounded-2xl border border-white/[0.06] bg-card/80 shadow-2xl backdrop-blur-xl",
      legacy:
        "text-card-foreground border-border bg-card/50 rounded-2xl border backdrop-blur-xl transition-all duration-300 hover:border-rose-500/30 hover:shadow-[0_0_30px_rgba(244,63,94,0.18)]",
    },
  },
  defaultVariants: {
    variant: "default",
  },
});

export interface CardProps
  extends React.HTMLAttributes<HTMLDivElement>, VariantProps<typeof cardVariants> {}

export function Card({ className, variant, ...props }: CardProps) {
  return <div className={cn(cardVariants({ variant }), className)} {...props} />;
}

export function CardHeader({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("flex flex-col space-y-1.5 p-6", className)} {...props} />;
}

export function CardTitle({ className, ...props }: React.HTMLAttributes<HTMLHeadingElement>) {
  return (
    <h3 className={cn("text-lg leading-none font-semibold tracking-tight", className)} {...props} />
  );
}

export function CardDescription({
  className,
  ...props
}: React.HTMLAttributes<HTMLParagraphElement>) {
  return <p className={cn("text-muted-foreground text-sm", className)} {...props} />;
}

export function CardContent({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("p-6 pt-0", className)} {...props} />;
}

export function CardFooter({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("flex items-center p-6 pt-0", className)} {...props} />;
}
