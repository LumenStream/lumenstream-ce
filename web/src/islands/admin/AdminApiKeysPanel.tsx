import React, { useEffect, useState } from "react";

import { ErrorState, LoadingState } from "@/components/domain/DataState";
import { Modal } from "@/components/domain/Modal";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { createApiKey, deleteApiKey, listApiKeys } from "@/lib/api/admin";
import type { ApiError } from "@/lib/api/client";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import { toast } from "@/lib/notifications/toast-store";
import type { AdminApiKey, AdminCreatedApiKey } from "@/lib/types/admin";
import { formatDate } from "@/lib/utils";

const API_KEY_MASK = "****";
const API_KEY_LIST_LIMIT = 100;

type DisplayedAdminApiKey = AdminApiKey & {
  plain_text_key?: string;
};

function buildCreatedApiKey(created: AdminCreatedApiKey): DisplayedAdminApiKey {
  return {
    id: created.id,
    name: created.name,
    created_at: created.created_at,
    last_used_at: null,
    plain_text_key: created.api_key,
  };
}

export function AdminApiKeysPanel() {
  const { ready } = useAuthSession({ requireAdmin: true });
  const [keys, setKeys] = useState<DisplayedAdminApiKey[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [deleteTarget, setDeleteTarget] = useState<DisplayedAdminApiKey | null>(null);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  async function reload() {
    setLoading(true);
    try {
      const payload = await listApiKeys(API_KEY_LIST_LIMIT);
      setKeys(payload);
      setError(null);
    } catch (cause) {
      const apiError = cause as ApiError;
      setError(apiError.message || "加载 API Keys 失败");
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

  async function onCreate(event: React.SubmitEvent<HTMLFormElement>) {
    event.preventDefault();

    try {
      const result = await createApiKey(name.trim());
      setKeys((current) => [buildCreatedApiKey(result), ...current].slice(0, API_KEY_LIST_LIMIT));
      setName("");
      toast.success("创建成功，已在列表中显示新密钥，请立即保存。");
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(`创建失败：${apiError.message}`);
    }
  }

  async function onConfirmDelete() {
    if (!deleteTarget) {
      return;
    }

    setDeletingId(deleteTarget.id);
    try {
      await deleteApiKey(deleteTarget.id);
      setKeys((current) => current.filter((key) => key.id !== deleteTarget.id));
      setDeleteTarget(null);
      toast.success(`密钥 ${deleteTarget.name} 已删除`);
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(`删除失败：${apiError.message}`);
    } finally {
      setDeletingId(null);
    }
  }

  if (!ready || loading) {
    return <LoadingState title="加载 API Keys" />;
  }

  if (error) {
    return <ErrorState title="API Keys 页面加载失败" description={error} />;
  }

  return (
    <div className="space-y-8">
      <section className="border-border/50 border-b pb-6">
        <h3 className="text-sm font-medium">创建 API Key</h3>
        <p className="text-muted-foreground mt-1 mb-4 text-xs">
          仅首次创建会返回明文密钥；当前页会在列表中显示本次生成结果，刷新后仅显示
          {API_KEY_MASK}。
        </p>
        <form className="grid gap-3 sm:grid-cols-[1fr_auto]" onSubmit={onCreate}>
          <Input
            placeholder="Key 名称"
            value={name}
            onChange={(event) => setName(event.target.value)}
            required
          />
          <Button type="submit">创建</Button>
        </form>
      </section>

      <section>
        <h3 className="text-sm font-medium">API Key 列表</h3>
        <div className="mt-4 space-y-3">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>名称</TableHead>
                <TableHead>密钥</TableHead>
                <TableHead>创建时间</TableHead>
                <TableHead>最后使用</TableHead>
                <TableHead>操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {keys.map((key) => (
                <TableRow key={key.id}>
                  <TableCell>{key.name}</TableCell>
                  <TableCell>
                    <span className="font-mono text-xs break-all">
                      {key.plain_text_key ?? API_KEY_MASK}
                    </span>
                  </TableCell>
                  <TableCell>{formatDate(key.created_at)}</TableCell>
                  <TableCell>{formatDate(key.last_used_at)}</TableCell>
                  <TableCell>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => setDeleteTarget(key)}
                      disabled={deletingId === key.id}
                    >
                      删除
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      </section>

      <Modal
        open={Boolean(deleteTarget)}
        title="确认删除 API Key"
        description="删除后该密钥将立即失效，且不可恢复。"
        onClose={() => setDeleteTarget(null)}
        showHeaderClose
        showFooterClose={false}
      >
        <div className="space-y-4 text-sm">
          <p>
            确认删除密钥 <span className="font-mono">{deleteTarget?.name}</span> 吗？
          </p>
          <div className="flex justify-end gap-2">
            <Button type="button" variant="secondary" onClick={() => setDeleteTarget(null)}>
              取消
            </Button>
            <Button
              type="button"
              variant="destructive"
              onClick={() => void onConfirmDelete()}
              disabled={!deleteTarget || deletingId === deleteTarget.id}
            >
              {deletingId && deleteTarget?.id === deletingId ? "删除中..." : "确认删除"}
            </Button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
