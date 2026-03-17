import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  ChevronLeft,
  ChevronRight,
  ImagePlus,
  Pencil,
  Plus,
  Power,
  PowerOff,
  Save,
  Sparkles,
  Trash2,
  X,
  XCircle,
  FolderOpen,
  Settings2,
} from "lucide-react";

import { ErrorState, LoadingState, EmptyState } from "@/components/domain/DataState";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { SortableProviderList } from "@/components/ui/sortable-provider-list";
import {
  createLibrary,
  deleteLibraryCover,
  listLibraries,
  listLibraryStatus,
  patchLibrary,
  setLibraryEnabled,
  uploadLibraryCover,
  listScraperProviders,
} from "@/lib/api/admin";
import {
  SCRAPER_SCENARIO_KEYS,
  extractLibraryScenarioInputs,
  formatLibraryPolicyInput,
  normalizeLibraryPolicyInput,
  parseLibraryPolicyInput,
  updateLibraryPolicyScenarioInput,
} from "@/lib/admin/scraper-policy";
import { buildItemImageUrl } from "@/lib/api/items";
import type { ApiError } from "@/lib/api/client";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import { toast } from "@/lib/notifications/toast-store";
import type { AdminLibraryStatusItem, LibraryType, ScraperProviderStatus } from "@/lib/types/admin";
import { formatDate } from "@/lib/utils";

const LIBRARY_TYPE_OPTIONS: Array<{ value: LibraryType; label: string }> = [
  { value: "Movie", label: "Movies" },
  { value: "Series", label: "Series" },
  { value: "Mixed", label: "Mixed" },
];

function normalizeLibraryType(value: string | null | undefined): LibraryType {
  const normalized = value?.trim().toLowerCase();
  if (normalized === "movie" || normalized === "movies") {
    return "Movie";
  }
  if (normalized === "series") {
    return "Series";
  }
  return "Mixed";
}

export function AdminLibrariesPanel() {
  const { ready, session } = useAuthSession({ requireAdmin: true });
  const [items, setItems] = useState<AdminLibraryStatusItem[]>([]);
  const [scraperProviders, setScraperProviders] = useState<ScraperProviderStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [coverBusyLibraryId, setCoverBusyLibraryId] = useState<string | null>(null);

  const [showCreateForm, setShowCreateForm] = useState(false);
  const [name, setName] = useState("");
  const [createPaths, setCreatePaths] = useState<string[]>([""]);
  const [createLibraryType, setCreateLibraryType] = useState<LibraryType>("Mixed");

  const [selectedLibraryId, setSelectedLibraryId] = useState<string | null>(null);

  const [libraryTypeDrafts, setLibraryTypeDrafts] = useState<Record<string, LibraryType>>({});
  const [libraryPolicyDrafts, setLibraryPolicyDrafts] = useState<Record<string, string>>({});
  const [libraryTypeBusyId, setLibraryTypeBusyId] = useState<string | null>(null);
  const [libraryPolicyBusyId, setLibraryPolicyBusyId] = useState<string | null>(null);

  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [editPaths, setEditPaths] = useState<string[]>([]);
  const [editBusy, setEditBusy] = useState(false);

  const fileInputRefs = useRef<Record<string, HTMLInputElement | null>>({});

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const [statusResult, libraries, providersResult] = await Promise.all([
        listLibraryStatus(),
        listLibraries().catch(() => []),
        listScraperProviders().catch(() => []),
      ]);

      setScraperProviders(providersResult as ScraperProviderStatus[]);

      const libraryTypeById = new Map(
        libraries.map((library) => [library.id, normalizeLibraryType(library.library_type)])
      );

      const nextItems = statusResult.items.map((item) => ({
        ...item,
        library_type: libraryTypeById.get(item.id) ?? normalizeLibraryType(item.library_type),
      }));

      const nextTypeDrafts = nextItems.reduce<Record<string, LibraryType>>((acc, item) => {
        acc[item.id] = normalizeLibraryType(item.library_type);
        return acc;
      }, {});
      const nextPolicyDrafts = nextItems.reduce<Record<string, string>>((acc, item) => {
        acc[item.id] = formatLibraryPolicyInput(item.scraper_policy);
        return acc;
      }, {});

      setItems(nextItems);
      setLibraryTypeDrafts(nextTypeDrafts);
      setLibraryPolicyDrafts(nextPolicyDrafts);
      setError(null);
    } catch (cause) {
      const apiError = cause as ApiError;
      setError(apiError.message || "加载媒体库失败");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!ready) return;
    void reload();
  }, [ready, reload]);

  async function onCreate(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    const paths = createPaths.map((path) => path.trim()).filter(Boolean);
    if (paths.length === 0) {
      toast.error("请至少添加一个路径");
      return;
    }

    try {
      await createLibrary({
        name: name.trim(),
        paths,
        library_type: createLibraryType,
      });
      setName("");
      setCreatePaths([""]);
      setCreateLibraryType("Mixed");
      setShowCreateForm(false);
      toast.success("媒体库创建成功");
      await reload();
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(`创建失败：${apiError.message}`);
    }
  }

  function onLibraryTypeDraftChange(libraryId: string, nextLibraryType: LibraryType) {
    setLibraryTypeDrafts((prev) => ({
      ...prev,
      [libraryId]: nextLibraryType,
    }));
  }

  async function onSaveLibraryType(libraryId: string) {
    const nextLibraryType = libraryTypeDrafts[libraryId];
    const current = items.find((item) => item.id === libraryId);
    if (
      !current ||
      !nextLibraryType ||
      nextLibraryType === normalizeLibraryType(current.library_type)
    ) {
      return;
    }

    setLibraryTypeBusyId(libraryId);
    try {
      await patchLibrary(libraryId, { library_type: nextLibraryType });
      toast.success("内容类型已更新");
      await reload();
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(`更新内容类型失败：${apiError.message}`);
    } finally {
      setLibraryTypeBusyId(null);
    }
  }

  async function onSaveLibraryPolicy(libraryId: string) {
    const draft = libraryPolicyDrafts[libraryId] ?? "{}";
    const parsedPolicy = parseLibraryPolicyInput(draft);
    if (!parsedPolicy) {
      toast.error("刮削策略 JSON 格式不正确");
      return;
    }

    setLibraryPolicyBusyId(libraryId);
    try {
      await patchLibrary(libraryId, { scraper_policy: parsedPolicy });
      toast.success("库级刮削链路已更新");
      await reload();
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(`更新刮削链路失败：${apiError.message}`);
    } finally {
      setLibraryPolicyBusyId(null);
    }
  }

  function onStartEdit(item: AdminLibraryStatusItem) {
    setEditingId(item.id);
    setEditName(item.name);
    setEditPaths(item.paths.length > 0 ? [...item.paths] : [item.root_path]);
  }

  function onCancelEdit() {
    setEditingId(null);
    setEditName("");
    setEditPaths([]);
  }

  async function onSaveEdit() {
    if (!editingId) return;
    const paths = editPaths.map((path) => path.trim()).filter(Boolean);
    if (!editName.trim()) {
      toast.error("库名称不能为空");
      return;
    }
    if (paths.length === 0) {
      toast.error("请至少保留一个路径");
      return;
    }

    setEditBusy(true);
    try {
      await patchLibrary(editingId, { name: editName.trim(), paths });
      toast.success("媒体库已更新");
      onCancelEdit();
      await reload();
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(`更新失败：${apiError.message}`);
    } finally {
      setEditBusy(false);
    }
  }

  async function onToggle(libraryId: string, enabled: boolean) {
    try {
      await setLibraryEnabled(libraryId, enabled);
      toast.success(`库状态已更新：${enabled ? "启用" : "禁用"}`);
      await reload();
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(`更新失败：${apiError.message}`);
    }
  }

  async function onCoverFileChange(libraryId: string, event: React.ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) return;

    setCoverBusyLibraryId(libraryId);
    try {
      await uploadLibraryCover(libraryId, file, "Primary");
      toast.success("封面上传成功");
      await reload();
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(`封面上传失败：${apiError.message}`);
    } finally {
      setCoverBusyLibraryId(null);
      if (event.target) {
        event.target.value = "";
      }
    }
  }

  function triggerFileInput(libraryId: string) {
    fileInputRefs.current[libraryId]?.click();
  }

  async function onDeleteCover(libraryId: string) {
    setCoverBusyLibraryId(libraryId);
    try {
      await deleteLibraryCover(libraryId, "Primary");
      toast.success("封面已删除");
      await reload();
    } catch (cause) {
      const apiError = cause as ApiError;
      toast.error(`删除封面失败：${apiError.message}`);
    } finally {
      setCoverBusyLibraryId(null);
    }
  }

  const handleSelectLibrary = (id: string | null) => {
    setSelectedLibraryId(id);
    onCancelEdit();
  };

  if (!ready || loading) {
    return <LoadingState title="加载媒体库" />;
  }

  if (error) {
    return <ErrorState title="媒体库页面加载失败" description={error} />;
  }

  const selectedItem = selectedLibraryId ? items.find((i) => i.id === selectedLibraryId) : null;

  if (selectedItem) {
    const item = selectedItem;
    const currentLibraryType = normalizeLibraryType(item.library_type);
    const draftLibraryType = libraryTypeDrafts[item.id] ?? currentLibraryType;
    const typeDirty = draftLibraryType !== currentLibraryType;
    const policyDraft =
      libraryPolicyDrafts[item.id] ?? formatLibraryPolicyInput(item.scraper_policy);
    const parsedPolicy = parseLibraryPolicyInput(policyDraft);
    const quickScenarioInputs = extractLibraryScenarioInputs(parsedPolicy);
    const policyDirty =
      normalizeLibraryPolicyInput(policyDraft) !==
      normalizeLibraryPolicyInput(formatLibraryPolicyInput(item.scraper_policy));
    const isEditingMeta = editingId === item.id;

    return (
      <div className="animate-in fade-in slide-in-from-bottom-4 mx-auto max-w-6xl space-y-6 duration-300">
        <Button
          variant="ghost"
          onClick={() => handleSelectLibrary(null)}
          className="text-muted-foreground hover:text-foreground mb-2 -ml-4"
        >
          <ChevronLeft className="mr-1 h-4 w-4" /> 返回媒体库列表
        </Button>

        <div className="grid gap-6 lg:grid-cols-[300px_1fr]">
          <div className="space-y-6">
            <div className="group bg-card relative aspect-video overflow-hidden rounded-xl border shadow-sm">
              <img
                src={buildItemImageUrl(item.id, session?.token)}
                alt={`${item.name} 封面`}
                className="h-full w-full object-cover transition duration-300 group-hover:scale-105"
                loading="lazy"
                onError={(event) => {
                  event.currentTarget.style.display = "none";
                  event.currentTarget.nextElementSibling?.classList.remove("hidden");
                }}
              />
              <div className="bg-muted/80 text-muted-foreground absolute inset-0 hidden items-center justify-center">
                <ImagePlus className="h-8 w-8" />
              </div>
              <div className="absolute inset-x-0 bottom-0 flex items-center justify-between bg-gradient-to-t from-black/80 via-black/40 to-transparent px-4 py-3">
                <Badge
                  variant={item.enabled ? "success" : "secondary"}
                  className="border-transparent font-normal"
                >
                  {item.enabled ? "已启用" : "已禁用"}
                </Badge>
                <span className="text-[11px] tracking-[0.2em] text-white/80 uppercase">
                  {currentLibraryType}
                </span>
              </div>
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div className="bg-card rounded-xl border p-4 shadow-sm">
                <p className="text-muted-foreground text-xs font-medium">条目总数</p>
                <p className="mt-1 text-2xl font-semibold">{item.item_count}</p>
              </div>
              <div className="bg-card rounded-xl border p-4 shadow-sm">
                <p className="text-muted-foreground text-xs font-medium">最近更新</p>
                <p
                  className="mt-1 truncate text-lg font-semibold"
                  title={
                    item.last_item_updated_at ? formatDate(item.last_item_updated_at) : "从未更新"
                  }
                >
                  {item.last_item_updated_at
                    ? formatDate(item.last_item_updated_at).split(" ")[0]
                    : "从未更新"}
                </p>
              </div>
            </div>

            <div className="bg-card space-y-4 rounded-xl border p-5 shadow-sm">
              <h3 className="text-sm font-semibold">媒体库管理</h3>
              <div className="grid grid-cols-2 gap-2">
                <Button
                  size="sm"
                  variant="secondary"
                  className="w-full justify-start text-xs font-normal"
                  onClick={() => onStartEdit(item)}
                >
                  <Pencil className="mr-2 h-3.5 w-3.5" /> 编辑路径
                </Button>
                <input
                  type="file"
                  accept="image/*"
                  ref={(el) => {
                    fileInputRefs.current[item.id] = el;
                  }}
                  className="hidden"
                  onChange={(event) => void onCoverFileChange(item.id, event)}
                />
                <Button
                  size="sm"
                  variant="secondary"
                  className="w-full justify-start text-xs font-normal"
                  disabled={coverBusyLibraryId === item.id}
                  onClick={() => triggerFileInput(item.id)}
                >
                  <ImagePlus className="mr-2 h-3.5 w-3.5" /> 更换封面
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  className="w-full justify-start text-xs font-normal text-rose-500 hover:bg-rose-500/10 hover:text-rose-600"
                  disabled={coverBusyLibraryId === item.id}
                  onClick={() => void onDeleteCover(item.id)}
                >
                  <Trash2 className="mr-2 h-3.5 w-3.5" /> 删除封面
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  className="w-full justify-start text-xs font-normal"
                  onClick={() => void onToggle(item.id, !item.enabled)}
                >
                  {item.enabled ? (
                    <>
                      <PowerOff className="mr-2 h-3.5 w-3.5 text-amber-500" /> 禁用库
                    </>
                  ) : (
                    <>
                      <Power className="mr-2 h-3.5 w-3.5 text-emerald-500" /> 启用库
                    </>
                  )}
                </Button>
              </div>
            </div>
          </div>

          <div className="space-y-6">
            <div className="bg-card rounded-xl border p-6 shadow-sm">
              <div className="mb-6 flex flex-col gap-1 border-b pb-6">
                {isEditingMeta ? (
                  <Input
                    value={editName}
                    onChange={(event) => setEditName(event.target.value)}
                    className="h-10 max-w-md text-lg font-bold"
                    placeholder="库名称"
                  />
                ) : (
                  <h2 className="text-2xl font-bold tracking-tight">{item.name}</h2>
                )}
                <p className="text-muted-foreground font-mono text-sm">ID: {item.id}</p>
              </div>

              <div className="grid gap-6">
                <div className="grid gap-6 md:grid-cols-2">
                  <div className="space-y-3">
                    <Field label="内容类型">
                      <Select
                        value={draftLibraryType}
                        disabled={libraryTypeBusyId === item.id}
                        onChange={(event) =>
                          onLibraryTypeDraftChange(item.id, event.target.value as LibraryType)
                        }
                      >
                        {LIBRARY_TYPE_OPTIONS.map((option) => (
                          <option key={option.value} value={option.value}>
                            {option.label}
                          </option>
                        ))}
                      </Select>
                    </Field>
                    {typeDirty && (
                      <div className="flex items-center justify-between rounded-md bg-amber-500/10 px-3 py-2 text-xs text-amber-700 dark:text-amber-400">
                        <span>内容类型已修改</span>
                        <Button
                          size="sm"
                          className="h-6 px-2 text-[10px]"
                          disabled={libraryTypeBusyId === item.id}
                          onClick={() => void onSaveLibraryType(item.id)}
                        >
                          保存类型
                        </Button>
                      </div>
                    )}
                  </div>

                  <div className="space-y-3">
                    <Field label="媒体路径">
                      <div className="space-y-2">
                        {(isEditingMeta ? editPaths : item.paths).map((path, index) => (
                          <div key={`${item.id}-path-${index}`} className="flex items-center gap-2">
                            {isEditingMeta ? (
                              <Input
                                value={path}
                                className="h-9 font-mono text-sm"
                                onChange={(event) => {
                                  const next = [...editPaths];
                                  next[index] = event.target.value;
                                  setEditPaths(next);
                                }}
                              />
                            ) : (
                              <div className="bg-muted/30 text-muted-foreground flex h-9 w-full items-center truncate rounded-md border px-3 font-mono text-sm">
                                {path}
                              </div>
                            )}
                            {isEditingMeta && editPaths.length > 1 && (
                              <Button
                                type="button"
                                size="sm"
                                variant="ghost"
                                className="text-muted-foreground h-9 w-9 shrink-0 p-0 hover:text-rose-500"
                                onClick={() =>
                                  setEditPaths(
                                    editPaths.filter((_, itemIndex) => itemIndex !== index)
                                  )
                                }
                              >
                                <X className="h-4 w-4" />
                              </Button>
                            )}
                          </div>
                        ))}
                      </div>
                    </Field>

                    {isEditingMeta && (
                      <div className="flex flex-wrap items-center gap-2 pt-2">
                        <Button
                          type="button"
                          size="sm"
                          variant="outline"
                          className="h-8 border-dashed text-xs"
                          onClick={() => setEditPaths([...editPaths, ""])}
                        >
                          <Plus className="mr-1 h-3 w-3" /> 添加路径
                        </Button>
                        <div className="flex-1"></div>
                        <Button
                          size="sm"
                          variant="ghost"
                          className="h-8"
                          disabled={editBusy}
                          onClick={onCancelEdit}
                        >
                          取消
                        </Button>
                        <Button
                          size="sm"
                          className="h-8"
                          disabled={editBusy}
                          onClick={() => void onSaveEdit()}
                        >
                          <Save className="mr-1 h-3.5 w-3.5" /> 保存路径
                        </Button>
                      </div>
                    )}
                  </div>
                </div>

                <div className="space-y-4 border-t pt-6">
                  <div>
                    <h3 className="flex items-center gap-2 text-base font-semibold">
                      <Settings2 className="h-4 w-4" /> 刮削链路策略
                    </h3>
                    <p className="text-muted-foreground mt-1 text-sm">
                      通过拖拽调整优先级，配置当前媒体库在不同场景下的刮削器回退链路。
                    </p>
                  </div>

                  <div className="grid gap-6 xl:grid-cols-2">
                    {SCRAPER_SCENARIO_KEYS.map((scenarioKey) => {
                      const currentChainRaw = quickScenarioInputs[scenarioKey] ?? "";
                      const currentChain = currentChainRaw
                        .split(",")
                        .map((s) => s.trim())
                        .filter(Boolean);

                      return (
                        <div
                          key={`${item.id}-${scenarioKey}`}
                          className="bg-card/50 rounded-xl border p-4 shadow-sm"
                        >
                          <Field label={`${scenarioKey}`}>
                            <SortableProviderList
                              providers={scraperProviders.map((p) => ({
                                id: p.provider_id,
                                label: p.display_name,
                              }))}
                              activeIds={currentChain.filter((id) =>
                                scraperProviders.some((p) => p.provider_id === id)
                              )}
                              onChange={(activeIds) => {
                                setLibraryPolicyDrafts((current) => ({
                                  ...current,
                                  [item.id]: updateLibraryPolicyScenarioInput(
                                    current[item.id] ?? "{}",
                                    scenarioKey,
                                    activeIds.join(", ")
                                  ),
                                }));
                              }}
                            />
                          </Field>
                        </div>
                      );
                    })}
                  </div>

                  <div className="mt-6 border-t pt-6">
                    <Field label="原始 JSON (高级)">
                      <Textarea
                        rows={6}
                        className="bg-muted/10 resize-y font-mono text-xs"
                        value={policyDraft}
                        onChange={(event) =>
                          setLibraryPolicyDrafts((current) => ({
                            ...current,
                            [item.id]: event.target.value,
                          }))
                        }
                      />
                    </Field>

                    <div className="mt-4 flex items-center justify-between">
                      <div className="text-muted-foreground text-sm">
                        {parsedPolicy ? (
                          policyDirty ? (
                            <span className="text-amber-600 dark:text-amber-400">
                              有未保存的链路修改
                            </span>
                          ) : (
                            "链路配置已同步"
                          )
                        ) : (
                          <span className="text-rose-500">JSON 无法解析，请修正</span>
                        )}
                      </div>
                      <div className="flex gap-2">
                        <Button
                          variant="outline"
                          size="sm"
                          disabled={libraryPolicyBusyId === item.id || !policyDirty}
                          onClick={() =>
                            setLibraryPolicyDrafts((current) => ({
                              ...current,
                              [item.id]: formatLibraryPolicyInput(item.scraper_policy),
                            }))
                          }
                        >
                          放弃修改
                        </Button>
                        <Button
                          size="sm"
                          disabled={!policyDirty || libraryPolicyBusyId === item.id}
                          onClick={() => void onSaveLibraryPolicy(item.id)}
                        >
                          <Sparkles className="mr-2 h-3.5 w-3.5" />
                          {libraryPolicyBusyId === item.id ? "保存中..." : "保存链路策略"}
                        </Button>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-6xl space-y-8">
      <div className="flex flex-col gap-4 border-b pb-6 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">媒体库管理</h1>
          <p className="text-muted-foreground mt-1 text-sm">
            管理系统中的媒体库，设置独立的刮削策略和路径映射。
          </p>
        </div>
        <div className="flex items-center gap-6">
          <div className="mr-4 hidden items-center gap-4 text-sm sm:flex">
            <div className="flex flex-col items-center">
              <span className="text-lg font-semibold">{items.length}</span>
              <span className="text-muted-foreground text-xs">总计</span>
            </div>
            <div className="bg-border h-8 w-px"></div>
            <div className="flex flex-col items-center">
              <span className="text-lg font-semibold text-emerald-600 dark:text-emerald-400">
                {items.filter((item) => item.enabled).length}
              </span>
              <span className="text-muted-foreground text-xs">已启用</span>
            </div>
          </div>
          <Button
            onClick={() => setShowCreateForm(!showCreateForm)}
            className="gap-2 transition-all"
          >
            {showCreateForm ? <XCircle className="h-4 w-4" /> : <Plus className="h-4 w-4" />}
            {showCreateForm ? "取消" : "新建媒体库"}
          </Button>
        </div>
      </div>

      {showCreateForm && (
        <div className="animate-in fade-in slide-in-from-top-4 bg-card rounded-xl border p-6 shadow-sm duration-300">
          <div className="mb-6">
            <h2 className="text-lg font-semibold">新建媒体库</h2>
            <p className="text-muted-foreground text-sm">创建后可以在详情页配置刮削策略。</p>
          </div>
          <form className="space-y-6" onSubmit={onCreate}>
            <div className="grid gap-6 md:grid-cols-2">
              <div className="space-y-2">
                <label className="text-sm font-medium">
                  库名称 <span className="text-rose-500">*</span>
                </label>
                <Input
                  placeholder="例如：Movies"
                  value={name}
                  onChange={(event) => setName(event.target.value)}
                  required
                />
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium">内容类型</label>
                <Select
                  value={createLibraryType}
                  onChange={(event) => setCreateLibraryType(event.target.value as LibraryType)}
                >
                  {LIBRARY_TYPE_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </Select>
              </div>
            </div>

            <div className="space-y-3">
              <label className="text-sm font-medium">
                媒体路径 <span className="text-rose-500">*</span>
              </label>
              <div className="space-y-3">
                {createPaths.map((path, index) => (
                  <div key={index} className="flex items-center gap-2">
                    <Input
                      placeholder="例如：/media/movies"
                      value={path}
                      onChange={(event) => {
                        const next = [...createPaths];
                        next[index] = event.target.value;
                        setCreatePaths(next);
                      }}
                      required={index === 0}
                    />
                    {createPaths.length > 1 && (
                      <Button
                        type="button"
                        size="sm"
                        variant="ghost"
                        className="text-muted-foreground h-9 w-9 shrink-0 p-0 hover:text-rose-500"
                        onClick={() =>
                          setCreatePaths(createPaths.filter((_, itemIndex) => itemIndex !== index))
                        }
                      >
                        <X className="h-4 w-4" />
                      </Button>
                    )}
                  </div>
                ))}
              </div>
              <Button
                type="button"
                size="sm"
                variant="outline"
                className="mt-2 border-dashed"
                onClick={() => setCreatePaths([...createPaths, ""])}
              >
                <Plus className="mr-1 h-3.5 w-3.5" />
                添加另一个路径
              </Button>
            </div>

            <div className="flex justify-end gap-3 pt-2">
              <Button type="button" variant="ghost" onClick={() => setShowCreateForm(false)}>
                取消
              </Button>
              <Button type="submit">创建并前往配置</Button>
            </div>
          </form>
        </div>
      )}

      <div className="bg-card overflow-hidden rounded-xl border shadow-sm">
        {items.length === 0 ? (
          <div className="py-16">
            <EmptyState
              title="暂无媒体库"
              description="您还没有创建任何媒体库，请点击右上方按钮创建。"
            />
          </div>
        ) : (
          <Table>
            <TableHeader className="bg-muted/30">
              <TableRow className="hover:bg-transparent">
                <TableHead className="w-[60px]"></TableHead>
                <TableHead>库名称与类型</TableHead>
                <TableHead>根路径</TableHead>
                <TableHead className="text-right">条目数</TableHead>
                <TableHead className="text-center">状态</TableHead>
                <TableHead className="text-right">更新时间</TableHead>
                <TableHead className="w-[40px]"></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {items.map((item) => {
                const currentLibraryType = normalizeLibraryType(item.library_type);
                return (
                  <TableRow
                    key={item.id}
                    className="hover:bg-muted/50 group h-[72px] cursor-pointer transition-colors"
                    onClick={() => handleSelectLibrary(item.id)}
                  >
                    <TableCell className="p-2 pl-4">
                      <div className="bg-muted relative flex h-10 w-16 items-center justify-center overflow-hidden rounded border">
                        <img
                          src={buildItemImageUrl(item.id, session?.token)}
                          alt=""
                          className="z-10 h-full w-full object-cover"
                          loading="lazy"
                          onError={(event) => {
                            event.currentTarget.style.display = "none";
                          }}
                        />
                        <FolderOpen className="text-muted-foreground/50 absolute z-0 h-5 w-5" />
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="text-foreground font-semibold">{item.name}</div>
                      <div className="text-muted-foreground mt-0.5 text-xs">
                        {currentLibraryType}
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="text-muted-foreground max-w-[200px] truncate font-mono text-xs lg:max-w-[300px]">
                        {item.root_path}{" "}
                        {item.paths.length > 1 && (
                          <span className="bg-muted ml-1 rounded border px-1.5 py-0.5 text-[10px]">
                            +{item.paths.length - 1}
                          </span>
                        )}
                      </div>
                    </TableCell>
                    <TableCell className="text-right tabular-nums">{item.item_count}</TableCell>
                    <TableCell className="text-center">
                      <Badge
                        variant={item.enabled ? "success" : "secondary"}
                        className="mx-auto font-normal"
                      >
                        {item.enabled ? "已启用" : "已禁用"}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-muted-foreground text-right text-sm">
                      {item.last_item_updated_at
                        ? formatDate(item.last_item_updated_at).split(" ")[0]
                        : "-"}
                    </TableCell>
                    <TableCell className="pr-4">
                      <ChevronRight className="text-muted-foreground ml-auto h-4 w-4 opacity-0 transition-opacity group-hover:opacity-100" />
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        )}
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-1.5">
      <label className="text-sm font-medium">{label}</label>
      {children}
    </div>
  );
}
