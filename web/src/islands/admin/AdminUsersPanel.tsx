import React, { useEffect, useMemo, useRef, useState } from "react";

import { ErrorState, LoadingState } from "@/components/domain/DataState";
import { cn } from "@/lib/utils";
import { Modal } from "@/components/domain/Modal";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { Select } from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  batchSetUserEnabled,
  createUser,
  deleteUser,
  getAdminUserProfile,
  listUserSummaries,
  patchUserProfile,
} from "@/lib/api/admin";
import {
  adminAdjustBalance,
  adminAssignSubscription,
  adminCancelSubscription,
  adminGetUserLedger,
  adminGetUserWallet,
  adminListPlans,
  adminUpdateSubscription,
  resetUserTrafficUsage,
  setUserStreamPolicy,
} from "@/lib/api/admin-commercial";
import type { ApiError } from "@/lib/api/client";
import { getPublicSystemCapabilities } from "@/lib/api/system";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import { toast } from "@/lib/notifications/toast-store";
import type { LedgerEntry, Plan, Subscription, Wallet } from "@/lib/types/billing";
import type {
  AdminSystemCapabilities,
  AdminUserManageProfile,
  AdminUserSummaryItem,
  AdminUserSummaryPage,
  UserRole,
} from "@/lib/types/admin";

function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  return `${(bytes / Math.pow(1024, index)).toFixed(2)} ${units[index]}`;
}

function formatDate(value: string | null | undefined): string {
  if (!value) return "-";
  return new Date(value).toLocaleString("zh-CN");
}

function formatPrice(price: string): string {
  return `¥${parseFloat(price).toFixed(2)}`;
}

function normalizeDeleteConfirmName(value: string): string {
  return value.trim();
}

function toDateTimeLocalValue(value: string | null): string {
  if (!value) return "";
  const date = new Date(value);
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 16);
}

type UserStatusFilter = "all" | "enabled" | "disabled";
type DetailTab =
  | "profile"
  | "limits"
  | "traffic"
  | "wallet"
  | "subscriptions"
  | "sessions"
  | "danger";

const DETAIL_TABS: { id: DetailTab; label: string }[] = [
  { id: "profile", label: "资料" },
  { id: "limits", label: "限制" },
  { id: "traffic", label: "流量" },
  { id: "wallet", label: "钱包" },
  { id: "subscriptions", label: "订阅" },
  { id: "sessions", label: "会话" },
  { id: "danger", label: "危险" },
];

export function AdminUsersPanel() {
  const { ready } = useAuthSession({ requireAdmin: true });

  const [capabilities, setCapabilities] = useState<AdminSystemCapabilities | null>(null);
  const [summary, setSummary] = useState<AdminUserSummaryPage | null>(null);
  const [usersLoading, setUsersLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [queryText, setQueryText] = useState("");
  const [statusFilter, setStatusFilter] = useState<UserStatusFilter>("all");
  const [roleFilter, setRoleFilter] = useState<"all" | UserRole>("all");
  const [sortBy, setSortBy] = useState<
    "id" | "email" | "online_devices" | "status" | "subscription" | "role" | "used_bytes"
  >("id");
  const [sortDir, setSortDir] = useState<"asc" | "desc">("desc");
  const [page, setPage] = useState(1);
  const pageSize = 20;
  const [selectedIds, setSelectedIds] = useState<string[]>([]);

  const [selectedUserId, setSelectedUserId] = useState<string | null>(null);
  const selectedUserIdRef = useRef(selectedUserId);
  selectedUserIdRef.current = selectedUserId;
  const [detail, setDetail] = useState<AdminUserManageProfile | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [_detailError, setDetailError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<DetailTab>("profile");

  const [plans, setPlans] = useState<Plan[]>([]);
  const [wallet, setWallet] = useState<Wallet | null>(null);
  const [ledger, setLedger] = useState<LedgerEntry[]>([]);

  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [confirmResetTrafficOpen, setConfirmResetTrafficOpen] = useState(false);
  const [confirmDeleteOpen, setConfirmDeleteOpen] = useState(false);
  const [cancelSubscriptionTarget, setCancelSubscriptionTarget] = useState<Subscription | null>(
    null
  );

  const [newUsername, setNewUsername] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [newRole, setNewRole] = useState<UserRole>("Viewer");

  const [email, setEmail] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [remark, setRemark] = useState("");
  const [role, setRole] = useState<UserRole>("Viewer");
  const [disabled, setDisabled] = useState(false);

  const [maxConcurrentStreams, setMaxConcurrentStreams] = useState("");
  const [trafficQuotaGb, setTrafficQuotaGb] = useState("");
  const [trafficWindowDays, setTrafficWindowDays] = useState("30");
  const [expiresAt, setExpiresAt] = useState("");

  const [adjustAmount, setAdjustAmount] = useState("");
  const [adjustNote, setAdjustNote] = useState("");
  const [assignPlanId, setAssignPlanId] = useState("");
  const [assignDurationDays, setAssignDurationDays] = useState("");
  const [deleteConfirmName, setDeleteConfirmName] = useState("");

  const [editingSub, setEditingSub] = useState<Subscription | null>(null);
  const [editExpiresAt, setEditExpiresAt] = useState("");
  const [editingSubId, setEditingSubId] = useState<string | null>(null);

  const items = summary?.items ?? [];
  const hasSelection = selectedIds.length > 0;
  const expectedDeleteName = detail?.user.Name ? normalizeDeleteConfirmName(detail.user.Name) : "";
  const deleteConfirmMatched =
    expectedDeleteName.length > 0 &&
    normalizeDeleteConfirmName(deleteConfirmName) === expectedDeleteName;
  const totalPages = useMemo(() => {
    if (!summary) return 1;
    return Math.max(1, Math.ceil(summary.total / summary.page_size));
  }, [summary]);
  const billingEnabled = capabilities?.billing_enabled ?? false;
  const trafficEnabled = capabilities?.advanced_traffic_controls_enabled ?? false;
  const visibleDetailTabs = useMemo(
    () =>
      DETAIL_TABS.filter((tab) => {
        if (tab.id === "limits") {
          return trafficEnabled;
        }
        if (tab.id === "traffic") {
          return trafficEnabled;
        }
        if (tab.id === "wallet" || tab.id === "subscriptions") {
          return billingEnabled;
        }
        return true;
      }),
    [billingEnabled, trafficEnabled]
  );

  const reloadSummaries = React.useCallback(
    async (options?: { keepSelection?: boolean }) => {
      const keepSelection = options?.keepSelection ?? true;
      const payload = await listUserSummaries({
        q: queryText || undefined,
        status: statusFilter,
        role: roleFilter,
        page,
        page_size: pageSize,
        sort_by: sortBy,
        sort_dir: sortDir,
      });
      setSummary(payload);
      setSelectedIds((prev) => prev.filter((id) => payload.items.some((item) => item.id === id)));
      const currentSelectedId = selectedUserIdRef.current;
      const selectionStillExists =
        currentSelectedId !== null && payload.items.some((item) => item.id === currentSelectedId);
      if (!keepSelection || !selectionStillExists) {
        setSelectedUserId(null);
        setDetail(null);
        setWallet(null);
        setLedger([]);
      }
    },
    [queryText, statusFilter, roleFilter, page, pageSize, sortBy, sortDir]
  );

  const initialize = React.useCallback(async () => {
    setUsersLoading(true);
    try {
      const systemCapabilities = await getPublicSystemCapabilities();
      setCapabilities(systemCapabilities);
      if (systemCapabilities.billing_enabled) {
        const plansData = await adminListPlans();
        setPlans(plansData.filter((item) => item.enabled));
      } else {
        setPlans([]);
      }
      await reloadSummaries({ keepSelection: false });
      setError(null);
    } catch (cause) {
      const apiError = cause as ApiError;
      setError(apiError.message || "加载用户管理失败");
    } finally {
      setUsersLoading(false);
    }
  }, [reloadSummaries]);

  useEffect(() => {
    if (!ready) return;
    void initialize();
  }, [ready, initialize]);

  useEffect(() => {
    if (!detail) return;
    const streamPolicy = detail.stream_policy;
    setEmail(detail.profile.email || "");
    setDisplayName(detail.profile.display_name || "");
    setRemark(detail.profile.remark || "");
    setRole((detail.user.Policy.Role || "Viewer") as UserRole);
    setDisabled(detail.user.Policy.IsDisabled);
    setMaxConcurrentStreams(streamPolicy?.max_concurrent_streams?.toString() || "");
    setTrafficQuotaGb(
      streamPolicy?.traffic_quota_bytes
        ? (streamPolicy.traffic_quota_bytes / (1024 * 1024 * 1024)).toString()
        : ""
    );
    setTrafficWindowDays(streamPolicy?.traffic_window_days?.toString() || "30");
    setExpiresAt(toDateTimeLocalValue(streamPolicy?.expires_at || null));
    setDeleteConfirmName("");
  }, [detail]);

  useEffect(() => {
    if (visibleDetailTabs.some((tab) => tab.id === activeTab)) {
      return;
    }
    setActiveTab(visibleDetailTabs[0]?.id ?? "profile");
  }, [activeTab, visibleDetailTabs]);

  async function changePage(nextPage: number) {
    const payload = await listUserSummaries({
      q: queryText || undefined,
      status: statusFilter,
      role: roleFilter,
      page: nextPage,
      page_size: pageSize,
      sort_by: sortBy,
      sort_dir: sortDir,
    });
    setPage(nextPage);
    setSummary(payload);
    setSelectedIds((prev) => prev.filter((id) => payload.items.some((item) => item.id === id)));
    if (selectedUserId && !payload.items.some((item) => item.id === selectedUserId)) {
      setSelectedUserId(null);
      setDetail(null);
      setWallet(null);
      setLedger([]);
    }
  }

  async function loadUserDetail(userId: string) {
    setDetailLoading(true);
    setDetailError(null);
    setWallet(null);
    setLedger([]);
    try {
      const [profileData, walletData, ledgerData] = await Promise.all([
        getAdminUserProfile(userId),
        billingEnabled ? adminGetUserWallet(userId).catch(() => null) : Promise.resolve(null),
        billingEnabled ? adminGetUserLedger(userId).catch(() => []) : Promise.resolve([]),
      ]);
      setDetail(profileData);
      setWallet(walletData);
      setLedger(ledgerData);
    } catch (cause) {
      const apiError = cause as ApiError;
      setDetail(null);
      toast.error(apiError.message || "加载用户详情失败");
      setDetailError("加载用户详情失败");
    } finally {
      setDetailLoading(false);
    }
  }
  async function onSearch(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await changePage(1);
  }

  async function onCreateUser(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    try {
      await createUser({ username: newUsername.trim(), password: newPassword, role: newRole });
      toast.success("用户创建成功");
      setNewUsername("");
      setNewPassword("");
      setNewRole("Viewer");
      setCreateModalOpen(false);
      await reloadSummaries({ keepSelection: true });
    } catch (cause) {
      toast.error(`创建失败：${(cause as ApiError).message}`);
    }
  }

  async function onBatchStatus(enabled: boolean) {
    if (!hasSelection) return;
    try {
      const result = await batchSetUserEnabled(selectedIds, enabled);
      toast.success(`批量状态更新完成，影响 ${result.updated} 个用户`);
      setSelectedIds([]);
      await reloadSummaries({ keepSelection: true });
    } catch (cause) {
      toast.error(`批量更新失败：${(cause as ApiError).message}`);
    }
  }

  async function onSelectUser(item: AdminUserSummaryItem) {
    setSelectedUserId(item.id);
    setActiveTab("profile");
    setEditingSub(null);
    await loadUserDetail(item.id);
  }

  async function onSaveProfile(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedUserId) return;
    try {
      const payload = await patchUserProfile(selectedUserId, {
        email: email.trim() || null,
        display_name: displayName.trim() || null,
        remark: remark.trim() || null,
        role,
        is_disabled: disabled,
      });
      setDetail(payload);
      toast.success("用户资料已更新");
      await reloadSummaries({ keepSelection: true });
    } catch (cause) {
      toast.error(`资料更新失败：${(cause as ApiError).message}`);
    }
  }

  async function onSaveStreamPolicy(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedUserId) return;
    try {
      await setUserStreamPolicy(selectedUserId, {
        max_concurrent_streams: maxConcurrentStreams ? parseInt(maxConcurrentStreams, 10) : null,
        traffic_quota_bytes: trafficQuotaGb
          ? Math.round(parseFloat(trafficQuotaGb) * 1024 * 1024 * 1024)
          : null,
        traffic_window_days: parseInt(trafficWindowDays, 10) || 30,
        expires_at: expiresAt ? new Date(expiresAt).toISOString() : null,
      });
      await loadUserDetail(selectedUserId);
      toast.success("流策略已更新");
      await reloadSummaries({ keepSelection: true });
    } catch (cause) {
      toast.error(`流策略更新失败：${(cause as ApiError).message}`);
    }
  }

  async function onResetTraffic() {
    if (!selectedUserId) return;
    try {
      await resetUserTrafficUsage(selectedUserId);
      await loadUserDetail(selectedUserId);
      await reloadSummaries({ keepSelection: true });
      toast.success("用户流量统计已重置");
      setConfirmResetTrafficOpen(false);
    } catch (cause) {
      toast.error(`重置流量失败：${(cause as ApiError).message}`);
    }
  }

  async function onAdjustBalance(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedUserId) return;
    try {
      await adminAdjustBalance(selectedUserId, { amount: adjustAmount, note: adjustNote });
      setAdjustAmount("");
      setAdjustNote("");
      const [w, l] = await Promise.all([
        adminGetUserWallet(selectedUserId),
        adminGetUserLedger(selectedUserId),
      ]);
      setWallet(w);
      setLedger(l);
      toast.success("钱包余额已更新");
      await reloadSummaries({ keepSelection: true });
    } catch (cause) {
      toast.error(`余额调整失败：${(cause as ApiError).message}`);
    }
  }

  async function onAssignSubscription(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedUserId || !assignPlanId) return;
    try {
      await adminAssignSubscription(selectedUserId, {
        plan_id: assignPlanId,
        duration_days: assignDurationDays ? parseInt(assignDurationDays, 10) : undefined,
      });
      setAssignPlanId("");
      setAssignDurationDays("");
      await loadUserDetail(selectedUserId);
      await reloadSummaries({ keepSelection: true });
      toast.success("订阅分配成功");
    } catch (cause) {
      toast.error(`分配订阅失败：${(cause as ApiError).message}`);
    }
  }

  async function onUpdateSubscription(subId: string) {
    if (!selectedUserId || !editExpiresAt) return;
    setEditingSubId(subId);
    try {
      await adminUpdateSubscription(selectedUserId, subId, {
        expires_at: new Date(editExpiresAt).toISOString(),
      });
      toast.success("订阅到期时间已更新");
      setEditingSub(null);
      setEditExpiresAt("");
      await loadUserDetail(selectedUserId);
    } catch (cause) {
      toast.error(`更新失败：${(cause as ApiError).message}`);
    } finally {
      setEditingSubId(null);
    }
  }

  async function onConfirmCancelSubscription() {
    if (!selectedUserId || !cancelSubscriptionTarget) return;
    try {
      await adminCancelSubscription(selectedUserId, cancelSubscriptionTarget.id);
      await loadUserDetail(selectedUserId);
      await reloadSummaries({ keepSelection: true });
      toast.success("订阅已取消");
      setCancelSubscriptionTarget(null);
    } catch (cause) {
      toast.error(`取消订阅失败：${(cause as ApiError).message}`);
    }
  }

  async function onDeleteUser() {
    if (!selectedUserId || !detail || !deleteConfirmMatched) return;
    try {
      await deleteUser(selectedUserId);
      toast.success(`用户 ${detail.user.Name} 已删除`);
      setDetail(null);
      setSelectedUserId(null);
      setDeleteConfirmName("");
      setConfirmDeleteOpen(false);
      await reloadSummaries({ keepSelection: false });
    } catch (cause) {
      toast.error(`删除失败：${(cause as ApiError).message}`);
    }
  }

  if (!ready || usersLoading) return <LoadingState title="加载用户管理" />;
  if (error) return <ErrorState title="用户管理加载失败" description={error} />;

  return (
    <div className="flex min-h-0 flex-col gap-3 overflow-auto">
      <form className="flex items-center gap-2" onSubmit={onSearch}>
        <Input
          className="min-w-0 flex-1"
          placeholder="搜索用户名/邮箱/显示名"
          value={queryText}
          onChange={(e) => setQueryText(e.target.value)}
        />
        <Button type="submit" size="sm" className="shrink-0">
          查询
        </Button>
        <Button
          type="button"
          size="sm"
          variant="outline"
          className="shrink-0"
          onClick={() => void reloadSummaries({ keepSelection: true })}
        >
          刷新
        </Button>
        <Button
          type="button"
          size="sm"
          className="shrink-0"
          onClick={() => setCreateModalOpen(true)}
        >
          创建
        </Button>
      </form>

      <div className="flex flex-wrap items-center gap-2">
        <Select
          className="w-auto min-w-[5.5rem]"
          value={statusFilter}
          onChange={(e) => setStatusFilter(e.target.value as UserStatusFilter)}
        >
          <option value="all">全部状态</option>
          <option value="enabled">启用</option>
          <option value="disabled">禁用</option>
        </Select>
        <Select
          className="w-auto min-w-[5.5rem]"
          value={roleFilter}
          onChange={(e) => setRoleFilter(e.target.value as "all" | UserRole)}
        >
          <option value="all">全部角色</option>
          <option value="Admin">Admin</option>
          <option value="Viewer">Viewer</option>
        </Select>
        <Select
          className="w-auto min-w-[5.5rem]"
          value={sortBy}
          onChange={(e) => setSortBy(e.target.value as typeof sortBy)}
        >
          <option value="id">按 ID</option>
          <option value="email">按邮箱</option>
          <option value="online_devices">按在线设备</option>
          <option value="status">按状态</option>
          <option value="subscription">按订阅</option>
          <option value="role">按权限组</option>
          <option value="used_bytes">按已用流量</option>
        </Select>
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="shrink-0"
          onClick={() => setSortDir((p) => (p === "asc" ? "desc" : "asc"))}
        >
          {sortDir === "asc" ? "↑ 升序" : "↓ 降序"}
        </Button>
        {hasSelection && (
          <>
            <span className="text-muted-foreground text-xs">已选 {selectedIds.length}</span>
            <Button
              size="sm"
              variant="secondary"
              className="shrink-0"
              onClick={() => void onBatchStatus(true)}
            >
              批量启用
            </Button>
            <Button
              size="sm"
              variant="secondary"
              className="shrink-0"
              onClick={() => void onBatchStatus(false)}
            >
              批量禁用
            </Button>
          </>
        )}
      </div>

      <div className="overflow-x-auto">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-8">选</TableHead>
              <TableHead>用户</TableHead>
              <TableHead>状态</TableHead>
              <TableHead>已用</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {items.map((item) => (
              <TableRow
                key={item.id}
                className={cn(
                  "cursor-pointer",
                  selectedUserId === item.id ? "bg-primary/10 hover:bg-primary/10" : undefined
                )}
                onClick={() => void onSelectUser(item)}
              >
                <TableCell onClick={(e) => e.stopPropagation()}>
                  <Checkbox
                    checked={selectedIds.includes(item.id)}
                    onCheckedChange={(checked) => {
                      if (checked) setSelectedIds((p) => [...p, item.id]);
                      else setSelectedIds((p) => p.filter((id) => id !== item.id));
                    }}
                  />
                </TableCell>
                <TableCell>
                  <div className="truncate text-sm">{item.email || item.id.slice(0, 8)}</div>
                  <div className="text-muted-foreground flex items-center gap-1 text-xs">
                    <Badge variant="outline" className="text-xs">
                      {item.role}
                    </Badge>
                    <span className="truncate">{item.subscription_name || "-"}</span>
                  </div>
                </TableCell>
                <TableCell>
                  {item.is_disabled ? (
                    <Badge variant="danger">禁用</Badge>
                  ) : (
                    <Badge variant="success">正常</Badge>
                  )}
                </TableCell>
                <TableCell className="text-sm whitespace-nowrap">
                  {formatBytes(item.used_bytes)}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      <div className="flex items-center justify-between text-sm">
        <span>
          共 {summary?.total ?? 0} 条，第 {page}/{totalPages} 页
        </span>
        <div className="flex gap-1">
          <Button
            variant="outline"
            size="sm"
            disabled={page <= 1}
            onClick={() => void changePage(page - 1)}
          >
            上一页
          </Button>
          <Button
            variant="outline"
            size="sm"
            disabled={page >= totalPages}
            onClick={() => void changePage(page + 1)}
          >
            下一页
          </Button>
        </div>
      </div>

      <Modal
        open={Boolean(selectedUserId && detail && !detailLoading)}
        title={detail?.user.Name ?? ""}
        description={detail?.user.Id}
        onClose={() => {
          setSelectedUserId(null);
          setDetail(null);
        }}
        showHeaderClose
        showFooterClose={false}
        cardClassName="max-w-2xl"
      >
        {detail && (
          <div className="space-y-4">
            <div className="flex items-center gap-2">
              <Badge variant="outline">{detail.user.Policy.Role || "Viewer"}</Badge>
              {detail.user.Policy.IsDisabled ? (
                <Badge variant="danger">禁用</Badge>
              ) : (
                <Badge variant="success">正常</Badge>
              )}
            </div>

            <div className="flex gap-1 overflow-x-auto">
              {visibleDetailTabs.map((tab) => (
                <Button
                  key={tab.id}
                  size="sm"
                  variant={activeTab === tab.id ? "default" : "ghost"}
                  className={tab.id === "danger" ? "text-rose-400" : undefined}
                  onClick={() => setActiveTab(tab.id)}
                >
                  {tab.label}
                </Button>
              ))}
            </div>

            <div>
              {activeTab === "profile" && (
                <form className="space-y-3" onSubmit={onSaveProfile}>
                  <label className="block space-y-1">
                    <span className="text-muted-foreground text-xs">邮箱</span>
                    <Input
                      placeholder="user@example.com"
                      value={email}
                      onChange={(e) => setEmail(e.target.value)}
                    />
                  </label>
                  <label className="block space-y-1">
                    <span className="text-muted-foreground text-xs">显示名</span>
                    <Input value={displayName} onChange={(e) => setDisplayName(e.target.value)} />
                  </label>
                  <label className="block space-y-1">
                    <span className="text-muted-foreground text-xs">备注</span>
                    <Input value={remark} onChange={(e) => setRemark(e.target.value)} />
                  </label>
                  <div className="grid grid-cols-2 gap-3">
                    <label className="block space-y-1">
                      <span className="text-muted-foreground text-xs">角色</span>
                      <Select value={role} onChange={(e) => setRole(e.target.value as UserRole)}>
                        <option value="Admin">Admin</option>
                        <option value="Viewer">Viewer</option>
                      </Select>
                    </label>
                    <label className="block space-y-1">
                      <span className="text-muted-foreground text-xs">状态</span>
                      <Select
                        value={disabled ? "disabled" : "enabled"}
                        onChange={(e) => setDisabled(e.target.value === "disabled")}
                      >
                        <option value="enabled">启用</option>
                        <option value="disabled">禁用</option>
                      </Select>
                    </label>
                  </div>
                  <Button type="submit" size="sm" className="w-full">
                    保存资料
                  </Button>
                </form>
              )}

              {activeTab === "limits" && (
                <form className="space-y-3" onSubmit={onSaveStreamPolicy}>
                  <label className="block space-y-1">
                    <span className="text-muted-foreground text-xs">最大并发流</span>
                    <Input
                      type="number"
                      min="0"
                      placeholder="留空不限"
                      value={maxConcurrentStreams}
                      onChange={(e) => setMaxConcurrentStreams(e.target.value)}
                    />
                  </label>
                  {trafficEnabled ? (
                    <>
                      <label className="block space-y-1">
                        <span className="text-muted-foreground text-xs">流量配额 (GB)</span>
                        <Input
                          type="number"
                          min="0"
                          placeholder="留空不限"
                          value={trafficQuotaGb}
                          onChange={(e) => setTrafficQuotaGb(e.target.value)}
                        />
                      </label>
                      <label className="block space-y-1">
                        <span className="text-muted-foreground text-xs">统计窗口 (天)</span>
                        <Input
                          type="number"
                          min="1"
                          value={trafficWindowDays}
                          onChange={(e) => setTrafficWindowDays(e.target.value)}
                        />
                      </label>
                    </>
                  ) : null}
                  <label className="block space-y-1">
                    <span className="text-muted-foreground text-xs">到期时间</span>
                    <Input
                      type="datetime-local"
                      value={expiresAt}
                      onChange={(e) => setExpiresAt(e.target.value)}
                    />
                  </label>
                  <Button type="submit" size="sm" className="w-full">
                    保存限制
                  </Button>
                </form>
              )}

              {activeTab === "traffic" && trafficEnabled && (
                <div className="space-y-3">
                  <div className="grid grid-cols-2 gap-2">
                    {[
                      { label: "统计窗口", value: `${detail.traffic_usage?.window_days ?? 0} 天` },
                      {
                        label: "已用流量",
                        value: formatBytes(detail.traffic_usage?.used_bytes ?? 0),
                      },
                      {
                        label: "流量配额",
                        value:
                          detail.traffic_usage?.quota_bytes === null ||
                          detail.traffic_usage?.quota_bytes === undefined
                            ? "不限"
                            : formatBytes(detail.traffic_usage.quota_bytes),
                      },
                      {
                        label: "剩余流量",
                        value:
                          detail.traffic_usage?.remaining_bytes === null ||
                          detail.traffic_usage?.remaining_bytes === undefined
                            ? "不限"
                            : formatBytes(detail.traffic_usage.remaining_bytes),
                      },
                    ].map((item) => (
                      <div key={item.label} className="border-border/50 rounded-md border p-2">
                        <p className="text-muted-foreground text-xs">{item.label}</p>
                        <p className="font-medium">{item.value}</p>
                      </div>
                    ))}
                  </div>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => setConfirmResetTrafficOpen(true)}
                  >
                    重置流量
                  </Button>
                </div>
              )}

              {activeTab === "wallet" && billingEnabled && (
                <div className="space-y-4">
                  <p className="text-sm">
                    当前余额: <span className="font-medium">¥{wallet?.balance ?? "0.00"}</span>
                  </p>
                  <form className="space-y-2" onSubmit={onAdjustBalance}>
                    <label className="block space-y-1">
                      <span className="text-muted-foreground text-xs">调整金额</span>
                      <Input
                        type="number"
                        step="0.01"
                        placeholder="可为负数"
                        value={adjustAmount}
                        onChange={(e) => setAdjustAmount(e.target.value)}
                        required
                      />
                    </label>
                    <label className="block space-y-1">
                      <span className="text-muted-foreground text-xs">备注</span>
                      <Input
                        placeholder="调整原因"
                        value={adjustNote}
                        onChange={(e) => setAdjustNote(e.target.value)}
                        required
                      />
                    </label>
                    <Button type="submit" size="sm" className="w-full">
                      调整余额
                    </Button>
                  </form>
                  {ledger.length > 0 && (
                    <div className="space-y-1">
                      <p className="text-sm font-medium">账单流水</p>
                      <Table>
                        <TableHeader>
                          <TableRow>
                            <TableHead>时间</TableHead>
                            <TableHead>类型</TableHead>
                            <TableHead>金额</TableHead>
                            <TableHead>余额</TableHead>
                          </TableRow>
                        </TableHeader>
                        <TableBody>
                          {ledger.map((entry) => (
                            <TableRow key={entry.id}>
                              <TableCell className="text-sm">
                                {formatDate(entry.created_at)}
                              </TableCell>
                              <TableCell>
                                <Badge variant="outline">{entry.entry_type}</Badge>
                              </TableCell>
                              <TableCell
                                className={
                                  parseFloat(entry.amount) >= 0 ? "text-green-400" : "text-red-400"
                                }
                              >
                                {parseFloat(entry.amount) >= 0 ? "+" : ""}
                                {formatPrice(entry.amount)}
                              </TableCell>
                              <TableCell>{formatPrice(entry.balance_after)}</TableCell>
                            </TableRow>
                          ))}
                        </TableBody>
                      </Table>
                    </div>
                  )}
                </div>
              )}
              {activeTab === "subscriptions" && billingEnabled && (
                <div className="space-y-4">
                  <form className="space-y-2" onSubmit={onAssignSubscription}>
                    <Select
                      value={assignPlanId}
                      onChange={(e) => setAssignPlanId(e.target.value)}
                      required
                    >
                      <option value="">选择套餐</option>
                      {plans.map((plan) => (
                        <option key={plan.id} value={plan.id}>
                          {plan.name} / {plan.duration_days} 天
                        </option>
                      ))}
                    </Select>
                    <Input
                      type="number"
                      min="1"
                      placeholder="覆盖时长（天，可选）"
                      value={assignDurationDays}
                      onChange={(e) => setAssignDurationDays(e.target.value)}
                    />
                    <div className="flex justify-end">
                      <Button type="submit" size="sm" disabled={plans.length === 0}>
                        分配订阅
                      </Button>
                    </div>
                  </form>
                  <div className="space-y-1">
                    {(detail.subscriptions?.length ?? 0) === 0 ? (
                      <p className="text-muted-foreground text-sm">暂无订阅记录</p>
                    ) : (
                      detail.subscriptions!.map((sub) => (
                        <div
                          key={sub.id}
                          className="flex items-center justify-between gap-2 text-sm"
                        >
                          <span className="min-w-0 truncate">
                            {sub.plan_name} ({sub.status})
                          </span>
                          {editingSub?.id === sub.id ? (
                            <div className="flex items-center gap-1">
                              <Input
                                type="datetime-local"
                                value={editExpiresAt}
                                onChange={(e) => setEditExpiresAt(e.target.value)}
                                className="w-44"
                              />
                              <Button
                                size="sm"
                                variant="ghost"
                                disabled={editingSubId === sub.id}
                                onClick={() => void onUpdateSubscription(sub.id)}
                              >
                                {editingSubId === sub.id ? "..." : "保存"}
                              </Button>
                              <Button size="sm" variant="ghost" onClick={() => setEditingSub(null)}>
                                取消
                              </Button>
                            </div>
                          ) : (
                            <div className="flex items-center gap-1">
                              <span>{formatDate(sub.expires_at)}</span>
                              {sub.status === "active" && (
                                <>
                                  <Button
                                    size="sm"
                                    variant="ghost"
                                    onClick={() => {
                                      setEditingSub(sub);
                                      setEditExpiresAt(toDateTimeLocalValue(sub.expires_at));
                                    }}
                                  >
                                    改期
                                  </Button>
                                  <Button
                                    size="sm"
                                    variant="ghost"
                                    onClick={() => setCancelSubscriptionTarget(sub)}
                                  >
                                    取消
                                  </Button>
                                </>
                              )}
                            </div>
                          )}
                        </div>
                      ))
                    )}
                  </div>
                </div>
              )}

              {activeTab === "sessions" && (
                <div className="grid grid-cols-2 gap-2">
                  {[
                    { label: "鉴权会话", value: detail.sessions_summary.active_auth_sessions },
                    { label: "播放会话", value: detail.sessions_summary.active_playback_sessions },
                    {
                      label: "最近鉴权",
                      value: formatDate(detail.sessions_summary.last_auth_seen_at),
                    },
                    {
                      label: "最近播放",
                      value: formatDate(detail.sessions_summary.last_playback_seen_at),
                    },
                  ].map((item) => (
                    <div key={item.label} className="border-border/50 rounded-md border p-2">
                      <p className="text-muted-foreground text-xs">{item.label}</p>
                      <p className="font-medium">{item.value}</p>
                    </div>
                  ))}
                </div>
              )}

              {activeTab === "danger" && (
                <div className="space-y-3">
                  <p className="text-sm text-rose-300">删除用户不可恢复，操作会记录审计日志。</p>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => {
                      setDeleteConfirmName("");
                      setConfirmDeleteOpen(true);
                    }}
                  >
                    删除用户
                  </Button>
                </div>
              )}
            </div>
          </div>
        )}
      </Modal>
      <Modal
        open={createModalOpen}
        title="创建用户"
        description="支持直接创建 Admin / Viewer 角色。"
        onClose={() => setCreateModalOpen(false)}
        showHeaderClose
        showFooterClose={false}
      >
        <form className="space-y-3" onSubmit={onCreateUser}>
          <Input
            placeholder="用户名"
            value={newUsername}
            onChange={(e) => setNewUsername(e.target.value)}
            required
          />
          <Input
            placeholder="密码（至少 6 位）"
            value={newPassword}
            onChange={(e) => setNewPassword(e.target.value)}
            type="password"
            required
          />
          <Select value={newRole} onChange={(e) => setNewRole(e.target.value as UserRole)}>
            <option value="Admin">Admin</option>
            <option value="Viewer">Viewer</option>
          </Select>
          <div className="flex justify-end gap-2">
            <Button type="button" variant="secondary" onClick={() => setCreateModalOpen(false)}>
              取消
            </Button>
            <Button type="submit">创建用户</Button>
          </div>
        </form>
      </Modal>

      <Modal
        open={confirmResetTrafficOpen && Boolean(detail)}
        title="确认重置流量"
        description="重置后将清空当前统计窗口内的累计已用流量。"
        onClose={() => setConfirmResetTrafficOpen(false)}
        showHeaderClose
        showFooterClose={false}
      >
        <div className="space-y-4 text-sm">
          <p>确认要重置用户 {detail?.user.Name} 的流量统计吗？</p>
          <div className="flex justify-end gap-2">
            <Button
              type="button"
              variant="secondary"
              onClick={() => setConfirmResetTrafficOpen(false)}
            >
              取消
            </Button>
            <Button type="button" onClick={() => void onResetTraffic()}>
              确认重置
            </Button>
          </div>
        </div>
      </Modal>

      <Modal
        open={Boolean(cancelSubscriptionTarget)}
        title="确认取消订阅"
        description="该操作会立即结束当前订阅权益。"
        onClose={() => setCancelSubscriptionTarget(null)}
        showHeaderClose
        showFooterClose={false}
      >
        <div className="space-y-4 text-sm">
          <p>确认取消订阅 {cancelSubscriptionTarget?.plan_name} 吗？</p>
          <div className="flex justify-end gap-2">
            <Button
              type="button"
              variant="secondary"
              onClick={() => setCancelSubscriptionTarget(null)}
            >
              取消
            </Button>
            <Button type="button" onClick={() => void onConfirmCancelSubscription()}>
              确认取消
            </Button>
          </div>
        </div>
      </Modal>

      <Modal
        open={confirmDeleteOpen && Boolean(detail)}
        title="删除用户"
        description="请输入用户名完成确认，删除后不可恢复。"
        onClose={() => {
          setConfirmDeleteOpen(false);
          setDeleteConfirmName("");
        }}
        showHeaderClose
        showFooterClose={false}
      >
        <div className="space-y-3">
          <p className="text-sm">
            请输入 <span className="font-mono">{expectedDeleteName}</span> 以确认删除。
          </p>
          <Input
            placeholder={`输入 ${expectedDeleteName} 以确认`}
            value={deleteConfirmName}
            onChange={(e) => setDeleteConfirmName(e.target.value)}
          />
          <div className="flex justify-end gap-2">
            <Button
              type="button"
              variant="secondary"
              onClick={() => {
                setConfirmDeleteOpen(false);
                setDeleteConfirmName("");
              }}
            >
              取消
            </Button>
            <Button
              type="button"
              disabled={!deleteConfirmMatched}
              onClick={() => void onDeleteUser()}
            >
              确认删除
            </Button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
