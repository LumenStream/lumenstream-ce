import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { EmptyState, ErrorState, LoadingState } from "@/components/domain/DataState";
import { PosterItemCard } from "@/components/domain/PosterItemCard";
import { buttonVariants } from "@/components/ui/button";
import {
  buildItemBackdropUrl,
  buildItemImageUrl,
  getItemCounts,
  getRootItemsShared,
  getResumeItems,
  getTopPlayedItems,
  getUserItems,
} from "@/lib/api/items";
import type { ApiError } from "@/lib/api/client";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import { useImageGlow } from "@/lib/hooks/use-image-glow";
import { resolveMediaItemHref } from "@/lib/media/item-href";
import type { BaseItem, ItemCounts, QueryResult, TopPlayedSummary } from "@/lib/types/jellyfin";

const CAROUSEL_INTERVAL_MS = 4_500;

interface DashboardRow {
  root: BaseItem;
  items: BaseItem[];
}

interface DashboardState {
  root?: QueryResult<BaseItem>;
  resume?: QueryResult<BaseItem>;
  counts?: ItemCounts;
  rows: DashboardRow[];
  topPlayed?: TopPlayedSummary | null;
}

interface TopCarouselItem {
  Id: string;
  Name: string;
  Type: string;
  RunTimeTicks?: number | null;
  Bitrate?: number | null;
  ProductionYear?: number | null;
  CommunityRating?: number | null;
  Overview?: string | null;
  ImagePrimaryUrl?: string | null;
  PlayCount?: number;
  UniqueUsers?: number;
}

function formatRuntime(runtimeTicks?: number | null): string {
  if (!runtimeTicks) {
    return "时长待补充";
  }

  const totalMinutes = Math.max(1, Math.floor(runtimeTicks / 10_000_000 / 60));
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;

  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }

  return `${minutes}m`;
}

function toRating(item?: { CommunityRating?: number | null }): string {
  if (!item || typeof item.CommunityRating !== "number") {
    return "--";
  }

  return item.CommunityRating.toFixed(1);
}

function currentDayKey(date = new Date()): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function hashString(input: string): number {
  return [...input].reduce((sum, ch) => sum + ch.charCodeAt(0), 0);
}

function rankScore(item: BaseItem, dayKey: string): number {
  const rating = typeof item.CommunityRating === "number" ? item.CommunityRating : 6.8;
  const resumeBoost = (item.UserData?.PlaybackPositionTicks || 0) > 0 ? 0.6 : 0;
  const dayOffset = (hashString(`${dayKey}:${item.Id}`) % 100) / 100;
  return rating + resumeBoost + dayOffset;
}

function uniqueById(items: BaseItem[]): BaseItem[] {
  const seen = new Set<string>();
  const deduped: BaseItem[] = [];

  items.forEach((item) => {
    if (seen.has(item.Id)) {
      return;
    }

    seen.add(item.Id);
    deduped.push(item);
  });

  return deduped;
}

function itemCover(item: { Id: string; ImagePrimaryUrl?: string | null }, token?: string): string {
  if (item.ImagePrimaryUrl) {
    return item.ImagePrimaryUrl;
  }

  return buildItemImageUrl(item.Id, token);
}

function itemBackdrop(item: { Id: string }, token?: string): string {
  return buildItemBackdropUrl(item.Id, token, 0);
}

export function HomeDashboard() {
  const { session, ready } = useAuthSession();
  const [state, setState] = useState<DashboardState>({ rows: [] });
  const [initialLoading, setInitialLoading] = useState(true);
  const [rowsLoading, setRowsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [snapshotDate, setSnapshotDate] = useState(() => currentDayKey());
  const [activeIndex, setActiveIndex] = useState(0);
  const [failedImages, setFailedImages] = useState<Record<string, true>>({});

  const markImageAsFailed = useCallback((url: string | null | undefined) => {
    if (!url) {
      return;
    }

    setFailedImages((prev) => {
      if (prev[url]) {
        return prev;
      }

      return {
        ...prev,
        [url]: true,
      };
    });
  }, []);

  const resolveCoverUrl = useCallback(
    (item: { Id: string; ImagePrimaryUrl?: string | null }) => {
      const url = itemCover(item, session?.token);
      return failedImages[url] ? null : url;
    },
    [failedImages, session?.token]
  );

  const resolveBackdropUrl = useCallback(
    (item: { Id: string }) => {
      const url = itemBackdrop(item, session?.token);
      return failedImages[url] ? null : url;
    },
    [failedImages, session?.token]
  );

  // Compute featuredImage early so useImageGlow can be called unconditionally (React Hooks rule)
  const topTenItemsForGlow = useMemo(() => {
    if (state.topPlayed?.Items && state.topPlayed.Items.length > 0) {
      return state.topPlayed.Items as TopCarouselItem[];
    }
    // Fallback computation (simplified - full version computed later)
    const items = uniqueById([
      ...(state.resume?.Items || []),
      ...state.rows.flatMap((row) => row.items),
    ])
      .filter((item) => item.Type !== "CollectionFolder")
      .slice(0, 10);
    return items.map(
      (item) =>
        ({
          Id: item.Id,
          Name: item.Name,
          Type: item.Type,
          ImagePrimaryUrl: item.ImagePrimaryUrl,
        }) as TopCarouselItem
    );
  }, [state.topPlayed, state.resume, state.rows]);

  const featuredItemForGlow = topTenItemsForGlow[activeIndex] || topTenItemsForGlow[0] || null;
  const featuredImageForGlow = featuredItemForGlow
    ? (resolveBackdropUrl(featuredItemForGlow) ?? resolveCoverUrl(featuredItemForGlow))
    : null;
  const { glowColor: heroGlowColor } = useImageGlow(featuredImageForGlow);

  useEffect(() => {
    if (!ready || !session) {
      return;
    }

    let cancelled = false;
    setInitialLoading(true);
    setRowsLoading(true);

    const loadRows = async (root: QueryResult<BaseItem>) => {
      const rows = await Promise.all(
        root.Items.map(async (rootItem) => {
          try {
            const payload = await getUserItems(session.user.Id, {
              parentId: rootItem.Id,
              excludeItemTypes: "Season,Episode",
              limit: 20,
              startIndex: 0,
            });

            return {
              root: rootItem,
              items: payload.Items,
            } satisfies DashboardRow;
          } catch {
            return {
              root: rootItem,
              items: [],
            } satisfies DashboardRow;
          }
        })
      );

      if (cancelled) {
        return;
      }

      setState((prev) => ({ ...prev, rows }));
    };

    Promise.all([
      getRootItemsShared(session.user.Id),
      getResumeItems(session.user.Id),
      getItemCounts(),
      getTopPlayedItems({ limit: 10, windowDays: 1, statDate: snapshotDate }).catch(() => null),
    ])
      .then(([root, resume, counts, topPlayed]) => {
        if (cancelled) {
          return;
        }

        setState({ root, resume, counts, rows: [], topPlayed });
        setError(null);
        setInitialLoading(false);

        void loadRows(root)
          .catch(() => {
            if (!cancelled) {
              setState((prev) => ({ ...prev, rows: [] }));
            }
          })
          .finally(() => {
            if (!cancelled) {
              setRowsLoading(false);
            }
          });
      })
      .catch((cause) => {
        if (cancelled) {
          return;
        }

        const apiError = cause as ApiError;
        setError(apiError.message || "加载首页数据失败");
        setInitialLoading(false);
        setRowsLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [ready, session, snapshotDate]);

  useEffect(() => {
    const ticker = window.setInterval(() => {
      const key = currentDayKey();
      setSnapshotDate((prev) => (prev === key ? prev : key));
    }, 60_000);

    return () => {
      window.clearInterval(ticker);
    };
  }, []);

  const resumeItems = useMemo(() => state.resume?.Items || [], [state.resume?.Items]);
  const counts = state.counts;

  const fallbackTop = useMemo(() => {
    return uniqueById([...resumeItems, ...state.rows.flatMap((row) => row.items)])
      .filter((item) => item.Type !== "CollectionFolder")
      .sort((left, right) => rankScore(right, snapshotDate) - rankScore(left, snapshotDate))
      .slice(0, 10)
      .map(
        (item, idx) =>
          ({
            Id: item.Id,
            Name: item.Name,
            Type: item.Type,
            RunTimeTicks: item.RunTimeTicks,
            Bitrate: item.Bitrate,
            ProductionYear: item.ProductionYear,
            CommunityRating: item.CommunityRating,
            Overview: item.Overview,
            ImagePrimaryUrl: item.ImagePrimaryUrl,
            PlayCount: Math.max(1, Math.round(rankScore(item, snapshotDate) * 8) - idx),
            UniqueUsers: 1 + (hashString(`${item.Id}:${snapshotDate}`) % 12),
          }) satisfies TopCarouselItem
      );
  }, [resumeItems, snapshotDate, state.rows]);

  const topTenItems = useMemo(() => {
    if (state.topPlayed?.Items && state.topPlayed.Items.length > 0) {
      return state.topPlayed.Items as TopCarouselItem[];
    }

    return fallbackTop;
  }, [fallbackTop, state.topPlayed]);

  useEffect(() => {
    setActiveIndex(0);
  }, [snapshotDate, topTenItems.length]);

  useEffect(() => {
    if (topTenItems.length <= 1) {
      return;
    }

    const timer = window.setInterval(() => {
      setActiveIndex((prev) => (prev + 1) % topTenItems.length);
    }, CAROUSEL_INTERVAL_MS);

    return () => {
      window.clearInterval(timer);
    };
  }, [topTenItems.length]);

  if (!ready || initialLoading) {
    return <LoadingState title="正在加载首页" description="读取 Top10、Root、Resume 与统计数据" />;
  }

  if (error) {
    return <ErrorState title="首页加载失败" description={error} />;
  }

  const featuredItem = topTenItems[activeIndex] || topTenItems[0] || fallbackTop[0] || null;

  const featuredImage = featuredItem
    ? (resolveBackdropUrl(featuredItem) ?? resolveCoverUrl(featuredItem))
    : null;
  const statDate = state.topPlayed?.StatDate || snapshotDate;
  const featuredLibraryHref = featuredItem
    ? (() => {
        const row = state.rows.find((r) => r.items.some((i) => i.Id === featuredItem.Id));
        return row ? `/app/library/${row.root.Id}` : "/app/search";
      })()
    : "/app/search";

  function moveCarousel(offset: number) {
    if (topTenItems.length === 0) {
      return;
    }

    setActiveIndex((prev) => {
      const next = (prev + offset + topTenItems.length) % topTenItems.length;
      return next;
    });
  }

  return (
    <div className="space-y-16">
      {featuredItem ? (
        <section className="space-y-8">
          <div
            className="relative overflow-hidden bg-black transition-shadow duration-500"
            style={{
              marginLeft: "calc(-50vw + 50%)",
              marginRight: "calc(-50vw + 50%)",
              ...(heroGlowColor
                ? {
                    boxShadow: `0 0 60px 15px ${heroGlowColor}40, 0 0 30px 5px ${heroGlowColor}25`,
                  }
                : {}),
            }}
          >
            {featuredImage ? (
              <img
                src={featuredImage}
                alt={featuredItem.Name}
                className="absolute inset-0 h-full w-full object-cover opacity-[0.58]"
                style={{ animation: "hero-zoom 28s ease-out infinite alternate" }}
                onError={() => markImageAsFailed(featuredImage)}
              />
            ) : (
              <div className="absolute inset-0 bg-gradient-to-br from-sky-950/45 via-slate-950 to-black" />
            )}
            <div className="absolute inset-0 bg-gradient-to-r from-black via-black/70 to-black/20" />
            <div className="absolute inset-0 bg-gradient-to-t from-black via-black/40 to-transparent" />
            <div className="absolute inset-0 bg-[radial-gradient(circle_at_15%_30%,rgba(255,255,255,0.14),transparent_45%)]" />

            <div className="relative mx-auto flex min-h-[76vh] max-w-7xl items-end px-6 py-10 sm:px-8 sm:py-14">
              <div
                className="max-w-3xl space-y-6"
                style={{ animation: "content-reveal 0.75s ease-out both" }}
              >
                <div className="flex items-center gap-2 text-xs tracking-[0.16em] text-white/60 uppercase">
                  <span>Top 10 Daily</span>
                  <span className="text-white/30">•</span>
                  <span>{statDate}</span>
                </div>

                <div className="space-y-3">
                  <h2 className="text-4xl font-bold tracking-tight text-white sm:text-6xl md:text-7xl">
                    {featuredItem.Name}
                  </h2>
                  <p className="line-clamp-2 text-base leading-relaxed text-white/78 sm:text-lg">
                    {featuredItem.Overview || "暂无简介。"}
                  </p>
                </div>

                <div className="flex flex-wrap items-center gap-x-2 gap-y-1 text-xs tracking-wide text-white/80 sm:text-sm">
                  <span>{toRating(featuredItem)}</span>
                  <span className="text-white/40">·</span>
                  <span>{featuredItem.ProductionYear || "----"}</span>
                  <span className="text-white/40">·</span>
                  <span>{formatRuntime(featuredItem.RunTimeTicks)}</span>
                </div>

                <div className="flex flex-wrap items-center gap-3">
                  <a
                    href={resolveMediaItemHref(featuredItem)}
                    className={buttonVariants({ variant: "immersive", size: "lg" })}
                  >
                    查看详情
                  </a>
                  <a
                    href={featuredLibraryHref}
                    className={buttonVariants({
                      variant: "glass",
                      size: "lg",
                      className:
                        "light:border-white/20 light:bg-white/10 light:text-white light:hover:bg-white/15",
                    })}
                  >
                    浏览更多
                  </a>
                  {topTenItems.length > 1 ? (
                    <>
                      <button
                        type="button"
                        className="inline-flex h-10 w-10 items-center justify-center rounded-full border border-white/20 bg-black/35 text-white transition-colors hover:bg-white/20"
                        onClick={() => moveCarousel(-1)}
                        aria-label="上一张海报"
                      >
                        ‹
                      </button>
                      <button
                        type="button"
                        className="inline-flex h-10 w-10 items-center justify-center rounded-full border border-white/20 bg-black/35 text-white transition-colors hover:bg-white/20"
                        onClick={() => moveCarousel(1)}
                        aria-label="下一张海报"
                      >
                        ›
                      </button>
                    </>
                  ) : null}
                </div>
              </div>
            </div>
          </div>

          {topTenItems.length > 0 ? (
            <div className="scrollbar-hide -mx-1 flex gap-3 overflow-x-auto px-1 pb-1">
              {topTenItems.map((item, index) => {
                const isActive = index === activeIndex;
                const thumbnailCover = resolveCoverUrl(item);
                return (
                  <button
                    key={item.Id}
                    type="button"
                    onClick={() => setActiveIndex(index)}
                    className={`group relative w-[126px] shrink-0 overflow-hidden rounded-xl text-left transition-all duration-300 ${
                      isActive
                        ? "scale-[1.02] opacity-100"
                        : "scale-95 opacity-45 hover:scale-[0.99] hover:opacity-80"
                    }`}
                    style={
                      isActive && heroGlowColor
                        ? {
                            boxShadow: `0 0 20px 4px ${heroGlowColor}50, 0 0 10px 2px ${heroGlowColor}30`,
                          }
                        : undefined
                    }
                  >
                    {thumbnailCover ? (
                      <img
                        src={thumbnailCover}
                        alt={item.Name}
                        className="aspect-[2/3] w-full object-cover"
                        loading="lazy"
                        onError={() => markImageAsFailed(thumbnailCover)}
                      />
                    ) : (
                      <div className="flex aspect-[2/3] w-full items-center justify-center bg-gradient-to-br from-slate-950 via-slate-900 to-sky-950/45">
                        <span className="line-clamp-2 px-3 text-center text-xs font-semibold text-white/80">
                          {item.Name}
                        </span>
                      </div>
                    )}
                    <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black to-transparent px-2 pt-5 pb-2">
                      <p className="line-clamp-1 text-xs font-semibold text-white">{item.Name}</p>
                    </div>
                    <span className="absolute top-1.5 left-1.5 rounded-md bg-black/55 px-1.5 py-0.5 text-[10px] font-medium text-white/85">
                      #{index + 1}
                    </span>
                  </button>
                );
              })}
            </div>
          ) : null}
        </section>
      ) : null}

      <PosterRow
        title="继续观看"
        items={resumeItems}
        emptyTitle="没有续播记录"
        emptyDescription="当客户端上报播放进度后，这里会显示续播项。"
        token={session?.token}
        userId={session?.user.Id}
      />

      {state.rows.length > 0 && (
        <LibraryOverviewShelf
          rows={state.rows}
          resolveBackdropUrl={resolveBackdropUrl}
          resolveCoverUrl={resolveCoverUrl}
          onImageError={markImageAsFailed}
        />
      )}

      {rowsLoading ? (
        <LoadingState title="正在加载媒体分区" description="按媒体库加载最近内容" />
      ) : state.rows.length === 0 ? (
        <EmptyState title="暂无媒体分区" description="请先在管理后台配置媒体库并执行扫描任务。" />
      ) : (
        state.rows.map((row) => (
          <PosterRow
            key={row.root.Id}
            title={row.root.Name}
            subtitle="最近入库"
            items={row.items}
            emptyTitle={`${row.root.Name} 暂无内容`}
            emptyDescription="请稍后重试，或在管理端检查扫描任务状态。"
            actionHref={`/app/library/${row.root.Id}`}
            showViewAllCard
            token={session?.token}
            userId={session?.user.Id}
          />
        ))
      )}

      {/* 媒体总览 - 放在最底部 */}
      <section className="space-y-4">
        <div>
          <h2 className="text-2xl font-semibold tracking-tight">媒体总览</h2>
          <p className="light:text-foreground/50 text-sm text-white/58">内容库实时统计</p>
        </div>
        {counts ? (
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <MetricCard label="电影" value={counts.MovieCount} hint="Movies" />
            <MetricCard label="剧集" value={counts.SeriesCount} hint="Series" />
            <MetricCard label="剧集条目" value={counts.EpisodeCount} hint="Episodes" />
            <MetricCard label="音乐" value={counts.SongCount} hint="Songs" />
          </div>
        ) : (
          <EmptyState title="暂无计数信息" />
        )}
      </section>
    </div>
  );
}

interface PosterRowProps {
  title: string;
  subtitle?: string;
  badgeLabel?: string;
  items: BaseItem[];
  emptyTitle: string;
  emptyDescription: string;
  actionHref?: string;
  showViewAllCard?: boolean;
  token?: string;
  userId?: string;
}

function PosterRow({
  title,
  subtitle,
  badgeLabel,
  items,
  emptyTitle,
  emptyDescription,
  actionHref,
  showViewAllCard,
  token,
  userId,
}: PosterRowProps) {
  const scrollRef = useRef<HTMLDivElement>(null);

  const scrollLeft = () => {
    scrollRef.current?.scrollBy({ left: -400, behavior: "smooth" });
  };

  const scrollRight = () => {
    scrollRef.current?.scrollBy({ left: 400, behavior: "smooth" });
  };

  return (
    <section className="space-y-2">
      <div className="mb-1 flex items-end justify-between gap-3">
        <div className="space-y-1">
          <h2 className="light:text-foreground text-2xl font-semibold tracking-tight text-white/92">
            {title}
          </h2>
          {subtitle ? (
            <p className="text-xs tracking-wide text-white/45 uppercase">{subtitle}</p>
          ) : null}
        </div>
        <div className="flex items-center gap-3">
          {badgeLabel ? <span className="text-xs text-white/45">{badgeLabel}</span> : null}
          {actionHref ? (
            <a
              className="group flex items-center gap-1 text-xs text-white/68 transition-colors duration-200 hover:text-white"
              href={actionHref}
            >
              查看全部
              <svg
                className="h-3 w-3 transition-transform duration-200 group-hover:translate-x-0.5"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M9 5l7 7-7 7"
                />
              </svg>
            </a>
          ) : null}
        </div>
      </div>

      {items.length === 0 ? (
        <EmptyState title={emptyTitle} description={emptyDescription} />
      ) : (
        <div className="relative">
          {/* Left scroll button */}
          <button
            type="button"
            onClick={scrollLeft}
            className="absolute top-1/2 left-0 z-10 hidden h-10 w-10 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full border border-white/10 bg-black/40 text-white backdrop-blur-sm transition-all duration-200 hover:border-white/20 hover:bg-black/60 md:flex"
            aria-label="向左滚动"
          >
            <svg
              className="h-5 w-5"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
            </svg>
          </button>

          {/* Right scroll button */}
          <button
            type="button"
            onClick={scrollRight}
            className="absolute top-1/2 right-0 z-10 hidden h-10 w-10 translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full border border-white/10 bg-black/40 text-white backdrop-blur-sm transition-all duration-200 hover:border-white/20 hover:bg-black/60 md:flex"
            aria-label="向右滚动"
          >
            <svg
              className="h-5 w-5"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
            </svg>
          </button>

          <div
            ref={scrollRef}
            className="scrollbar-hide -mx-20 flex items-start gap-5 overflow-x-auto px-20 pt-3 pb-3"
            style={{
              overscrollBehaviorX: "contain",
              contentVisibility: "auto",
              containIntrinsicBlockSize: "320px",
            }}
          >
            {items.slice(0, 30).map((item, index) => (
              <div
                key={item.Id}
                className="animate-fade-in-up"
                style={{ animationDelay: `${index * 50}ms`, animationFillMode: "both" }}
              >
                <PosterItemCard
                  item={item}
                  href={resolveMediaItemHref(item)}
                  token={token}
                  userId={userId}
                />
              </div>
            ))}
            {showViewAllCard && actionHref ? (
              <div
                className="animate-fade-in-up"
                style={{
                  animationDelay: `${Math.min(items.length, 30) * 50}ms`,
                  animationFillMode: "both",
                }}
              >
                <a href={actionHref} className="group block w-[170px] shrink-0 sm:w-[190px]">
                  <div className="py-2">
                    <div className="relative flex aspect-[2/3] items-center justify-center overflow-hidden rounded-xl border border-white/[0.08] bg-white/[0.03] transition-all duration-300 group-hover:-translate-y-2 group-hover:border-white/20 group-hover:bg-white/[0.06]">
                      <div className="flex flex-col items-center gap-3 text-center">
                        <div className="flex h-12 w-12 items-center justify-center rounded-full border border-white/15 bg-white/[0.06] transition-colors group-hover:border-white/25 group-hover:bg-white/10">
                          <svg
                            className="h-5 w-5 text-white/60 transition-transform duration-300 group-hover:translate-x-0.5 group-hover:text-white/90"
                            fill="none"
                            viewBox="0 0 24 24"
                            stroke="currentColor"
                            strokeWidth={2}
                          >
                            <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
                          </svg>
                        </div>
                        <span className="text-sm font-medium text-white/60 transition-colors group-hover:text-white/90">
                          查看全部
                        </span>
                      </div>
                    </div>
                  </div>
                </a>
              </div>
            ) : null}
          </div>
        </div>
      )}
    </section>
  );
}

interface LibraryOverviewShelfProps {
  rows: DashboardRow[];
  resolveCoverUrl: (item: { Id: string; ImagePrimaryUrl?: string | null }) => string | null;
  resolveBackdropUrl: (item: { Id: string }) => string | null;
  onImageError: (url: string | null | undefined) => void;
}

function LibraryOverviewShelf({
  rows,
  resolveCoverUrl,
  resolveBackdropUrl,
  onImageError,
}: LibraryOverviewShelfProps) {
  const scrollRef = useRef<HTMLDivElement>(null);

  const scrollLeft = () => {
    scrollRef.current?.scrollBy({ left: -420, behavior: "smooth" });
  };

  const scrollRight = () => {
    scrollRef.current?.scrollBy({ left: 420, behavior: "smooth" });
  };

  const cards = useMemo(() => {
    return rows.map((row) => {
      // Priority: admin-uploaded library image -> library backdrop -> first playable child image.
      const rootCover = resolveCoverUrl(row.root);
      const rootBackdrop = resolveBackdropUrl(row.root);
      let coverUrl: string | null = rootCover ?? rootBackdrop ?? null;

      if (!coverUrl) {
        for (const item of row.items) {
          const itemBackdrop = resolveBackdropUrl(item);
          if (itemBackdrop) {
            coverUrl = itemBackdrop;
            break;
          }

          const itemCover = resolveCoverUrl(item);
          if (itemCover) {
            coverUrl = itemCover;
            break;
          }
        }
      }

      return {
        row,
        coverUrl,
      };
    });
  }, [resolveBackdropUrl, resolveCoverUrl, rows]);

  return (
    <section className="space-y-3">
      <div>
        <h2 className="light:text-foreground text-2xl font-semibold tracking-tight text-white/92">
          媒体库
        </h2>
        <p className="light:text-foreground/50 text-sm text-white/58">所有媒体库概览</p>
      </div>

      <div className="relative">
        <button
          type="button"
          onClick={scrollLeft}
          className="absolute top-1/2 left-0 z-10 hidden h-10 w-10 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full border border-white/10 bg-black/40 text-white backdrop-blur-sm transition-all duration-200 hover:border-white/20 hover:bg-black/60 md:flex"
          aria-label="向左滑动媒体库"
        >
          <svg
            className="h-5 w-5"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            strokeWidth={2}
          >
            <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
          </svg>
        </button>

        <button
          type="button"
          onClick={scrollRight}
          className="absolute top-1/2 right-0 z-10 hidden h-10 w-10 translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full border border-white/10 bg-black/40 text-white backdrop-blur-sm transition-all duration-200 hover:border-white/20 hover:bg-black/60 md:flex"
          aria-label="向右滑动媒体库"
        >
          <svg
            className="h-5 w-5"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            strokeWidth={2}
          >
            <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
          </svg>
        </button>

        <div
          ref={scrollRef}
          className="scrollbar-hide -mx-20 flex snap-x snap-mandatory items-start gap-5 overflow-x-auto px-20 pt-1 pb-1"
          style={{ overscrollBehaviorX: "contain" }}
          aria-label="媒体库横向列表"
        >
          {cards.map(({ row, coverUrl }, index) => (
            <LibraryOverviewCard
              key={row.root.Id}
              row={row}
              coverUrl={coverUrl}
              onImageError={onImageError}
              animationDelayMs={index * 40}
            />
          ))}
        </div>
      </div>
    </section>
  );
}

interface LibraryOverviewCardProps {
  row: DashboardRow;
  coverUrl: string | null;
  onImageError: (url: string | null | undefined) => void;
  animationDelayMs: number;
}

function LibraryOverviewCard({
  row,
  coverUrl,
  onImageError,
  animationDelayMs,
}: LibraryOverviewCardProps) {
  const [isHovered, setIsHovered] = useState(false);
  const { glowColor } = useImageGlow(coverUrl ?? undefined, {
    enabled: Boolean(coverUrl) && isHovered,
  });

  return (
    <a
      href={`/app/library/${row.root.Id}`}
      className="group relative block w-[280px] shrink-0 snap-start sm:w-[340px]"
      style={{ animationDelay: `${animationDelayMs}ms`, animationFillMode: "both" }}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      <div className="relative py-2" style={{ contain: "layout style" }}>
        {coverUrl && isHovered ? (
          <div
            className="pointer-events-none absolute inset-0 z-0 animate-[glow-fade-in_250ms_ease-out_forwards]"
            aria-hidden="true"
          >
            <div
              className="absolute top-2 right-1 bottom-2 left-1 rounded-xl opacity-55"
              style={{
                backgroundImage: `url(${coverUrl})`,
                backgroundSize: "cover",
                backgroundPosition: "center",
                filter: "blur(16px) saturate(130%) brightness(1.08)",
                transform: "scale(1.02)",
              }}
            />
            <div
              className="absolute top-2 right-1 bottom-2 left-1 rounded-xl"
              style={{
                boxShadow: glowColor
                  ? `0 0 26px ${glowColor}66, 0 0 50px ${glowColor}36`
                  : "0 0 22px rgba(255,255,255,0.14)",
              }}
            />
          </div>
        ) : null}

        <div className="relative z-10 aspect-video overflow-hidden rounded-xl bg-black transition-all duration-300 ease-out group-hover:-translate-y-1 group-hover:scale-[1.01] group-hover:shadow-[0_20px_45px_-18px_rgba(0,0,0,0.75)]">
          {coverUrl ? (
            <img
              src={coverUrl}
              alt={row.root.Name}
              className="h-full w-full object-cover transition-transform duration-500 group-hover:scale-105"
              loading="lazy"
              onError={() => onImageError(coverUrl)}
            />
          ) : (
            <div className="flex h-full w-full items-end bg-gradient-to-br from-slate-950 via-slate-900 to-sky-950/45 p-4">
              <p className="line-clamp-2 text-sm font-semibold text-white/85">{row.root.Name}</p>
            </div>
          )}
          <div className="absolute inset-0 bg-gradient-to-t from-black/26 to-transparent" />
        </div>
      </div>

      <div className="px-1 pt-2">
        <p className="light:text-foreground line-clamp-1 text-base font-semibold text-white/92">
          {row.root.Name}
        </p>
      </div>
    </a>
  );
}

function MetricCard({ label, value, hint }: { label: string; value: number; hint: string }) {
  return (
    <div className="light:bg-black/[0.03] rounded-2xl bg-white/[0.03] p-5 backdrop-blur-sm">
      <p className="light:text-foreground/45 text-xs tracking-[0.14em] text-white/45 uppercase">
        {hint}
      </p>
      <p className="light:text-foreground mt-2 text-3xl font-semibold text-white/95">{value}</p>
      <p className="light:text-foreground/55 mt-1 text-xs text-white/55">{label}</p>
    </div>
  );
}
