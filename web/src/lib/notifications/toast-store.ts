import { useSyncExternalStore } from "react";

export type ToastVariant = "success" | "error" | "warning" | "info";

export interface Toast {
  id: string;
  message: string;
  variant: ToastVariant;
  duration: number;
}

type Listener = () => void;

let toasts: Toast[] = [];
const listeners = new Set<Listener>();

function emitChange() {
  for (const listener of listeners) {
    listener();
  }
}

function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

export function addToast(message: string, variant: ToastVariant = "info", duration = 5000): string {
  const id = generateId();
  toasts = [...toasts, { id, message, variant, duration }];
  emitChange();
  return id;
}

export function removeToast(id: string): void {
  toasts = toasts.filter((t) => t.id !== id);
  emitChange();
}

export function clearToasts(): void {
  toasts = [];
  emitChange();
}

function subscribe(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function getSnapshot(): Toast[] {
  return toasts;
}

export function useToasts(): Toast[] {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}

export const toast = {
  success: (message: string, duration?: number) => addToast(message, "success", duration),
  error: (message: string, duration?: number) => addToast(message, "error", duration),
  warning: (message: string, duration?: number) => addToast(message, "warning", duration),
  info: (message: string, duration?: number) => addToast(message, "info", duration),
  dismiss: removeToast,
  clear: clearToasts,
};
