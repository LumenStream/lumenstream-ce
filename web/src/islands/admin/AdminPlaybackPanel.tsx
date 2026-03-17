import React, { useCallback, useEffect, useMemo, useState } from "react";

import { ErrorState, LoadingState } from "@/components/domain/DataState";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  createLumenBackendNode,
  deletePlaybackDomain,
  deleteLumenBackendNode,
  getSettings,
  getLumenBackendNodeConfig,
  getLumenBackendNodeSchema,
  listPlaybackDomains,
  listLumenBackendNodes,
  patchLumenBackendNode,
  upsertPlaybackDomain,
  upsertSettings,
  upsertLumenBackendNodeConfig,
} from "@/lib/api/admin";
import type { ApiError } from "@/lib/api/client";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import { toast } from "@/lib/notifications/toast-store";
import type {
  PlaybackDomain,
  LumenBackendNode,
  LumenBackendNodeRuntimeSchema,
  LumenBackendRuntimeSchemaField,
  WebAppSettings,
} from "@/lib/types/admin";

function readObject(input: unknown): Record<string, unknown> {
  if (!input || typeof input !== "object" || Array.isArray(input)) {
    return {};
  }
  return input as Record<string, unknown>;
}

function readString(input: unknown, fallback = ""): string {
  if (typeof input === "string") {
    return input;
  }
  return fallback;
}

function readNumber(input: unknown, fallback: number): number {
  if (typeof input === "number" && Number.isFinite(input)) {
    return input;
  }
  if (typeof input === "string") {
    const parsed = Number.parseInt(input, 10);
    if (Number.isFinite(parsed)) {
      return parsed;
    }
  }
  return fallback;
}

function pathValue(source: Record<string, unknown>, key: string): unknown {
  if (!key.trim()) {
    return undefined;
  }
  let current: unknown = source;
  for (const part of key.split(".")) {
    const segment = part.trim();
    if (!segment || !current || typeof current !== "object" || Array.isArray(current)) {
      return undefined;
    }
    current = (current as Record<string, unknown>)[segment];
  }
  return current;
}

function setPathValue(target: Record<string, unknown>, key: string, value: unknown): void {
  const parts = key
    .split(".")
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
  if (parts.length === 0) {
    return;
  }

  let cursor = target;
  parts.forEach((segment, index) => {
    const isLast = index + 1 === parts.length;
    if (isLast) {
      cursor[segment] = value;
      return;
    }
    const next = cursor[segment];
    if (!next || typeof next !== "object" || Array.isArray(next)) {
      cursor[segment] = {};
    }
    cursor = cursor[segment] as Record<string, unknown>;
  });
}

function runtimeFields(
  schema: LumenBackendNodeRuntimeSchema | null
): LumenBackendRuntimeSchemaField[] {
  if (!schema) {
    return [];
  }
  return schema.schema.sections.flatMap((section) => section.fields || []);
}

function buildRuntimeDraft(
  schema: LumenBackendNodeRuntimeSchema,
  config: Record<string, unknown>
): Record<string, unknown> {
  const draft: Record<string, unknown> = {};
  runtimeFields(schema).forEach((field) => {
    const fromConfig = pathValue(config, field.key);
    draft[field.key] = fromConfig ?? field.default ?? (field.type === "boolean" ? false : "");
  });
  return draft;
}

function normalizeFieldValue(field: LumenBackendRuntimeSchemaField, raw: unknown): unknown {
  if (field.type === "boolean") {
    return Boolean(raw);
  }
  if (field.type === "number") {
    if (typeof raw === "number" && Number.isFinite(raw)) {
      return raw;
    }
    const parsed = Number.parseFloat(String(raw ?? ""));
    return Number.isFinite(parsed) ? parsed : 0;
  }
  return String(raw ?? "");
}

function buildRuntimeConfigPayload(
  schema: LumenBackendNodeRuntimeSchema,
  draft: Record<string, unknown>
): Record<string, unknown> {
  const payload: Record<string, unknown> = {};
  runtimeFields(schema).forEach((field) => {
    setPathValue(payload, field.key, normalizeFieldValue(field, draft[field.key]));
  });
  return payload;
}

function shouldDisplayRuntimeField(
  field: LumenBackendRuntimeSchemaField,
  draft: Record<string, unknown>
): boolean {
  const dependsOn = field.depends_on;
  if (!dependsOn) {
    return true;
  }
  const baseValue = draft[dependsOn.key];
  if (dependsOn.equals !== undefined && baseValue !== dependsOn.equals) {
    return false;
  }
  if (dependsOn.not_equals !== undefined && baseValue === dependsOn.not_equals) {
    return false;
  }
  if (dependsOn.in && dependsOn.in.length > 0 && !dependsOn.in.includes(baseValue as never)) {
    return false;
  }
  return true;
}

function fieldInputValue(field: LumenBackendRuntimeSchemaField, value: unknown): string {
  if (field.type === "number") {
    if (typeof value === "number" && Number.isFinite(value)) {
      return String(value);
    }
    return String(value ?? "");
  }
  return String(value ?? "");
}

function safeNodeLabel(node: LumenBackendNode): string {
  return node.name?.trim() ? node.name : node.node_id;
}

export function AdminPlaybackPanel() {
  const { ready } = useAuthSession({ requireAdmin: true });

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [nodes, setNodes] = useState<LumenBackendNode[]>([]);
  const [domains, setDomains] = useState<PlaybackDomain[]>([]);

  const [selectedNodeId, setSelectedNodeId] = useState("");
  const [nodeSchema, setNodeSchema] = useState<LumenBackendNodeRuntimeSchema | null>(null);
  const [nodeConfigVersion, setNodeConfigVersion] = useState(0);
  const [runtimeDraft, setRuntimeDraft] = useState<Record<string, unknown>>({});
  const [nodeRuntimeLoading, setNodeRuntimeLoading] = useState(false);
  const [nodeRuntimeSaving, setNodeRuntimeSaving] = useState(false);

  const [creatingNode, setCreatingNode] = useState(false);
  const [nodeActionId, setNodeActionId] = useState<string | null>(null);
  const [newNodeId, setNewNodeId] = useState("");
  const [newNodeName, setNewNodeName] = useState("");
  const [newNodeEnabled, setNewNodeEnabled] = useState(true);

  const [domainSaving, setDomainSaving] = useState(false);
  const [domainName, setDomainName] = useState("");
  const [domainBaseUrl, setDomainBaseUrl] = useState("");
  const [domainEnabled, setDomainEnabled] = useState(true);
  const [domainPriority, setDomainPriority] = useState("0");
  const [domainIsDefault, setDomainIsDefault] = useState(false);
  const [domainBoundNodeId, setDomainBoundNodeId] = useState("");
  const [domainTrafficMultiplier, setDomainTrafficMultiplier] = useState("1");

  const [globalStreamSaving, setGlobalStreamSaving] = useState(false);
  const [globalStreamEditable, setGlobalStreamEditable] = useState(false);
  const [streamSigningKey, setStreamSigningKey] = useState("");
  const [streamTokenTtlSeconds, setStreamTokenTtlSeconds] = useState("86400");

  const enabledNodes = useMemo(() => nodes.filter((node) => node.enabled), [nodes]);

  function applyGlobalStreamSettings(settings: WebAppSettings) {
    const storage = readObject(settings.storage);
    setStreamSigningKey(readString(storage.lumenbackend_stream_signing_key));
    setStreamTokenTtlSeconds(
      String(readNumber(storage.lumenbackend_stream_token_ttl_seconds, 86400))
    );
  }

  const loadGlobalStreamSettings = useCallback(async () => {
    try {
      const settings = await getSettings(true);
      applyGlobalStreamSettings(settings);
      setGlobalStreamEditable(true);
    } catch (cause) {
      const apiError = cause as ApiError;
      setGlobalStreamEditable(false);
      if (apiError.status === 401 || apiError.status === 403) {
        toast.warning("仅超级管理员可查看和修改 stream signing key。");
      } else {
        toast.error(`加载全局 stream 配置失败：${apiError.message}`);
      }
    }
  }, []);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const [domainItems, nodeItems] = await Promise.all([
        listPlaybackDomains(),
        listLumenBackendNodes(),
        loadGlobalStreamSettings(),
      ]);
      setDomains(domainItems);
      setNodes(nodeItems);
      setSelectedNodeId((prev) => {
        if (!nodeItems.length) {
          return "";
        }
        const found = nodeItems.find((node) => node.node_id === prev);
        return found ? found.node_id : nodeItems[0]!.node_id;
      });
      setError(null);
    } catch (cause) {
      const apiError = cause as ApiError;
      setError(apiError.message || "加载推流配置失败");
    } finally {
      setLoading(false);
    }
  }, [loadGlobalStreamSettings]);

  const loadNodeRuntime = useCallback(async (nodeId: string) => {
    if (!nodeId) {
      setNodeSchema(null);
      setRuntimeDraft({});
      setNodeConfigVersion(0);
      return;
    }

    setNodeRuntimeLoading(true);
    try {
      const configPayload = await getLumenBackendNodeConfig(nodeId, false);
      setNodeConfigVersion(configPayload.version);

      let schemaPayload: LumenBackendNodeRuntimeSchema | null = null;
      try {
        schemaPayload = await getLumenBackendNodeSchema(nodeId);
      } catch (cause) {
        const apiError = cause as ApiError;
        if (apiError.status !== 404) {
          throw cause;
        }
      }

      setNodeSchema(schemaPayload);
      if (schemaPayload) {
        setRuntimeDraft(buildRuntimeDraft(schemaPayload, readObject(configPayload.config)));
      } else {
        setRuntimeDraft({});
      }
    } catch (cause) {
      const apiError = cause as ApiError;
      setNodeSchema(null);
      setRuntimeDraft({});
      toast.error(`节点运行配置加载失败：${apiError.message}`);
    } finally {
      setNodeRuntimeLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!ready) {
      return;
    }
    void reload();
  }, [ready, reload]);

  useEffect(() => {
    if (!ready || !selectedNodeId) {
      return;
    }
    void loadNodeRuntime(selectedNodeId);
  }, [ready, selectedNodeId, loadNodeRuntime]);

  async function onCreateNode() {
    if (!newNodeId.trim()) {
      toast.warning("请填写 Node ID。");
      return;
    }

    setCreatingNode(true);
    try {
      const created = await createLumenBackendNode({
        node_id: newNodeId.trim(),
        node_name: newNodeName.trim(),
        enabled: newNodeEnabled,
      });
      setNewNodeId("");
      setNewNodeName("");
      setNewNodeEnabled(true);
      toast.success(`节点 ${created.node_id} 已创建。`);
      await reload();
      setSelectedNodeId(created.node_id);
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(`新增节点失败：${apiError.message}`);
    } finally {
      setCreatingNode(false);
    }
  }

  async function onToggleNodeEnabled(node: LumenBackendNode) {
    setNodeActionId(node.node_id);
    try {
      const updated = await patchLumenBackendNode(node.node_id, { enabled: !node.enabled });
      toast.success(`${safeNodeLabel(updated)} 已${updated.enabled ? "启用" : "禁用"}。`);
      await reload();
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(`更新节点状态失败：${apiError.message}`);
    } finally {
      setNodeActionId(null);
    }
  }

  async function onDeleteNode(node: LumenBackendNode) {
    if (!window.confirm(`确认删除节点 ${safeNodeLabel(node)}？`)) {
      return;
    }

    setNodeActionId(node.node_id);
    try {
      await deleteLumenBackendNode(node.node_id);
      toast.success(`节点 ${safeNodeLabel(node)} 已删除。`);
      await reload();
      if (selectedNodeId === node.node_id) {
        setSelectedNodeId("");
      }
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(`删除节点失败：${apiError.message}`);
    } finally {
      setNodeActionId(null);
    }
  }

  async function onSaveNodeConfig() {
    if (!selectedNodeId) {
      toast.warning("请先选择节点。");
      return;
    }
    if (!nodeSchema) {
      toast.warning("当前节点尚未上报运行配置 schema，暂不可编辑。");
      return;
    }

    setNodeRuntimeSaving(true);
    try {
      const payload = buildRuntimeConfigPayload(nodeSchema, runtimeDraft);
      const saved = await upsertLumenBackendNodeConfig(selectedNodeId, payload);
      setNodeConfigVersion(saved.version);
      toast.success(`节点配置已保存（version ${saved.version}）。`);
      await loadNodeRuntime(selectedNodeId);
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(`节点配置保存失败：${apiError.message}`);
    } finally {
      setNodeRuntimeSaving(false);
    }
  }

  async function onCreateDomain() {
    if (!domainName.trim() || !domainBaseUrl.trim()) {
      toast.warning("请填写域名名称和 Base URL。");
      return;
    }

    setDomainSaving(true);
    try {
      await upsertPlaybackDomain({
        name: domainName.trim(),
        base_url: domainBaseUrl.trim(),
        enabled: domainEnabled,
        priority: Number.parseInt(domainPriority, 10) || 0,
        is_default: domainIsDefault,
        lumenbackend_node_id: domainBoundNodeId.trim() || null,
        traffic_multiplier: Math.max(0.01, Number.parseFloat(domainTrafficMultiplier) || 1),
      });
      setDomainName("");
      setDomainBaseUrl("");
      setDomainEnabled(true);
      setDomainPriority("0");
      setDomainIsDefault(false);
      setDomainBoundNodeId("");
      setDomainTrafficMultiplier("1");
      toast.success("播放域名已保存。");
      await reload();
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(`保存域名失败：${apiError.message}`);
    } finally {
      setDomainSaving(false);
    }
  }

  async function onSetDefaultDomain(item: PlaybackDomain) {
    setDomainSaving(true);
    try {
      await upsertPlaybackDomain({
        id: item.id,
        name: item.name,
        base_url: item.base_url,
        enabled: item.enabled,
        priority: item.priority,
        is_default: true,
        lumenbackend_node_id: item.lumenbackend_node_id,
        traffic_multiplier: item.traffic_multiplier,
      });
      toast.success(`默认域名已切换为 ${item.name}`);
      await reload();
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(`设置默认域名失败：${apiError.message}`);
    } finally {
      setDomainSaving(false);
    }
  }

  async function onDeleteDomain(item: PlaybackDomain) {
    if (!window.confirm(`确定要删除播放域名「${item.name}」吗？此操作不可撤销。`)) {
      return;
    }

    setDomainSaving(true);
    try {
      await deletePlaybackDomain(item.id);
      toast.success(`播放域名「${item.name}」已删除。`);
      await reload();
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(`删除域名失败：${apiError.message}`);
    } finally {
      setDomainSaving(false);
    }
  }

  async function onSaveGlobalStreamSettings() {
    if (!globalStreamEditable) {
      toast.warning("仅超级管理员可修改。 ");
      return;
    }

    setGlobalStreamSaving(true);
    try {
      const settings = await getSettings(true);
      const storage = readObject(settings.storage);
      const nextSettings: WebAppSettings = {
        ...settings,
        storage: {
          ...storage,
          lumenbackend_stream_signing_key: streamSigningKey.trim(),
          lumenbackend_stream_token_ttl_seconds: Math.max(
            1,
            Number.parseInt(streamTokenTtlSeconds, 10) || 86400
          ),
        },
      };

      const saved = await upsertSettings(nextSettings);
      await loadGlobalStreamSettings();
      toast.success(
        saved.restart_required
          ? "全局 stream 配置已保存，需重启 LumenStream 后生效。"
          : "全局 stream 配置已保存。"
      );
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(`保存全局 stream 配置失败：${apiError.message}`);
    } finally {
      setGlobalStreamSaving(false);
    }
  }

  if (!ready || loading) {
    return <LoadingState title="加载推流配置" />;
  }
  if (error) {
    return <ErrorState title="推流配置加载失败" description={error} />;
  }

  return (
    <div className="space-y-8">
      <section className="border-border/50 border-b pb-6">
        <h3 className="text-sm font-medium">LumenBackend 节点管理</h3>
        <p className="text-muted-foreground mt-1 mb-4 text-xs">
          先在此手动创建节点，再由 lumenbackend 使用相同 node_id 注册并上报 schema。
        </p>

        <div className="mb-4 grid gap-2 md:grid-cols-3">
          <label className="space-y-1">
            <span className="text-muted-foreground text-xs">Node ID</span>
            <input
              className="bg-background border-border w-full rounded-md border px-2 py-1"
              value={newNodeId}
              onChange={(event) => setNewNodeId(event.target.value)}
              placeholder="node-cn-sh-01"
            />
          </label>
          <label className="space-y-1">
            <span className="text-muted-foreground text-xs">节点名称</span>
            <input
              className="bg-background border-border w-full rounded-md border px-2 py-1"
              value={newNodeName}
              onChange={(event) => setNewNodeName(event.target.value)}
              placeholder="上海节点"
            />
          </label>
          <div className="flex items-end gap-3">
            <div className="flex items-center space-x-2 pb-2">
              <Checkbox
                id="newNodeEnabled"
                checked={newNodeEnabled}
                onCheckedChange={(checked) => setNewNodeEnabled(!!checked)}
              />
              <label
                htmlFor="newNodeEnabled"
                className="cursor-pointer text-sm leading-none font-medium"
              >
                启用
              </label>
            </div>
            <Button disabled={creatingNode} onClick={() => void onCreateNode()}>
              {creatingNode ? "创建中..." : "新增节点"}
            </Button>
          </div>
        </div>

        <div className="space-y-2 text-sm">
          {nodes.map((node) => {
            const status = node.last_seen_at ? "在线" : "未上报";
            const active = node.node_id === selectedNodeId;
            return (
              <div
                key={node.node_id}
                className="border-border bg-muted/10 flex flex-wrap items-center justify-between gap-2 rounded-md border px-3 py-2"
              >
                <button
                  type="button"
                  className={`min-w-0 text-left ${active ? "text-foreground" : "text-muted-foreground"}`}
                  onClick={() => setSelectedNodeId(node.node_id)}
                >
                  <p className="font-medium">{safeNodeLabel(node)}</p>
                  <p className="text-xs">node_id: {node.node_id}</p>
                </button>

                <div className="flex items-center gap-2">
                  <Badge variant={node.enabled ? "success" : "outline"}>
                    {node.enabled ? "enabled" : "disabled"}
                  </Badge>
                  <Badge variant={node.last_seen_at ? "secondary" : "outline"}>{status}</Badge>
                  <span className="text-muted-foreground text-xs">
                    version: {node.last_version || "-"}
                  </span>
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={nodeActionId === node.node_id}
                    onClick={() => void onToggleNodeEnabled(node)}
                  >
                    {node.enabled ? "禁用" : "启用"}
                  </Button>
                  <Button
                    size="sm"
                    variant="destructive"
                    disabled={nodeActionId === node.node_id}
                    onClick={() => void onDeleteNode(node)}
                  >
                    删除
                  </Button>
                </div>
              </div>
            );
          })}
          {nodes.length === 0 ? (
            <p className="text-muted-foreground text-xs">暂无节点，请先创建节点。</p>
          ) : null}
        </div>
      </section>

      <section className="border-border/50 border-b pb-6">
        <h3 className="text-sm font-medium">节点运行配置（动态）</h3>
        <p className="text-muted-foreground mt-1 mb-4 text-xs">
          字段定义由 LumenBackend 上报，前端按 schema 动态渲染。stream_route/stream_token 由
          LumenStream 托管。
        </p>

        <div className="space-y-3 text-sm">
          <label className="space-y-1">
            <span className="text-muted-foreground text-xs">选择节点</span>
            <select
              className="bg-background border-border w-full rounded-md border px-2 py-1"
              value={selectedNodeId}
              onChange={(event) => setSelectedNodeId(event.target.value)}
            >
              <option value="">请选择</option>
              {nodes.map((node) => (
                <option key={node.node_id} value={node.node_id}>
                  {safeNodeLabel(node)}
                </option>
              ))}
            </select>
          </label>

          <p className="text-muted-foreground text-xs">配置版本：{nodeConfigVersion}</p>
          {nodeSchema ? (
            <p className="text-muted-foreground text-xs">
              Schema 版本：{nodeSchema.schema_version}（更新时间 {nodeSchema.updated_at}）
            </p>
          ) : (
            <p className="text-muted-foreground text-xs">
              当前节点尚未上报 schema，请先让 lumenbackend 完成 register。
            </p>
          )}

          {nodeSchema
            ? nodeSchema.schema.sections.map((section) => (
                <div
                  key={section.id}
                  className="border-border/60 bg-muted/10 space-y-3 rounded-md border p-3"
                >
                  <div>
                    <p className="font-medium">{section.title}</p>
                    {section.description ? (
                      <p className="text-muted-foreground text-xs">{section.description}</p>
                    ) : null}
                  </div>

                  <div className="grid gap-2 md:grid-cols-2">
                    {section.fields
                      .filter((field) => shouldDisplayRuntimeField(field, runtimeDraft))
                      .map((field) => {
                        const fieldLabel = field.label?.trim() || field.key;
                        const requiredMark = field.required ? " *" : "";
                        const commonClass =
                          "bg-background border-border w-full rounded-md border px-2 py-1";

                        if (field.type === "boolean") {
                          return (
                            <div key={field.key} className="flex items-end md:col-span-2">
                              <div className="flex items-center space-x-2 pb-1">
                                <Checkbox
                                  id={`field-${field.key}`}
                                  checked={Boolean(runtimeDraft[field.key])}
                                  onCheckedChange={(checked) =>
                                    setRuntimeDraft((prev) => ({
                                      ...prev,
                                      [field.key]: !!checked,
                                    }))
                                  }
                                />
                                <label
                                  htmlFor={`field-${field.key}`}
                                  className="cursor-pointer text-sm leading-none font-medium"
                                >
                                  {fieldLabel}
                                  {requiredMark}
                                </label>
                              </div>
                            </div>
                          );
                        }

                        if (field.type === "textarea") {
                          return (
                            <label key={field.key} className="space-y-1 md:col-span-2">
                              <span className="text-muted-foreground text-xs">
                                {fieldLabel}
                                {requiredMark}
                              </span>
                              <textarea
                                className={commonClass}
                                rows={3}
                                placeholder={field.placeholder || ""}
                                value={fieldInputValue(field, runtimeDraft[field.key])}
                                onChange={(event) =>
                                  setRuntimeDraft((prev) => ({
                                    ...prev,
                                    [field.key]: event.target.value,
                                  }))
                                }
                              />
                              {field.help ? (
                                <span className="text-muted-foreground block text-xs">
                                  {field.help}
                                </span>
                              ) : null}
                            </label>
                          );
                        }

                        if (field.type === "select") {
                          return (
                            <label key={field.key} className="space-y-1">
                              <span className="text-muted-foreground text-xs">
                                {fieldLabel}
                                {requiredMark}
                              </span>
                              <select
                                className={commonClass}
                                value={fieldInputValue(field, runtimeDraft[field.key])}
                                onChange={(event) =>
                                  setRuntimeDraft((prev) => ({
                                    ...prev,
                                    [field.key]: event.target.value,
                                  }))
                                }
                              >
                                <option value="">请选择</option>
                                {(field.options || []).map((option) => (
                                  <option key={option.value} value={option.value}>
                                    {option.label || option.value}
                                  </option>
                                ))}
                              </select>
                              {field.help ? (
                                <span className="text-muted-foreground block text-xs">
                                  {field.help}
                                </span>
                              ) : null}
                            </label>
                          );
                        }

                        return (
                          <label key={field.key} className="space-y-1">
                            <span className="text-muted-foreground text-xs">
                              {fieldLabel}
                              {requiredMark}
                            </span>
                            <input
                              className={commonClass}
                              type={
                                field.type === "number"
                                  ? "number"
                                  : field.type === "password"
                                    ? "password"
                                    : "text"
                              }
                              placeholder={field.placeholder || ""}
                              value={fieldInputValue(field, runtimeDraft[field.key])}
                              onChange={(event) =>
                                setRuntimeDraft((prev) => ({
                                  ...prev,
                                  [field.key]:
                                    field.type === "number"
                                      ? event.target.value
                                      : event.target.value,
                                }))
                              }
                            />
                            {field.help ? (
                              <span className="text-muted-foreground block text-xs">
                                {field.help}
                              </span>
                            ) : null}
                          </label>
                        );
                      })}
                  </div>
                </div>
              ))
            : null}

          <Button
            disabled={!selectedNodeId || !nodeSchema || nodeRuntimeLoading || nodeRuntimeSaving}
            onClick={() => void onSaveNodeConfig()}
          >
            {nodeRuntimeLoading ? "加载中..." : nodeRuntimeSaving ? "保存中..." : "保存节点配置"}
          </Button>
        </div>
      </section>

      <section className="border-border/50 border-b pb-6">
        <h3 className="text-sm font-medium">播放域名管理</h3>
        <p className="text-muted-foreground mt-1 mb-4 text-xs">
          LumenStream 将按用户选择的域名返回 302 跳转地址，可绑定到已启用节点。
        </p>

        <div className="space-y-3 text-sm">
          <div className="grid gap-2 md:grid-cols-2">
            <label className="space-y-1">
              <span className="text-muted-foreground text-xs">域名名称</span>
              <input
                className="bg-background border-border w-full rounded-md border px-2 py-1"
                value={domainName}
                onChange={(event) => setDomainName(event.target.value)}
                placeholder="如：A 域名"
              />
            </label>
            <label className="space-y-1">
              <span className="text-muted-foreground text-xs">Base URL</span>
              <input
                className="bg-background border-border w-full rounded-md border px-2 py-1"
                value={domainBaseUrl}
                onChange={(event) => setDomainBaseUrl(event.target.value)}
                placeholder="https://lumenbackend-a.example.com"
              />
            </label>
            <label className="space-y-1">
              <span className="text-muted-foreground text-xs">优先级</span>
              <input
                className="bg-background border-border w-full rounded-md border px-2 py-1"
                value={domainPriority}
                onChange={(event) => setDomainPriority(event.target.value)}
              />
            </label>
            <label className="space-y-1">
              <span className="text-muted-foreground text-xs">绑定 LumenBackend 节点</span>
              <select
                className="bg-background border-border w-full rounded-md border px-2 py-1"
                value={domainBoundNodeId}
                onChange={(event) => setDomainBoundNodeId(event.target.value)}
              >
                <option value="">不绑定（默认倍率）</option>
                {enabledNodes.map((node) => (
                  <option key={node.node_id} value={node.node_id}>
                    {safeNodeLabel(node)}
                  </option>
                ))}
              </select>
            </label>
            <label className="space-y-1">
              <span className="text-muted-foreground text-xs">流量倍率</span>
              <input
                className="bg-background border-border w-full rounded-md border px-2 py-1"
                value={domainTrafficMultiplier}
                onChange={(event) => setDomainTrafficMultiplier(event.target.value)}
                placeholder="1"
              />
            </label>
            <div className="flex items-end gap-4">
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="domainEnabled"
                  checked={domainEnabled}
                  onCheckedChange={(checked) => setDomainEnabled(!!checked)}
                />
                <label
                  htmlFor="domainEnabled"
                  className="cursor-pointer text-sm leading-none font-medium"
                >
                  启用
                </label>
              </div>
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="domainIsDefault"
                  checked={domainIsDefault}
                  onCheckedChange={(checked) => setDomainIsDefault(!!checked)}
                />
                <label
                  htmlFor="domainIsDefault"
                  className="cursor-pointer text-sm leading-none font-medium"
                >
                  设为默认
                </label>
              </div>
            </div>
          </div>

          <Button disabled={domainSaving} onClick={() => void onCreateDomain()}>
            {domainSaving ? "保存中..." : "新增播放域名"}
          </Button>

          <div className="space-y-2">
            {domains.map((item) => (
              <div
                key={item.id}
                className="border-border bg-muted/10 flex flex-wrap items-center justify-between gap-2 rounded-md border px-3 py-2"
              >
                <div>
                  <p className="font-medium">{item.name}</p>
                  <p className="text-muted-foreground text-xs">
                    {item.base_url} · 倍率 x{item.traffic_multiplier.toFixed(2)}
                    {item.lumenbackend_node_id
                      ? ` · 节点 ${item.lumenbackend_node_id}`
                      : " · 未绑定节点"}
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <Badge variant={item.enabled ? "success" : "outline"}>
                    {item.enabled ? "enabled" : "disabled"}
                  </Badge>
                  {item.is_default ? <Badge variant="secondary">default</Badge> : null}
                  <span className="text-muted-foreground text-xs">priority {item.priority}</span>
                  {!item.is_default ? (
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={domainSaving}
                      onClick={() => void onSetDefaultDomain(item)}
                    >
                      设为默认
                    </Button>
                  ) : null}
                  <Button
                    size="sm"
                    variant="destructive"
                    disabled={domainSaving}
                    onClick={() => void onDeleteDomain(item)}
                  >
                    删除
                  </Button>
                </div>
              </div>
            ))}
            {domains.length === 0 ? (
              <p className="text-muted-foreground text-xs">暂无播放域名，请先创建。</p>
            ) : null}
          </div>
        </div>
      </section>

      <section>
        <h3 className="text-sm font-medium">全局推流鉴权</h3>
        <p className="text-muted-foreground mt-1 mb-4 text-xs">
          节点运行时 stream_route / stream_token 由 LumenStream 全局下发，节点侧不可单独覆盖。
        </p>
        <div className="space-y-3 text-sm">
          <label className="space-y-1">
            <span className="text-muted-foreground text-xs">Stream Signing Key</span>
            <input
              className="bg-background border-border w-full rounded-md border px-2 py-1"
              value={streamSigningKey}
              onChange={(event) => setStreamSigningKey(event.target.value)}
              placeholder="留空表示关闭 stream token"
              disabled={!globalStreamEditable}
            />
          </label>
          <label className="space-y-1">
            <span className="text-muted-foreground text-xs">Token TTL (seconds)</span>
            <input
              className="bg-background border-border w-full rounded-md border px-2 py-1"
              value={streamTokenTtlSeconds}
              onChange={(event) => setStreamTokenTtlSeconds(event.target.value)}
              disabled={!globalStreamEditable}
            />
          </label>
          <Button
            disabled={!globalStreamEditable || globalStreamSaving}
            onClick={() => void onSaveGlobalStreamSettings()}
          >
            {globalStreamSaving ? "保存中..." : "保存全局鉴权配置"}
          </Button>
        </div>
      </section>
    </div>
  );
}
