import { motion } from "framer-motion";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";

interface ProfileHeroProps {
  userName: string;
  userId: string;
  role: string;
  mockMode: boolean;
  showUserId: boolean;
  onToggleUserId: () => void;
  onLogout: () => void;
}

export function ProfileHero({
  userName,
  userId,
  role,
  mockMode,
  showUserId,
  onToggleUserId,
  onLogout,
}: ProfileHeroProps) {
  const initials = (userName || "U").slice(0, 2).toUpperCase();

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.5, ease: [0.22, 1, 0.36, 1] }}
      className="border-border/50 border-b pb-6"
    >
      <div className="flex items-center gap-5">
        <div className="bg-primary/15 text-primary ring-primary/20 flex h-16 w-16 shrink-0 items-center justify-center rounded-full text-xl font-bold ring-2 sm:h-20 sm:w-20 sm:text-2xl">
          {initials}
        </div>

        <div className="min-w-0 flex-1 space-y-1.5">
          <div className="flex flex-wrap items-center gap-2.5">
            <h2 className="text-xl font-semibold sm:text-2xl">{userName}</h2>
            <Badge variant="glass">{role}</Badge>
            {mockMode && <Badge variant="warning">Mock</Badge>}
          </div>
          <button
            type="button"
            className="text-muted-foreground hover:text-foreground text-xs transition-colors"
            onClick={onToggleUserId}
          >
            {showUserId ? userId : "点击显示 UserId"}
          </button>
        </div>

        <Button
          variant="ghost"
          size="sm"
          className="text-muted-foreground shrink-0"
          onClick={onLogout}
        >
          {mockMode ? "退出演示" : "退出登录"}
        </Button>
      </div>
    </motion.div>
  );
}
