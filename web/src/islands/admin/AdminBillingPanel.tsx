import React, { useEffect, useState } from "react";

import { ErrorState, LoadingState } from "@/components/domain/DataState";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { listPlaybackDomains } from "@/lib/api/admin";
import {
  adminCreatePermissionGroup,
  adminCreatePlan,
  adminGetBillingConfig,
  adminListPermissionGroups,
  adminListPlans,
  adminListRechargeOrders,
  adminUpdatePermissionGroup,
  adminUpdateBillingConfig,
  adminUpdatePlan,
} from "@/lib/api/admin-commercial";
import type { ApiError } from "@/lib/api/client";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import { toast } from "@/lib/notifications/toast-store";
import type { BillingPermissionGroup, Plan, RechargeOrder } from "@/lib/types/billing";
import type { PlaybackDomain } from "@/lib/types/admin";

type TabId = "plans" | "permission_groups" | "orders" | "config";

function formatPrice(price: string): string {
  const amount = parseFloat(price);
  return `¥${amount.toFixed(2)}`;
}

function formatTrafficBytes(bytes: number): string {
  const gb = bytes / (1024 * 1024 * 1024);
  return `${gb.toFixed(0)} GB`;
}

function formatDate(iso: string | null | undefined): string {
  if (!iso) return "-";
  return new Date(iso).toLocaleString("zh-CN");
}

function toPlanUpdatePayload(plan: Plan, enabled: boolean) {
  return {
    code: plan.code,
    name: plan.name,
    price: plan.price,
    duration_days: plan.duration_days,
    traffic_quota_bytes: plan.traffic_quota_bytes,
    traffic_window_days: plan.traffic_window_days,
    permission_group_id: plan.permission_group_id ?? null,
    enabled,
  };
}

export function AdminBillingPanel() {
  const { ready } = useAuthSession({ requireAdmin: true });
  const [activeTab, setActiveTab] = useState<TabId>("plans");

  if (!ready) {
    return <LoadingState title="加载计费管理" />;
  }

  return (
    <div className="space-y-4">
      <div className="border-border flex gap-2 border-b pb-2">
        <Button
          variant={activeTab === "plans" ? "default" : "ghost"}
          size="sm"
          onClick={() => setActiveTab("plans")}
        >
          套餐管理
        </Button>
        <Button
          variant={activeTab === "permission_groups" ? "default" : "ghost"}
          size="sm"
          onClick={() => setActiveTab("permission_groups")}
        >
          权限组
        </Button>
        <Button
          variant={activeTab === "orders" ? "default" : "ghost"}
          size="sm"
          onClick={() => setActiveTab("orders")}
        >
          充值订单
        </Button>
        <Button
          variant={activeTab === "config" ? "default" : "ghost"}
          size="sm"
          onClick={() => setActiveTab("config")}
        >
          支付配置
        </Button>
      </div>

      {activeTab === "plans" && <PlansTab />}
      {activeTab === "permission_groups" && <PermissionGroupsTab />}
      {activeTab === "orders" && <OrdersTab />}
      {activeTab === "config" && <ConfigTab />}
    </div>
  );
}

function PlansTab() {
  const [plans, setPlans] = useState<Plan[]>([]);
  const [permissionGroups, setPermissionGroups] = useState<BillingPermissionGroup[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [editingPlan, setEditingPlan] = useState<Plan | null>(null);
  const [showCreate, setShowCreate] = useState(false);

  async function reload() {
    setLoading(true);
    try {
      const [plansData, groupsData] = await Promise.all([
        adminListPlans(),
        adminListPermissionGroups(),
      ]);
      setPlans(plansData);
      setPermissionGroups(groupsData);
      setError(null);
    } catch (cause) {
      const apiError = cause as ApiError;
      setError(apiError.message || "加载套餐失败");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void reload();
  }, []);

  async function onDelete(planId: string) {
    if (!confirm("确定要删除此套餐吗？")) return;
    try {
      const plan = plans.find((item) => item.id === planId);
      if (!plan) return;
      await adminUpdatePlan(planId, toPlanUpdatePayload(plan, false));
      toast.success("套餐已禁用");
      await reload();
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(`删除失败：${apiError.message}`);
    }
  }

  async function onToggleActive(plan: Plan) {
    try {
      await adminUpdatePlan(plan.id, toPlanUpdatePayload(plan, !plan.enabled));
      toast.success(`套餐已${plan.enabled ? "下架" : "上架"}`);
      await reload();
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(`操作失败：${apiError.message}`);
    }
  }

  if (loading) return <LoadingState title="加载套餐列表" />;
  if (error) return <ErrorState title="套餐加载失败" description={error} />;

  return (
    <div className="space-y-4">
      {showCreate && (
        <PlanForm
          permissionGroups={permissionGroups}
          onCancel={() => setShowCreate(false)}
          onSuccess={() => {
            setShowCreate(false);
            toast.success("套餐创建成功");
            void reload();
          }}
        />
      )}

      {editingPlan && (
        <PlanForm
          plan={editingPlan}
          permissionGroups={permissionGroups}
          onCancel={() => setEditingPlan(null)}
          onSuccess={() => {
            setEditingPlan(null);
            toast.success("套餐更新成功");
            void reload();
          }}
        />
      )}

      {!showCreate && !editingPlan && (
        <section>
          <h3 className="text-sm font-medium">套餐列表</h3>
          <p className="text-muted-foreground mt-1 mb-4 text-xs">
            管理计费套餐，包括创建、编辑、上下架和删除。
          </p>
          <div className="space-y-3">
            <div className="flex gap-2">
              <Button onClick={() => setShowCreate(true)}>创建套餐</Button>
              <Button variant="outline" onClick={() => void reload()}>
                刷新
              </Button>
            </div>

            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>名称</TableHead>
                  <TableHead>价格</TableHead>
                  <TableHead>时长</TableHead>
                  <TableHead>流量</TableHead>
                  <TableHead>权限组</TableHead>
                  <TableHead>状态</TableHead>
                  <TableHead className="w-40">操作</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {plans.map((plan) => (
                  <TableRow key={plan.id}>
                    <TableCell>
                      <div>
                        <p>{plan.name}</p>
                        <p className="text-muted-foreground text-xs">{plan.code}</p>
                      </div>
                    </TableCell>
                    <TableCell>{formatPrice(plan.price)}</TableCell>
                    <TableCell>{plan.duration_days} 天</TableCell>
                    <TableCell>{formatTrafficBytes(plan.traffic_quota_bytes)}</TableCell>
                    <TableCell>{plan.permission_group_name || "不限域名"}</TableCell>
                    <TableCell>
                      {plan.enabled ? (
                        <Badge variant="success">上架</Badge>
                      ) : (
                        <Badge variant="secondary">下架</Badge>
                      )}
                    </TableCell>
                    <TableCell>
                      <div className="flex gap-1">
                        <Button size="sm" variant="ghost" onClick={() => setEditingPlan(plan)}>
                          编辑
                        </Button>
                        <Button size="sm" variant="ghost" onClick={() => void onToggleActive(plan)}>
                          {plan.enabled ? "下架" : "上架"}
                        </Button>
                        <Button size="sm" variant="ghost" onClick={() => void onDelete(plan.id)}>
                          删除
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </section>
      )}
    </div>
  );
}

function PlanForm({
  plan,
  permissionGroups,
  onCancel,
  onSuccess,
}: {
  plan?: Plan;
  permissionGroups: BillingPermissionGroup[];
  onCancel: () => void;
  onSuccess: () => void;
}) {
  const [name, setName] = useState(plan?.name || "");
  const [code, setCode] = useState(plan?.code || "");
  const [price, setPrice] = useState(plan?.price || "");
  const [durationDays, setDurationDays] = useState(plan?.duration_days?.toString() || "30");
  const [trafficQuotaGb, setTrafficQuotaGb] = useState(
    plan?.traffic_quota_bytes ? (plan.traffic_quota_bytes / (1024 * 1024 * 1024)).toString() : ""
  );
  const [trafficWindowDays, setTrafficWindowDays] = useState(
    plan?.traffic_window_days?.toString() || "30"
  );
  const [permissionGroupId, setPermissionGroupId] = useState(plan?.permission_group_id || "");
  const [submitting, setSubmitting] = useState(false);

  async function onSubmit(event: React.SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);

    try {
      const trafficQuotaBytes = Math.round(parseFloat(trafficQuotaGb) * 1024 * 1024 * 1024);

      if (plan) {
        await adminUpdatePlan(plan.id, {
          name,
          code,
          price,
          duration_days: parseInt(durationDays, 10),
          traffic_quota_bytes: trafficQuotaBytes,
          traffic_window_days: parseInt(trafficWindowDays, 10),
          permission_group_id: permissionGroupId || null,
        });
      } else {
        await adminCreatePlan({
          name,
          code,
          price,
          duration_days: parseInt(durationDays, 10),
          traffic_quota_bytes: trafficQuotaBytes,
          traffic_window_days: parseInt(trafficWindowDays, 10),
          permission_group_id: permissionGroupId || null,
        });
      }
      onSuccess();
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(apiError.message || "操作失败");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <section className="border-border/50 border-b pb-6">
      <h3 className="text-sm font-medium">{plan ? "编辑套餐" : "创建套餐"}</h3>
      <div className="mt-4">
        <form className="space-y-3" onSubmit={onSubmit}>
          <div className="grid gap-3 sm:grid-cols-2">
            <Input
              placeholder="套餐名称"
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
            />
            <Input
              placeholder="套餐代码"
              value={code}
              onChange={(e) => setCode(e.target.value)}
              required
            />
          </div>
          <Input
            placeholder="价格（元）"
            type="number"
            step="0.01"
            value={price}
            onChange={(e) => setPrice(e.target.value)}
            required
          />
          <div className="grid gap-3 sm:grid-cols-3">
            <Input
              placeholder="时长（天）"
              type="number"
              value={durationDays}
              onChange={(e) => setDurationDays(e.target.value)}
              required
            />
            <Input
              placeholder="流量（GB）"
              type="number"
              value={trafficQuotaGb}
              onChange={(e) => setTrafficQuotaGb(e.target.value)}
              required
            />
            <Input
              placeholder="流量周期（天）"
              type="number"
              value={trafficWindowDays}
              onChange={(e) => setTrafficWindowDays(e.target.value)}
              required
            />
          </div>
          <div>
            <label className="text-muted-foreground mb-1 block text-xs">权限组（可选）</label>
            <select
              className="border-input bg-background h-10 w-full rounded-md border px-3 text-sm"
              value={permissionGroupId}
              onChange={(event) => setPermissionGroupId(event.target.value)}
            >
              <option value="">不限域名</option>
              {permissionGroups
                .filter((item) => item.enabled)
                .map((group) => (
                  <option key={group.id} value={group.id}>
                    {group.name} ({group.code})
                  </option>
                ))}
            </select>
          </div>
          <div className="flex gap-2">
            <Button type="submit" disabled={submitting}>
              {submitting ? "提交中..." : plan ? "保存" : "创建"}
            </Button>
            <Button type="button" variant="outline" onClick={onCancel}>
              取消
            </Button>
          </div>
        </form>
      </div>
    </section>
  );
}

function PermissionGroupsTab() {
  const [groups, setGroups] = useState<BillingPermissionGroup[]>([]);
  const [domains, setDomains] = useState<PlaybackDomain[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [editingGroup, setEditingGroup] = useState<BillingPermissionGroup | null>(null);
  const [showCreate, setShowCreate] = useState(false);

  async function reload() {
    setLoading(true);
    try {
      const [groupsData, domainsData] = await Promise.all([
        adminListPermissionGroups(),
        listPlaybackDomains(),
      ]);
      setGroups(groupsData);
      setDomains(domainsData.filter((item) => item.enabled));
      setError(null);
    } catch (cause) {
      const apiError = cause as ApiError;
      setError(apiError.message || "加载权限组失败");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void reload();
  }, []);

  if (loading) return <LoadingState title="加载权限组" />;
  if (error) return <ErrorState title="权限组加载失败" description={error} />;

  return (
    <div className="space-y-4">
      {showCreate && (
        <PermissionGroupForm
          domains={domains}
          onCancel={() => setShowCreate(false)}
          onSuccess={() => {
            setShowCreate(false);
            toast.success("权限组创建成功");
            void reload();
          }}
        />
      )}

      {editingGroup && (
        <PermissionGroupForm
          group={editingGroup}
          domains={domains}
          onCancel={() => setEditingGroup(null)}
          onSuccess={() => {
            setEditingGroup(null);
            toast.success("权限组更新成功");
            void reload();
          }}
        />
      )}

      {!showCreate && !editingGroup && (
        <section>
          <h3 className="text-sm font-medium">权限组列表</h3>
          <p className="text-muted-foreground mt-1 mb-4 text-xs">
            权限组可限制账号可选的播放域名，套餐可绑定一个权限组。
          </p>
          <div className="space-y-3">
            <div className="flex gap-2">
              <Button onClick={() => setShowCreate(true)}>创建权限组</Button>
              <Button variant="outline" onClick={() => void reload()}>
                刷新
              </Button>
            </div>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>名称</TableHead>
                  <TableHead>代码</TableHead>
                  <TableHead>域名数量</TableHead>
                  <TableHead>状态</TableHead>
                  <TableHead className="w-40">操作</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {groups.map((group) => (
                  <TableRow key={group.id}>
                    <TableCell>{group.name}</TableCell>
                    <TableCell>{group.code}</TableCell>
                    <TableCell>{group.domain_ids.length}</TableCell>
                    <TableCell>
                      {group.enabled ? (
                        <Badge variant="success">启用</Badge>
                      ) : (
                        <Badge variant="secondary">禁用</Badge>
                      )}
                    </TableCell>
                    <TableCell>
                      <Button size="sm" variant="ghost" onClick={() => setEditingGroup(group)}>
                        编辑
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </section>
      )}
    </div>
  );
}

function PermissionGroupForm({
  group,
  domains,
  onCancel,
  onSuccess,
}: {
  group?: BillingPermissionGroup;
  domains: PlaybackDomain[];
  onCancel: () => void;
  onSuccess: () => void;
}) {
  const [name, setName] = useState(group?.name || "");
  const [code, setCode] = useState(group?.code || "");
  const [enabled, setEnabled] = useState(group?.enabled ?? true);
  const [domainIds, setDomainIds] = useState<string[]>(group?.domain_ids || []);
  const [submitting, setSubmitting] = useState(false);

  function toggleDomainId(domainId: string) {
    setDomainIds((prev) => {
      if (prev.includes(domainId)) {
        return prev.filter((item) => item !== domainId);
      }
      return [...prev, domainId];
    });
  }

  async function onSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (domainIds.length === 0) {
      toast.error("请至少选择一个播放域名");
      return;
    }

    setSubmitting(true);
    try {
      if (group) {
        await adminUpdatePermissionGroup(group.id, {
          name,
          code,
          enabled,
          domain_ids: domainIds,
        });
      } else {
        await adminCreatePermissionGroup({
          name,
          code,
          enabled,
          domain_ids: domainIds,
        });
      }
      onSuccess();
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(apiError.message || "操作失败");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <section className="border-border/50 border-b pb-6">
      <h3 className="text-sm font-medium">{group ? "编辑权限组" : "创建权限组"}</h3>
      <div className="mt-4">
        <form className="space-y-3" onSubmit={onSubmit}>
          <div className="grid gap-3 sm:grid-cols-2">
            <Input
              placeholder="权限组名称"
              value={name}
              onChange={(event) => setName(event.target.value)}
              required
            />
            <Input
              placeholder="权限组代码"
              value={code}
              onChange={(event) => setCode(event.target.value)}
              required
            />
          </div>
          <div className="flex items-center space-x-2 pt-1 pb-1">
            <Switch id="groupEnabled" checked={enabled} onCheckedChange={setEnabled} />
            <label
              htmlFor="groupEnabled"
              className="cursor-pointer text-sm leading-none font-medium"
            >
              启用该权限组
            </label>
          </div>

          <div className="space-y-2">
            <p className="text-muted-foreground text-xs">允许使用的播放域名</p>
            <div className="grid gap-2 sm:grid-cols-2">
              {domains.map((domain) => (
                <label
                  key={domain.id}
                  htmlFor={`domain-${domain.id}`}
                  className="border-border hover:bg-muted/50 flex cursor-pointer items-start gap-2 rounded-md border p-2 transition-colors"
                >
                  <Checkbox
                    id={`domain-${domain.id}`}
                    checked={domainIds.includes(domain.id)}
                    onCheckedChange={() => toggleDomainId(domain.id)}
                    className="mt-0.5"
                  />
                  <span className="text-xs">
                    <span className="font-medium">{domain.name}</span>
                    <br />
                    <span className="text-muted-foreground">{domain.base_url}</span>
                  </span>
                </label>
              ))}
            </div>
          </div>

          <div className="flex gap-2">
            <Button type="submit" disabled={submitting}>
              {submitting ? "提交中..." : group ? "保存" : "创建"}
            </Button>
            <Button type="button" variant="outline" onClick={onCancel}>
              取消
            </Button>
          </div>
        </form>
      </div>
    </section>
  );
}

function OrdersTab() {
  const [orders, setOrders] = useState<RechargeOrder[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  async function reload() {
    setLoading(true);
    try {
      const data = await adminListRechargeOrders();
      setOrders(data);
      setError(null);
    } catch (cause) {
      const apiError = cause as ApiError;
      setError(apiError.message || "加载订单失败");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void reload();
  }, []);

  if (loading) return <LoadingState title="加载充值订单" />;
  if (error) return <ErrorState title="订单加载失败" description={error} />;

  return (
    <section>
      <h3 className="text-sm font-medium">充值订单</h3>
      <p className="text-muted-foreground mt-1 mb-4 text-xs">查看所有用户的充值订单记录。</p>
      <div className="space-y-3">
        <Button variant="outline" onClick={() => void reload()}>
          刷新
        </Button>

        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>订单号</TableHead>
              <TableHead>用户 ID</TableHead>
              <TableHead>金额</TableHead>
              <TableHead>状态</TableHead>
              <TableHead>创建时间</TableHead>
              <TableHead>支付时间</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {orders.map((order) => (
              <TableRow key={order.id}>
                <TableCell className="font-mono text-xs">{order.id}</TableCell>
                <TableCell className="font-mono text-xs">{order.user_id}</TableCell>
                <TableCell>{formatPrice(order.amount)}</TableCell>
                <TableCell>
                  {order.status === "paid" && <Badge variant="success">已支付</Badge>}
                  {order.status === "pending" && <Badge variant="outline">待支付</Badge>}
                  {order.status === "expired" && <Badge variant="secondary">已过期</Badge>}
                  {order.status === "cancelled" && <Badge variant="danger">已取消</Badge>}
                </TableCell>
                <TableCell>{formatDate(order.created_at)}</TableCell>
                <TableCell>{formatDate(order.paid_at)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
    </section>
  );
}

function ConfigTab() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const [gatewayUrl, setGatewayUrl] = useState("");
  const [pid, setPid] = useState("");
  const [key, setKey] = useState("");
  const [notifyUrl, setNotifyUrl] = useState("");
  const [returnUrl, setReturnUrl] = useState("");
  const [sitename, setSitename] = useState("");

  const [billingEnabled, setBillingEnabled] = useState(true);
  const [minAmount, setMinAmount] = useState("");
  const [maxAmount, setMaxAmount] = useState("");
  const [expireMinutes, setExpireMinutes] = useState("");
  const [channels, setChannels] = useState("");

  async function reload() {
    setLoading(true);
    setError(null);
    try {
      const data = await adminGetBillingConfig();
      setGatewayUrl(data.epay.gateway_url);
      setPid(data.epay.pid);
      setKey("");
      setNotifyUrl(data.epay.notify_url);
      setReturnUrl(data.epay.return_url);
      setSitename(data.epay.sitename);
      setBillingEnabled(data.billing.enabled);
      setMinAmount(data.billing.min_recharge_amount);
      setMaxAmount(data.billing.max_recharge_amount);
      setExpireMinutes(data.billing.order_expire_minutes.toString());
      setChannels(data.billing.channels.join(", "));
    } catch (cause) {
      const apiError = cause as ApiError;
      setError(apiError.message || "加载配置失败");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void reload();
  }, []);

  async function onSubmit(event: React.SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);

    try {
      const epayUpdate: Record<string, string> = {
        gateway_url: gatewayUrl,
        pid,
        notify_url: notifyUrl,
        return_url: returnUrl,
        sitename,
      };
      if (key.trim()) {
        epayUpdate.key = key;
      }

      const channelList = channels
        .split(",")
        .map((ch) => ch.trim())
        .filter((ch) => ch.length > 0);

      await adminUpdateBillingConfig({
        epay: epayUpdate,
        billing: {
          enabled: billingEnabled,
          min_recharge_amount: minAmount,
          max_recharge_amount: maxAmount,
          order_expire_minutes: parseInt(expireMinutes, 10),
          channels: channelList,
        },
      });

      toast.success("配置保存成功");
      setKey("");
      await reload();
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(`保存失败：${apiError.message}`);
    } finally {
      setSubmitting(false);
    }
  }

  if (loading) return <LoadingState title="加载支付配置" />;
  if (error) return <ErrorState title="配置加载失败" description={error} />;

  return (
    <form className="space-y-8" onSubmit={onSubmit}>
      <section className="border-border/50 border-b pb-6">
        <h3 className="text-sm font-medium">EPay 网关配置</h3>
        <p className="text-muted-foreground mt-1 mb-4 text-xs">
          配置 EPay 支付网关参数。密钥为敏感信息，仅在需要更新时填写。
        </p>
        <div className="space-y-3">
          <div className="grid gap-3 sm:grid-cols-2">
            <div>
              <label className="text-muted-foreground mb-1 block text-sm">网关地址</label>
              <Input
                placeholder="https://pay.example.com/submit.php"
                value={gatewayUrl}
                onChange={(e) => setGatewayUrl(e.target.value)}
                required
              />
            </div>
            <div>
              <label className="text-muted-foreground mb-1 block text-sm">商户 ID (PID)</label>
              <Input
                placeholder="1001"
                value={pid}
                onChange={(e) => setPid(e.target.value)}
                required
              />
            </div>
          </div>
          <div>
            <label className="text-muted-foreground mb-1 block text-sm">商户密钥 (Key)</label>
            <Input
              type="password"
              placeholder="留空表示不更新"
              value={key}
              onChange={(e) => setKey(e.target.value)}
              autoComplete="new-password"
            />
            <p className="text-muted-foreground mt-1 text-xs">
              密钥为只写字段，不会显示当前值。仅在需要更新时填写。
            </p>
          </div>
          <div className="grid gap-3 sm:grid-cols-2">
            <div>
              <label className="text-muted-foreground mb-1 block text-sm">异步通知地址</label>
              <Input
                placeholder="https://api.example.com/billing/notify"
                value={notifyUrl}
                onChange={(e) => setNotifyUrl(e.target.value)}
                required
              />
            </div>
            <div>
              <label className="text-muted-foreground mb-1 block text-sm">同步跳转地址</label>
              <Input
                placeholder="https://app.example.com/billing/return"
                value={returnUrl}
                onChange={(e) => setReturnUrl(e.target.value)}
                required
              />
            </div>
          </div>
          <div>
            <label className="text-muted-foreground mb-1 block text-sm">站点名称</label>
            <Input
              placeholder="LumenStream"
              value={sitename}
              onChange={(e) => setSitename(e.target.value)}
              required
            />
          </div>
        </div>
      </section>

      <section className="pb-6">
        <h3 className="text-sm font-medium">计费设置</h3>
        <p className="text-muted-foreground mt-1 mb-4 text-xs">
          配置充值金额限制、订单过期时间和支付渠道。
        </p>
        <div className="space-y-3">
          <div className="flex items-center gap-2">
            <Switch
              id="billing-enabled"
              checked={billingEnabled}
              onCheckedChange={setBillingEnabled}
            />
            <label htmlFor="billing-enabled" className="text-sm">
              启用计费系统
            </label>
          </div>
          <div className="grid gap-3 sm:grid-cols-3">
            <div>
              <label className="text-muted-foreground mb-1 block text-sm">最小充值金额（元）</label>
              <Input
                type="number"
                step="0.01"
                placeholder="10.00"
                value={minAmount}
                onChange={(e) => setMinAmount(e.target.value)}
                required
              />
            </div>
            <div>
              <label className="text-muted-foreground mb-1 block text-sm">最大充值金额（元）</label>
              <Input
                type="number"
                step="0.01"
                placeholder="1000.00"
                value={maxAmount}
                onChange={(e) => setMaxAmount(e.target.value)}
                required
              />
            </div>
            <div>
              <label className="text-muted-foreground mb-1 block text-sm">
                订单过期时间（分钟）
              </label>
              <Input
                type="number"
                placeholder="30"
                value={expireMinutes}
                onChange={(e) => setExpireMinutes(e.target.value)}
                required
              />
            </div>
          </div>
          <div>
            <label className="text-muted-foreground mb-1 block text-sm">支付渠道</label>
            <Input
              placeholder="alipay, wxpay"
              value={channels}
              onChange={(e) => setChannels(e.target.value)}
              required
            />
            <p className="text-muted-foreground mt-1 text-xs">
              多个渠道用逗号分隔，如：alipay, wxpay
            </p>
          </div>
        </div>
      </section>

      <div className="flex items-center gap-3">
        <Button type="submit" disabled={submitting}>
          {submitting ? "保存中..." : "保存配置"}
        </Button>
        <Button type="button" variant="outline" onClick={() => void reload()} disabled={submitting}>
          重置
        </Button>
      </div>
    </form>
  );
}
