import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { formatEpisodeLabel } from "@/lib/media/episode-label";
import type { BaseItem } from "@/lib/types/jellyfin";

interface MediaCardProps {
  item: BaseItem;
  href: string;
}

function formatRuntimeTicks(runtimeTicks?: number | null): string {
  if (!runtimeTicks) {
    return "未知时长";
  }

  const seconds = Math.floor(runtimeTicks / 10_000_000);
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);

  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }

  return `${minutes}m`;
}

export function MediaCard({ item, href }: MediaCardProps) {
  return (
    <a href={href} className="block">
      <Card className="h-full transition-transform hover:-translate-y-0.5 hover:border-rose-700/50">
        <CardHeader className="space-y-2">
          <div className="flex items-center justify-between gap-2">
            <CardTitle className="line-clamp-1 text-base">{formatEpisodeLabel(item)}</CardTitle>
            <Badge variant="secondary">{item.Type}</Badge>
          </div>
          <CardDescription className="line-clamp-1">
            {item.Overview || "仅展示媒体元信息（已隐藏文件路径）"}
          </CardDescription>
        </CardHeader>
        <CardContent className="text-muted-foreground space-y-2 text-xs">
          <p>时长：{formatRuntimeTicks(item.RunTimeTicks)}</p>
          <p>码率：{item.Bitrate ? `${Math.round(item.Bitrate / 1000)} kbps` : "未知"}</p>
          {item.UserData ? (
            <p>
              观看：{item.UserData.Played ? "已观看" : "未观看"}
              {item.UserData.PlaybackPositionTicks > 0 ? " · 有续播进度" : ""}
            </p>
          ) : null}
        </CardContent>
      </Card>
    </a>
  );
}
