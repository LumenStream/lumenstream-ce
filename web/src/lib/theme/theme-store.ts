import { useSyncExternalStore } from "react";

export type Theme = "light" | "dark";

type Listener = () => void;

let theme: Theme = "dark";
const listeners = new Set<Listener>();

function emitChange() {
  for (const listener of listeners) {
    listener();
  }
}

function applyTheme(newTheme: Theme) {
  const root = document.documentElement;
  root.classList.remove("light", "dark");
  root.classList.add(newTheme);
  theme = newTheme;
  emitChange();
}

export function setTheme(newTheme: Theme, event?: MouseEvent) {
  if (newTheme === theme) return;

  localStorage.setItem("theme", newTheme);

  // Use View Transitions API with circular clip-path animation
  if (document.startViewTransition && event) {
    const x = event.clientX;
    const y = event.clientY;
    const endRadius = Math.hypot(
      Math.max(x, window.innerWidth - x),
      Math.max(y, window.innerHeight - y)
    );

    const transition = document.startViewTransition(() => {
      applyTheme(newTheme);
    });

    transition.ready.then(() => {
      // Always animate new view expanding from click point
      document.documentElement.animate(
        {
          clipPath: [`circle(0px at ${x}px ${y}px)`, `circle(${endRadius}px at ${x}px ${y}px)`],
        },
        {
          duration: 350,
          easing: "ease-out",
          pseudoElement: "::view-transition-new(root)",
        }
      );
    });
  } else {
    applyTheme(newTheme);
  }
}

export function cycleTheme(event?: React.MouseEvent) {
  const newTheme = theme === "dark" ? "light" : "dark";
  setTheme(newTheme, event?.nativeEvent);
}

export function initializeTheme() {
  const stored = localStorage.getItem("theme") as Theme | null;
  const systemPrefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  const initialTheme = stored ?? (systemPrefersDark ? "dark" : "light");

  applyTheme(initialTheme);

  // Listen for system theme changes (only if user hasn't manually set a preference)
  const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
  mediaQuery.addEventListener("change", (e) => {
    if (!localStorage.getItem("theme")) {
      applyTheme(e.matches ? "dark" : "light");
    }
  });
}

function subscribe(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function getSnapshot(): Theme {
  return theme;
}

export function useTheme(): Theme {
  return useSyncExternalStore(subscribe, getSnapshot, () => "dark");
}
