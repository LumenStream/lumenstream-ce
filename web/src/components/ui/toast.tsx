import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { X, CheckCircle, AlertCircle, AlertTriangle, Info } from "lucide-react";

import { cn } from "@/lib/utils";

const toastVariants = cva(
  "pointer-events-auto relative flex w-full items-center gap-3 rounded-lg border p-4 pr-10 shadow-lg transition-all",
  {
    variants: {
      variant: {
        success: "border-emerald-500/30 bg-emerald-950/90 text-emerald-100",
        error: "border-red-500/30 bg-red-950/90 text-red-100",
        warning: "border-amber-500/30 bg-amber-950/90 text-amber-100",
        info: "border-sky-500/30 bg-sky-950/90 text-sky-100",
      },
    },
    defaultVariants: {
      variant: "info",
    },
  }
);

const iconMap = {
  success: CheckCircle,
  error: AlertCircle,
  warning: AlertTriangle,
  info: Info,
};

const iconColorMap = {
  success: "text-emerald-400",
  error: "text-red-400",
  warning: "text-amber-400",
  info: "text-sky-400",
};

export interface ToastProps
  extends React.HTMLAttributes<HTMLDivElement>, VariantProps<typeof toastVariants> {
  onClose?: () => void;
}

export function Toast({ className, variant = "info", children, onClose, ...props }: ToastProps) {
  const Icon = iconMap[variant ?? "info"];
  const iconColor = iconColorMap[variant ?? "info"];

  return (
    <div
      role="alert"
      aria-live="polite"
      className={cn(toastVariants({ variant }), className)}
      {...props}
    >
      <Icon className={cn("h-5 w-5 shrink-0", iconColor)} aria-hidden="true" />
      <div className="flex-1 text-sm font-medium">{children}</div>
      {onClose && (
        <button
          type="button"
          onClick={onClose}
          className="absolute top-2 right-2 rounded-md p-1.5 opacity-70 transition-opacity hover:opacity-100 focus:ring-2 focus:ring-white/20 focus:outline-none"
          aria-label="关闭通知"
        >
          <X className="h-4 w-4" />
        </button>
      )}
    </div>
  );
}
