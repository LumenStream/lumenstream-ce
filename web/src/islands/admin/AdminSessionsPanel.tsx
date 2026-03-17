import { useEffect, useState } from "react";

import { ErrorState, LoadingState } from "@/components/domain/DataState";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { listPlaybackSessions } from "@/lib/api/admin";
import type { ApiError } from "@/lib/api/client";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import type { PlaybackSession } from "@/lib/types/admin";
import { formatDate } from "@/lib/utils";

export function AdminSessionsPanel() {
  const { ready } = useAuthSession({ requireAdmin: true });
  const [playbackSessions, setPlaybackSessions] = useState<PlaybackSession[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  async function reload() {
    setLoading(true);
    try {
      const playback = await listPlaybackSessions({ limit: 80, active_only: true });
      setPlaybackSessions(
        playback.filter(
          (item) => item.is_active && Boolean(item.media_item_id || item.media_item_name)
        )
      );
      setError(null);
    } catch (cause) {
      const apiError = cause as ApiError;
      setError(apiError.message || "加载会话失败");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (!ready) {
      return;
    }
    void reload();
  }, [ready]);

  if (!ready || loading) {
    return <LoadingState title="加载会话数据" />;
  }

  if (error) {
    return <ErrorState title="会话页面加载失败" description={error} />;
  }

  return (
    <div className="space-y-8">
      <div className="flex justify-end">
        <Button variant="glass" onClick={() => void reload()}>
          刷新
        </Button>
      </div>

      <section className="border-border/50 border-b pb-6">
        <h3 className="text-sm font-medium">播放会话</h3>
        <p className="text-muted-foreground mt-1 mb-4 text-xs">
          来源：`/admin/sessions`（仅显示正在播放）
        </p>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>用户</TableHead>
              <TableHead>媒体</TableHead>
              <TableHead>设备</TableHead>
              <TableHead>方法</TableHead>
              <TableHead>状态</TableHead>
              <TableHead>更新时间</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {playbackSessions.length === 0 ? (
              <TableRow>
                <TableCell colSpan={6} className="text-muted-foreground text-center text-xs">
                  暂无正在播放会话
                </TableCell>
              </TableRow>
            ) : (
              playbackSessions.map((item) => (
                <TableRow key={item.id}>
                  <TableCell>{item.user_name}</TableCell>
                  <TableCell>{item.media_item_name || "-"}</TableCell>
                  <TableCell>{item.device_name || item.client_name || "-"}</TableCell>
                  <TableCell>{item.play_method || "-"}</TableCell>
                  <TableCell>
                    {item.is_active ? (
                      <Badge variant="success">活跃</Badge>
                    ) : (
                      <Badge variant="glass">结束</Badge>
                    )}
                  </TableCell>
                  <TableCell>{formatDate(item.updated_at)}</TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </section>
    </div>
  );
}
