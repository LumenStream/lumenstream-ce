import { Wallet, Play, Share2, ScrollText } from "lucide-react";
import type { ProfileSection } from "@/lib/edition/capabilities";
import { cn } from "@/lib/utils";

interface ProfileNavProps {
  activeSection: ProfileSection;
  onSectionChange: (section: ProfileSection) => void;
  availableSections: ProfileSection[];
  className?: string;
}

const navItems: { id: ProfileSection; label: string; icon: React.ReactNode }[] = [
  { id: "billing", label: "账单流量", icon: <Wallet className="h-4 w-4" /> },
  { id: "playback", label: "播放设置", icon: <Play className="h-4 w-4" /> },
  { id: "social", label: "社交邀请", icon: <Share2 className="h-4 w-4" /> },
  { id: "traffic", label: "流量明细", icon: <ScrollText className="h-4 w-4" /> },
];

export function ProfileNav({
  activeSection,
  onSectionChange,
  availableSections,
  className,
}: ProfileNavProps) {
  return (
    <nav className={cn(className)} aria-label="个人中心导航">
      {navItems
        .filter((item) => availableSections.includes(item.id))
        .map((item) => (
          <button
            key={item.id}
            type="button"
            aria-current={activeSection === item.id ? "page" : undefined}
            className={cn(
              "focus-visible:ring-ring flex items-center gap-2.5 rounded-lg px-3 py-2.5 text-sm font-medium transition-all duration-300 focus-visible:ring-2 focus-visible:outline-none",
              activeSection === item.id
                ? "light:bg-black/[0.04] light:text-foreground bg-white/[0.06] text-white"
                : "text-muted-foreground hover:text-foreground"
            )}
            onClick={() => onSectionChange(item.id)}
          >
            {item.icon}
            {item.label}
          </button>
        ))}
    </nav>
  );
}
