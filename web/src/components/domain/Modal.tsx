import React, { useEffect, useId, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { cn } from "@/lib/utils";

interface ModalProps {
  open: boolean;
  title: string;
  description?: string;
  onClose: () => void;
  showHeaderClose?: boolean;
  showFooterClose?: boolean;
  overlayClassName?: string;
  cardClassName?: string;
  contentClassName?: string;
  children?: ReactNode;
}

export function Modal({
  open,
  title,
  description,
  onClose,
  showHeaderClose = false,
  showFooterClose = true,
  overlayClassName,
  cardClassName,
  contentClassName,
  children,
}: ModalProps) {
  const titleId = useId();
  const descriptionId = useId();
  const dialogRef = useRef<HTMLDivElement>(null);
  const closeButtonRef = useRef<HTMLButtonElement>(null);
  const onCloseRef = useRef(onClose);
  const [portalTarget, setPortalTarget] = useState<HTMLElement | null>(null);

  useEffect(() => {
    onCloseRef.current = onClose;
  }, [onClose]);

  useEffect(() => {
    setPortalTarget(document.body);
  }, []);

  useEffect(() => {
    if (!open) {
      return;
    }

    const activeElement =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const previousOverflow = document.body.style.overflow;
    const previousPaddingRight = document.body.style.paddingRight;
    const scrollbarCompensation = Math.max(
      0,
      window.innerWidth - document.documentElement.clientWidth
    );

    document.body.style.overflow = "hidden";
    if (scrollbarCompensation > 0) {
      document.body.style.paddingRight = `${scrollbarCompensation}px`;
    }

    window.requestAnimationFrame(() => {
      closeButtonRef.current?.focus();
    });

    function onEsc(event: KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        onCloseRef.current();
        return;
      }

      if (event.key !== "Tab") {
        return;
      }

      const dialog = dialogRef.current;
      if (!dialog) {
        return;
      }

      const focusable = Array.from(
        dialog.querySelectorAll<HTMLElement>(
          'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])'
        )
      ).filter((el) => !el.hasAttribute("disabled") && el.getAttribute("aria-hidden") !== "true");

      if (focusable.length === 0) {
        event.preventDefault();
        return;
      }

      const first = focusable[0];
      const last = focusable[focusable.length - 1];

      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last?.focus();
        return;
      }

      if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first?.focus();
      }
    }

    window.addEventListener("keydown", onEsc);
    return () => {
      window.removeEventListener("keydown", onEsc);
      document.body.style.overflow = previousOverflow;
      document.body.style.paddingRight = previousPaddingRight;
      activeElement?.focus();
    };
  }, [open]);

  if (!open || !portalTarget) {
    return null;
  }

  return createPortal(
    <div
      className={cn(
        "fixed inset-0 z-[130] overflow-y-auto bg-black/80 p-4 backdrop-blur-sm sm:p-6",
        overlayClassName
      )}
      onMouseDown={(event) => {
        if (event.currentTarget === event.target) {
          onClose();
        }
      }}
    >
      <div
        className="flex min-h-full items-start justify-center py-2 sm:items-center sm:py-4"
        onMouseDown={(event) => {
          if (event.currentTarget === event.target) onClose();
        }}
      >
        <div
          ref={dialogRef}
          role="dialog"
          aria-modal="true"
          aria-labelledby={titleId}
          aria-describedby={description ? descriptionId : undefined}
        >
          <Card
            variant="elevated"
            className={cn(
              "max-h-[calc(100dvh-1.5rem)] w-full max-w-lg sm:max-h-[calc(100dvh-3rem)]",
              cardClassName
            )}
          >
            <CardHeader
              className={cn(
                showHeaderClose ? "flex-row items-start justify-between gap-3 space-y-0" : ""
              )}
            >
              <div className={cn(showHeaderClose ? "min-w-0 space-y-1.5" : "")}>
                <CardTitle id={titleId}>{title}</CardTitle>
                {description ? (
                  <CardDescription id={descriptionId}>{description}</CardDescription>
                ) : null}
              </div>
              {showHeaderClose ? (
                <Button
                  ref={closeButtonRef}
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="h-8 w-8 shrink-0 p-0"
                  onClick={onClose}
                  aria-label="关闭弹窗"
                >
                  X
                </Button>
              ) : null}
            </CardHeader>
            <CardContent
              className={cn(
                "max-h-[calc(100dvh-16rem)] overflow-y-auto sm:max-h-[calc(100dvh-18rem)]",
                contentClassName
              )}
            >
              {children}
            </CardContent>
            {showFooterClose ? (
              <CardFooter className="justify-end">
                <Button
                  ref={showHeaderClose ? undefined : closeButtonRef}
                  variant="secondary"
                  onClick={onClose}
                >
                  关闭
                </Button>
              </CardFooter>
            ) : null}
          </Card>
        </div>
      </div>
    </div>,
    portalTarget
  );
}
