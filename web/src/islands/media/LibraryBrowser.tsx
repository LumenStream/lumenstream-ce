import { useEffect, useMemo, useState } from "react";

import { AddToPlaylistModal } from "@/components/domain/AddToPlaylistModal";
import { getRouteParam } from "@/lib/hooks/use-route-param";
import { formatEpisodeCode } from "@/lib/media/episode-label";
import { EmptyState, ErrorState, LoadingState } from "@/components/domain/DataState";
import { AdminItemModal } from "@/components/domain/AdminItemModal";
import { PosterItemCard } from "@/components/domain/PosterItemCard";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  addFavoriteItem,
  buildItemBackdropUrl,
  buildItemImageUrl,
  getShowEpisodes,
  getShowSeasons,
  getUserItem,
  getUserItems,
  removeFavoriteItem,
} from "@/lib/api/items";
import type { ApiError } from "@/lib/api/client";
import { canAccessAdmin } from "@/lib/auth/token";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import { resolveMediaItemHref } from "@/lib/media/item-href";
import { toast } from "@/lib/notifications/toast-store";
import type { BaseItem, QueryResult, Season } from "@/lib/types/jellyfin";

interface LibraryBrowserProps {
  parentId?: string;
}

type BrowserMode = "detecting" | "library" | "series";
type EpisodeSortOrder = "asc" | "desc";

const PAGE_SIZE = 24;

function seasonSortValue(season: Season): number {
  if (typeof season.IndexNumber === "number") {
    return season.IndexNumber;
  }

  return Number.MAX_SAFE_INTEGER;
}

function episodeSortValue(episode: BaseItem): number {
  if (typeof episode.IndexNumber === "number") {
    return episode.IndexNumber;
  }

  return Number.MAX_SAFE_INTEGER;
}

function formatEpisodeRuntime(runtimeTicks?: number | null): string {
  if (!runtimeTicks) {
    return "未知时长";
  }

  const minutes = Math.max(1, Math.floor(runtimeTicks / 10_000_000 / 60));
  return `${minutes}min`;
}

export function LibraryBrowser({ parentId: parentIdProp }: LibraryBrowserProps) {
  const parentId = parentIdProp || getRouteParam("library");
  const { session, ready } = useAuthSession();
  const [mode, setMode] = useState<BrowserMode>("detecting");
  const [parentItem, setParentItem] = useState<BaseItem | null>(null);

  const [startIndex, setStartIndex] = useState(0);
  const [loading, setLoading] = useState(true);
  const [result, setResult] = useState<QueryResult<BaseItem> | null>(null);

  const [seasonsLoading, setSeasonsLoading] = useState(false);
  const [episodesLoading, setEpisodesLoading] = useState(false);
  const [seasons, setSeasons] = useState<Season[]>([]);
  const [selectedSeasonId, setSelectedSeasonId] = useState<string | null>(null);
  const [episodes, setEpisodes] = useState<BaseItem[]>([]);
  const [episodeSortOrder, setEpisodeSortOrder] = useState<EpisodeSortOrder>("asc");
  const [seasonEpisodeCounts, setSeasonEpisodeCounts] = useState<Record<string, number>>({});
  const [favoritePending, setFavoritePending] = useState(false);
  const [playlistModalOpen, setPlaylistModalOpen] = useState(false);
  const [adminModalOpen, setAdminModalOpen] = useState(false);
  const [posterFailed, setPosterFailed] = useState(false);
  const [backdropFailed, setBackdropFailed] = useState(false);

  const [error, setError] = useState<string | null>(null);

  const hasNextPage = useMemo(() => {
    if (!result) {
      return false;
    }
    return result.StartIndex + result.Items.length < result.TotalRecordCount;
  }, [result]);

  const selectedSeason = useMemo(
    () => seasons.find((season) => season.Id === selectedSeasonId) || null,
    [seasons, selectedSeasonId]
  );

  const sortedEpisodes = useMemo(() => {
    const sorted = [...episodes].sort((left, right) => {
      const leftIndex = episodeSortValue(left);
      const rightIndex = episodeSortValue(right);
      if (leftIndex !== rightIndex) {
        return leftIndex - rightIndex;
      }

      return left.Name.localeCompare(right.Name, "zh-Hans-CN");
    });

    if (episodeSortOrder === "desc") {
      sorted.reverse();
    }

    return sorted;
  }, [episodes, episodeSortOrder]);

  useEffect(() => {
    if (!ready || !session) {
      return;
    }

    let cancelled = false;

    setMode("detecting");
    setError(null);
    setParentItem(null);
    setResult(null);
    setStartIndex(0);
    setLoading(true);

    setSeasons([]);
    setSelectedSeasonId(null);
    setEpisodes([]);
    setSeasonEpisodeCounts({});
    setEpisodeSortOrder("asc");
    setFavoritePending(false);
    setPlaylistModalOpen(false);
    setPosterFailed(false);
    setBackdropFailed(false);

    getUserItem(session.user.Id, parentId)
      .then((item) => {
        if (cancelled) {
          return;
        }

        setParentItem(item);
        setMode(item.Type === "Series" ? "series" : "library");
      })
      .catch(() => {
        if (cancelled) {
          return;
        }

        setMode("library");
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [parentId, ready, session]);

  useEffect(() => {
    if (!ready || !session || mode !== "library") {
      return;
    }

    let cancelled = false;
    setLoading(true);

    getUserItems(session.user.Id, {
      parentId,
      excludeItemTypes: "Season,Episode",
      limit: PAGE_SIZE,
      startIndex,
    })
      .then((payload) => {
        if (cancelled) {
          return;
        }
        setResult(payload);
        setError(null);
      })
      .catch((cause) => {
        if (cancelled) {
          return;
        }
        const apiError = cause as ApiError;
        setError(apiError.message || "加载媒体列表失败");
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [mode, parentId, ready, session, startIndex]);

  useEffect(() => {
    if (!ready || !session || mode !== "series") {
      return;
    }

    let cancelled = false;
    setSeasonsLoading(true);

    getShowSeasons(parentId)
      .then(async (payload) => {
        if (cancelled) {
          return;
        }

        const orderedSeasons = [...payload.Items].sort((left, right) => {
          const leftValue = seasonSortValue(left);
          const rightValue = seasonSortValue(right);
          if (leftValue !== rightValue) {
            return leftValue - rightValue;
          }

          return left.Name.localeCompare(right.Name, "zh-Hans-CN");
        });

        setSeasons(orderedSeasons);
        setSelectedSeasonId(orderedSeasons[0]?.Id || null);

        const countEntries = await Promise.all(
          orderedSeasons.map(async (season) => {
            try {
              const result = await getShowEpisodes(parentId, { seasonId: season.Id });
              return [season.Id, result.TotalRecordCount || result.Items.length] as const;
            } catch {
              return [season.Id, 0] as const;
            }
          })
        );

        if (!cancelled) {
          setSeasonEpisodeCounts(Object.fromEntries(countEntries));
        }

        setError(null);
      })
      .catch((cause) => {
        if (cancelled) {
          return;
        }

        const apiError = cause as ApiError;
        setError(apiError.message || "加载季信息失败");
        setSeasons([]);
        setSelectedSeasonId(null);
        setSeasonEpisodeCounts({});
      })
      .finally(() => {
        if (!cancelled) {
          setSeasonsLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [mode, parentId, ready, session]);

  useEffect(() => {
    if (mode !== "series") {
      return;
    }

    if (!selectedSeasonId) {
      setEpisodes([]);
      return;
    }

    let cancelled = false;
    setEpisodesLoading(true);

    getShowEpisodes(parentId, { seasonId: selectedSeasonId })
      .then((payload) => {
        if (cancelled) {
          return;
        }

        setEpisodes(payload.Items);
        setError(null);
      })
      .catch((cause) => {
        if (cancelled) {
          return;
        }

        const apiError = cause as ApiError;
        setError(apiError.message || "加载剧集失败");
        setEpisodes([]);
      })
      .finally(() => {
        if (!cancelled) {
          setEpisodesLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [mode, parentId, selectedSeasonId]);

  if (!ready || mode === "detecting" || loading || (mode === "series" && seasonsLoading)) {
    return <LoadingState title="加载媒体列表" description="正在拉取指定库下的媒体元信息。" />;
  }

  if (error) {
    return <ErrorState title="媒体列表加载失败" description={error} />;
  }

  if (mode === "series") {
    const isFavorite = parentItem?.UserData?.IsFavorite ?? false;
    const isAdmin = canAccessAdmin(session?.user || null);
    const seriesPosterUrl =
      parentItem?.ImagePrimaryUrl || buildItemImageUrl(parentId, session?.token);
    const seriesBackdropUrl = buildItemBackdropUrl(parentId, session?.token, 0);

    async function onToggleSeriesFavorite() {
      if (!session || !parentItem || favoritePending) {
        return;
      }

      const nextFavorite = !(parentItem.UserData?.IsFavorite ?? false);
      setFavoritePending(true);

      setParentItem((prev) => {
        if (!prev) {
          return prev;
        }

        return {
          ...prev,
          UserData: {
            ...(prev.UserData || { Played: false, PlaybackPositionTicks: 0 }),
            IsFavorite: nextFavorite,
          },
        };
      });

      try {
        if (nextFavorite) {
          await addFavoriteItem(session.user.Id, parentItem.Id);
          toast.success("已加入喜欢");
        } else {
          await removeFavoriteItem(session.user.Id, parentItem.Id);
          toast.info("已取消喜欢");
        }
      } catch (cause) {
        setParentItem((prev) => {
          if (!prev) {
            return prev;
          }

          return {
            ...prev,
            UserData: {
              ...(prev.UserData || { Played: false, PlaybackPositionTicks: 0 }),
              IsFavorite: !nextFavorite,
            },
          };
        });
        const apiError = cause as ApiError;
        toast.error(apiError.message || "更新喜欢状态失败");
      } finally {
        setFavoritePending(false);
      }
    }

    return (
      <div className="space-y-5">
        <section
          className="relative overflow-hidden bg-black"
          style={{
            marginLeft: "calc(-50vw + 50%)",
            marginRight: "calc(-50vw + 50%)",
            width: "100vw",
          }}
        >
          {!backdropFailed ? (
            <img
              src={seriesBackdropUrl}
              alt={parentItem?.Name || "Series backdrop"}
              className="absolute inset-0 h-full w-full object-cover"
              onError={() => setBackdropFailed(true)}
            />
          ) : (
            <div className="absolute inset-0 bg-gradient-to-r from-slate-950 via-slate-900 to-slate-800" />
          )}
          <div className="absolute inset-0 bg-gradient-to-r from-black/90 via-black/70 to-black/35" />
          <div className="absolute inset-0 bg-gradient-to-t from-black via-black/60 to-transparent" />
          <div className="from-background pointer-events-none absolute inset-x-0 bottom-0 h-20 bg-gradient-to-t to-transparent" />

          <div className="relative mx-auto flex min-h-[56vh] max-w-7xl items-end gap-6 px-4 pb-10 sm:px-6 md:min-h-[60vh] lg:px-8">
            <div className="hidden w-[150px] shrink-0 overflow-hidden rounded-lg bg-black/45 shadow-2xl md:block">
              {!posterFailed ? (
                <img
                  src={seriesPosterUrl}
                  alt={parentItem?.Name || "Series poster"}
                  className="aspect-[2/3] w-full object-cover"
                  onError={() => setPosterFailed(true)}
                />
              ) : (
                <div className="flex aspect-[2/3] items-center justify-center text-xs text-neutral-300">
                  海报加载失败
                </div>
              )}
            </div>

            <div className="min-w-0 flex-1 space-y-3 text-white">
              <h1 className="text-3xl font-semibold sm:text-4xl md:text-5xl">
                {parentItem?.Name || "剧集目录"}
              </h1>
              {typeof parentItem?.ProductionYear === "number" ? (
                <p className="text-sm text-neutral-300">{parentItem.ProductionYear}</p>
              ) : null}
              {parentItem?.Overview ? (
                <p className="line-clamp-3 max-w-3xl text-sm text-neutral-300">
                  {parentItem.Overview}
                </p>
              ) : null}
              <div className="flex items-center gap-3">
                <button
                  type="button"
                  onClick={() => void onToggleSeriesFavorite()}
                  disabled={favoritePending}
                  className={`flex items-center gap-1.5 rounded-full border border-white/20 bg-white/10 px-3 py-2 text-sm text-white transition-colors disabled:opacity-50 ${
                    isFavorite ? "bg-white/22 hover:bg-white/30" : "hover:bg-white/16"
                  }`}
                >
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 20 20"
                    fill="currentColor"
                    className="h-4 w-4"
                    aria-hidden="true"
                  >
                    <path d="M9.653 16.915l-.005-.003-.019-.01a20.759 20.759 0 01-1.162-.682 22.045 22.045 0 01-2.582-1.9C4.045 12.733 2 10.352 2 7.5a4.5 4.5 0 018-2.828A4.5 4.5 0 0118 7.5c0 2.852-2.044 5.233-3.885 6.82a22.049 22.049 0 01-3.744 2.582l-.019.01-.005.003h-.002a.723.723 0 01-.692 0h-.002z" />
                  </svg>
                  {isFavorite ? "取消喜欢" : "喜欢"}
                </button>
                <button
                  type="button"
                  onClick={() => setPlaylistModalOpen(true)}
                  disabled={!parentItem}
                  className="flex items-center gap-1.5 rounded-full border border-white/20 bg-white/10 px-3 py-2 text-sm text-white transition-colors hover:bg-white/16 disabled:opacity-50"
                >
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 20 20"
                    fill="currentColor"
                    className="h-4 w-4"
                    aria-hidden="true"
                  >
                    <path d="M3 5a1 1 0 011-1h8a1 1 0 110 2H4a1 1 0 01-1-1zm0 5a1 1 0 011-1h8a1 1 0 110 2H4a1 1 0 01-1-1zm1 4a1 1 0 100 2h5a1 1 0 100-2H4zm11-2a1 1 0 10-2 0v1h-1a1 1 0 100 2h1v1a1 1 0 102 0v-1h1a1 1 0 100-2h-1v-1z" />
                  </svg>
                  加入收藏列表
                </button>
                {isAdmin ? (
                  <button
                    type="button"
                    onClick={() => setAdminModalOpen(true)}
                    className="flex h-9 w-9 items-center justify-center rounded-full border border-amber-300/40 bg-amber-300/12 text-amber-100 transition-colors hover:bg-amber-300/20"
                    aria-label="管理员编辑"
                  >
                    <svg
                      xmlns="http://www.w3.org/2000/svg"
                      viewBox="0 0 20 20"
                      fill="currentColor"
                      className="h-4 w-4"
                      aria-hidden="true"
                    >
                      <path
                        fillRule="evenodd"
                        d="M11.49 3.17a1 1 0 011.42 0l3.92 3.92a1 1 0 010 1.42l-7.39 7.4a1 1 0 01-.5.27l-3.2.8a.75.75 0 01-.91-.9l.8-3.2a1 1 0 01.27-.5l7.39-7.4zM13.5 4.58L7 11.08l-.47 1.9 1.9-.48 6.5-6.5-1.43-1.42zm-9 8.67a.75.75 0 011.5 0v2.25h10V6.5a.75.75 0 011.5 0v9A1.5 1.5 0 0116 17H6.25A1.75 1.75 0 014.5 15.25V13.25z"
                        clipRule="evenodd"
                      />
                    </svg>
                  </button>
                ) : null}
              </div>
            </div>
          </div>
        </section>

        {seasons.length === 0 ? (
          <EmptyState title="暂无季信息" description="该剧集没有可展示的季数据。" />
        ) : (
          <>
            <div className="flex items-center justify-between">
              <h2 className="text-lg font-semibold">季目录</h2>
              <Badge variant="glass">{seasons.length} 季</Badge>
            </div>

            <div
              className="scrollbar-hide flex gap-4 overflow-x-auto pb-2"
              style={{ overscrollBehaviorX: "contain" }}
            >
              {seasons.map((season) => {
                const isActive = selectedSeasonId === season.Id;
                const poster =
                  season.ImagePrimaryUrl || buildItemImageUrl(season.Id, session?.token);
                const episodeCount = seasonEpisodeCounts[season.Id];

                return (
                  <button
                    key={season.Id}
                    type="button"
                    onClick={() => setSelectedSeasonId(season.Id)}
                    className={`group w-[160px] shrink-0 overflow-hidden rounded-xl text-left transition ${
                      isActive
                        ? "scale-[1.015] opacity-100 shadow-[0_18px_40px_-18px_rgba(255,255,255,0.35)]"
                        : "opacity-60 hover:opacity-100"
                    }`}
                  >
                    <div className="relative overflow-hidden bg-black/45">
                      <img
                        src={poster}
                        alt={season.Name}
                        className="aspect-[2/3] w-full object-cover"
                        loading="lazy"
                        onError={(event) => {
                          (event.target as HTMLImageElement).style.display = "none";
                        }}
                      />
                      <div className="absolute inset-x-0 bottom-0 h-16 bg-gradient-to-t from-black/75 to-transparent" />
                    </div>
                    <div className="space-y-1 p-3">
                      <p className="text-sm font-medium">{season.Name}</p>
                      <p className="text-muted-foreground text-xs">
                        {typeof episodeCount === "number"
                          ? `${episodeCount} Episodes`
                          : "-- Episodes"}
                      </p>
                    </div>
                  </button>
                );
              })}
            </div>

            <div className="space-y-3">
              <div className="flex items-center justify-between gap-3">
                <h2 className="text-lg font-semibold">{selectedSeason?.Name || "剧集"} Episodes</h2>
                <div className="flex gap-2">
                  <Button
                    size="sm"
                    variant={episodeSortOrder === "asc" ? "immersive" : "glass"}
                    onClick={() => setEpisodeSortOrder("asc")}
                  >
                    正序
                  </Button>
                  <Button
                    size="sm"
                    variant={episodeSortOrder === "desc" ? "immersive" : "glass"}
                    onClick={() => setEpisodeSortOrder("desc")}
                  >
                    倒序
                  </Button>
                </div>
              </div>

              {episodesLoading ? (
                <p className="text-muted-foreground text-sm">加载剧集中...</p>
              ) : sortedEpisodes.length === 0 ? (
                <p className="text-muted-foreground text-sm">当前季暂无剧集信息。</p>
              ) : (
                <div className="flex flex-wrap gap-3">
                  {sortedEpisodes.map((episode) => {
                    const cover =
                      episode.ImagePrimaryUrl || buildItemImageUrl(episode.Id, session?.token);
                    return (
                      <a
                        key={episode.Id}
                        href={resolveMediaItemHref(episode)}
                        className="group w-[220px] overflow-hidden rounded-xl transition"
                      >
                        <div className="relative overflow-hidden bg-neutral-900">
                          <img
                            src={cover}
                            alt={episode.Name}
                            className="aspect-video w-full object-cover transition-transform duration-200 group-hover:scale-105"
                            loading="lazy"
                            onError={(event) => {
                              (event.target as HTMLImageElement).style.display = "none";
                            }}
                          />
                          {episode.UserData?.Played ? (
                            <div className="absolute top-2 right-2">
                              <Badge variant="glass" className="text-[10px]">
                                已看
                              </Badge>
                            </div>
                          ) : null}
                          <div className="absolute inset-x-0 bottom-0 h-20 bg-gradient-to-t from-black/85 to-transparent" />
                        </div>

                        <div className="space-y-1 p-3">
                          <p className="text-sm font-medium">
                            {formatEpisodeCode(episode.ParentIndexNumber, episode.IndexNumber)}
                          </p>
                          <p className="text-muted-foreground line-clamp-1 text-xs">
                            {episode.Name}
                          </p>
                          <p className="text-muted-foreground text-xs">
                            {formatEpisodeRuntime(episode.RunTimeTicks)}
                          </p>
                        </div>
                      </a>
                    );
                  })}
                </div>
              )}
            </div>
          </>
        )}
        {parentItem ? (
          <>
            <AddToPlaylistModal
              open={playlistModalOpen}
              item={parentItem}
              onClose={() => setPlaylistModalOpen(false)}
            />
            <AdminItemModal
              open={adminModalOpen}
              item={parentItem}
              onClose={() => setAdminModalOpen(false)}
              onSuccess={async () => {
                if (session) {
                  const latest = await getUserItem(session.user.Id, parentId);
                  setParentItem(latest);
                }
              }}
              onDeleted={() => {
                window.location.href = "/app";
              }}
            />
          </>
        ) : null}
      </div>
    );
  }

  const items = result?.Items || [];

  return (
    <div className="space-y-5">
      <section className="space-y-3">
        <div>
          <h1 className="text-xl font-semibold">媒体库浏览</h1>
          <p className="text-muted-foreground text-sm">
            ParentId: <span className="font-mono text-xs text-neutral-400">{parentId}</span>
          </p>
        </div>
      </section>

      {items.length === 0 ? (
        <EmptyState title="当前目录暂无媒体项" description="请检查后端扫描/索引状态。" />
      ) : (
        <>
          <div className="flex items-center justify-end">
            <Badge variant="glass">{result?.TotalRecordCount || 0} 项</Badge>
          </div>

          <div className="flex flex-wrap gap-3">
            {items.map((item) => (
              <PosterItemCard
                key={item.Id}
                item={item}
                href={resolveMediaItemHref(item)}
                token={session?.token}
                userId={session?.user.Id}
              />
            ))}
          </div>

          <div className="flex items-center justify-between rounded-full bg-white/5 px-4 py-3 backdrop-blur-sm">
            <p className="text-muted-foreground text-sm">
              显示 {result?.StartIndex || 0} - {(result?.StartIndex || 0) + items.length} /{" "}
              {result?.TotalRecordCount || 0}
            </p>
            <div className="flex gap-2">
              <Button
                variant="glass"
                disabled={startIndex === 0}
                onClick={() => setStartIndex((value) => Math.max(0, value - PAGE_SIZE))}
              >
                上一页
              </Button>
              <Button
                variant="glass"
                disabled={!hasNextPage}
                onClick={() => setStartIndex((value) => value + PAGE_SIZE)}
              >
                下一页
              </Button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
