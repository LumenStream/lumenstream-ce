import React, { useEffect, useState } from "react";

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
import { getTopTrafficUsers } from "@/lib/api/admin-commercial";
import type { ApiError } from "@/lib/api/client";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import type { TopTrafficUser } from "@/lib/types/edition-commercial";

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(2)} ${units[i]}`;
}

export function AdminTrafficPanel() {
  const { ready } = useAuthSession({ requireAdmin: true });
  const [topUsers, setTopUsers] = useState<TopTrafficUser[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  async function reload() {
    setLoading(true);
    try {
      const topList = await getTopTrafficUsers(20);
      setTopUsers(topList);
      setError(null);
    } catch (cause) {
      const apiError = cause as ApiError;
      setError(apiError.message || "加载数据失败");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (!ready) return;
    void reload();
  }, [ready]);

  if (!ready || loading) {
    return <LoadingState title="加载流量管理" />;
  }

  if (error) {
    return <ErrorState title="流量管理加载失败" description={error} />;
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-sm font-medium">流量排行</h3>
          <p className="text-muted-foreground mt-1 text-xs">本月流量使用最多的用户</p>
        </div>
        <Button variant="outline" size="sm" onClick={() => void reload()}>
          刷新
        </Button>
      </div>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>排名</TableHead>
            <TableHead>用户名</TableHead>
            <TableHead>本月流量</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {topUsers.map((user, index) => (
            <TableRow key={user.user_id}>
              <TableCell>
                <Badge variant={index < 3 ? "danger" : "outline"}>{index + 1}</Badge>
              </TableCell>
              <TableCell>
                <div>
                  <p>{user.username}</p>
                  <p className="text-muted-foreground font-mono text-[10px]">{user.user_id}</p>
                </div>
              </TableCell>
              <TableCell>{formatBytes(user.used_bytes)}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}
