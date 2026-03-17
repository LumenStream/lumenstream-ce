import { useEffect } from "react";
import { motion } from "framer-motion";
import { useTheme, cycleTheme, initializeTheme } from "@/lib/theme/theme-store";

export function ThemeToggle() {
  const theme = useTheme();

  useEffect(() => {
    initializeTheme();
  }, []);

  const handleClick = (e: React.MouseEvent) => {
    cycleTheme(e);
  };

  return (
    <button
      type="button"
      onClick={handleClick}
      className="border-border text-muted-foreground hover:text-foreground focus-visible:ring-ring light:hover:border-black/20 relative flex cursor-pointer items-center justify-center rounded-md border p-2 transition-colors hover:border-white/30 focus-visible:ring-2 focus-visible:outline-none"
      aria-label={theme === "dark" ? "切换到浅色模式" : "切换到深色模式"}
    >
      <motion.div
        initial={false}
        animate={{ rotate: theme === "dark" ? 0 : 180 }}
        transition={{ duration: 0.3, ease: "easeInOut" }}
      >
        {theme === "dark" ? (
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z" />
          </svg>
        ) : (
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <circle cx="12" cy="12" r="4" />
            <path d="M12 2v2" />
            <path d="M12 20v2" />
            <path d="m4.93 4.93 1.41 1.41" />
            <path d="m17.66 17.66 1.41 1.41" />
            <path d="M2 12h2" />
            <path d="M20 12h2" />
            <path d="m6.34 17.66-1.41 1.41" />
            <path d="m19.07 4.93-1.41 1.41" />
          </svg>
        )}
      </motion.div>
    </button>
  );
}

export default ThemeToggle;
