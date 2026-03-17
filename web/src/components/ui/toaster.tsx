"use client";

import * as React from "react";
import { useToasts, removeToast, type Toast as ToastData } from "@/lib/notifications/toast-store";
import { Toast } from "./toast";
import { cn } from "@/lib/utils";

export function Toaster() {
  const toasts = useToasts();

  return (
    <div
      aria-label="通知"
      className="pointer-events-none fixed top-4 right-4 z-50 flex max-h-screen w-full max-w-sm flex-col gap-2"
    >
      {toasts.map((toast) => (
        <ToastItem key={toast.id} toast={toast} />
      ))}
    </div>
  );
}

function ToastItem({ toast }: { toast: ToastData }) {
  const [isVisible, setIsVisible] = React.useState(false);
  const [isLeaving, setIsLeaving] = React.useState(false);

  const handleClose = React.useCallback(() => {
    setIsLeaving(true);
    setTimeout(() => {
      removeToast(toast.id);
    }, 200);
  }, [toast.id]);

  React.useEffect(() => {
    const showTimer = requestAnimationFrame(() => setIsVisible(true));
    return () => cancelAnimationFrame(showTimer);
  }, []);

  React.useEffect(() => {
    if (toast.duration <= 0) return;

    const timer = setTimeout(() => {
      handleClose();
    }, toast.duration);

    return () => clearTimeout(timer);
  }, [toast.duration, toast.id, handleClose]);

  return (
    <div
      className={cn(
        "transform transition-all duration-200 ease-out",
        isVisible && !isLeaving
          ? "translate-y-0 opacity-100"
          : isLeaving
            ? "-translate-y-2 opacity-0"
            : "translate-y-4 opacity-0"
      )}
    >
      <Toast variant={toast.variant} onClose={handleClose}>
        {toast.message}
      </Toast>
    </div>
  );
}
