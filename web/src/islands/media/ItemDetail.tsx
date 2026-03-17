import { useEffect, useState } from "react";

import { AddToPlaylistModal } from "@/components/domain/AddToPlaylistModal";
import { AdminItemModal } from "@/components/domain/AdminItemModal";
import { getRouteParam } from "@/lib/hooks/use-route-param";
import { formatEpisodeCode } from "@/lib/media/episode-label";
import { EmptyState, ErrorState } from "@/components/domain/DataState";
import { PlayerPickerModal } from "@/components/domain/PlayerPickerModal";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Select } from "@/components/ui/select";
import {
  addFavoriteItem,
  buildItemBackdropUrl,
  buildItemImageUrl,
  buildPersonImageUrl,
  buildStreamUrl,
  getItemSubtitles,
  getPlaybackInfo,
  getShowEpisodes,
  getShowSeasons,
  getUserItem,
  removeFavoriteItem,
} from "@/lib/api/items";
import type { ApiError } from "@/lib/api/client";
import { canAccessAdmin } from "@/lib/auth/token";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import { toast } from "@/lib/notifications/toast-store";
import type {
  BaseItem,
  BaseItemPerson,
  MediaSource,
  MediaStream,
  PlaybackInfo,
  Season,
  SubtitleTrack,
} from "@/lib/types/jellyfin";

function formatDuration(runtimeTicks?: number | null): string {
  if (!runtimeTicks) {
    return "未知时长";
  }

  const totalMinutes = Math.max(1, Math.floor(runtimeTicks / 10_000_000 / 60));
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;

  if (hours > 0) {
    return `${hours} 小时 ${minutes} 分钟`;
  }

  return `${minutes} 分钟`;
}

function formatSeriesEpisodeCount(item: BaseItem, loadedEpisodes: number): string | null {
  if (item.Type !== "Series") {
    return null;
  }
  if (typeof item.ChildCount === "number" && item.ChildCount >= 0) {
    return `${item.ChildCount} 集`;
  }
  if (loadedEpisodes > 0) {
    return `${loadedEpisodes} 集`;
  }
  return null;
}

function formatBitrate(bitrate?: number | null): string {
  if (!bitrate) {
    return "未知码率";
  }

  if (bitrate >= 1_000_000) {
    return `${(bitrate / 1_000_000).toFixed(1)} Mbps`;
  }

  return `${Math.round(bitrate / 1000)} kbps`;
}

function formatLanguage(language?: string | null): string {
  if (!language || language.trim().length === 0) {
    return "未知";
  }
  return language.toUpperCase();
}

function roleType(person: BaseItemPerson): string {
  return (person.Type || "").trim().toLowerCase();
}

function roleLabel(person: BaseItemPerson): string {
  return (person.Role || "").trim().toLowerCase();
}

function isActor(person: BaseItemPerson): boolean {
  return roleType(person) === "actor";
}

function isDirector(person: BaseItemPerson): boolean {
  return roleType(person) === "director" || roleLabel(person).includes("director");
}

function isWriter(person: BaseItemPerson): boolean {
  if (roleType(person) === "writer") {
    return true;
  }

  const role = roleLabel(person);
  return role.includes("writer") || role.includes("screenplay") || role.includes("story");
}

function uniquePeople(people: BaseItemPerson[]): BaseItemPerson[] {
  const seen = new Set<string>();
  const items: BaseItemPerson[] = [];

  people.forEach((person) => {
    const key = `${person.Id || ""}:${person.Name}`;
    if (seen.has(key)) {
      return;
    }

    seen.add(key);
    items.push(person);
  });

  return items;
}

function buildPersonHref(person: BaseItemPerson): string {
  const personName = person.Name.trim();
  const personId = (person.Id || "").trim();
  if (personId) {
    return `/app/person/${encodeURIComponent(personId)}`;
  }

  const params = new URLSearchParams();
  params.set("type", "Movie,Series");
  if (personName) {
    params.set("q", personName);
  }
  return `/app/search?${params.toString()}`;
}

function subtitleTrackFromStream(stream: MediaStream): SubtitleTrack {
  const codec = stream.Codec || "subtitle";
  const language = stream.Language || null;
  const displayTitle =
    stream.DisplayTitle ||
    (language ? `${language.toUpperCase()} (${codec.toUpperCase()})` : codec.toUpperCase());

  return {
    Index: stream.Index,
    Codec: codec,
    Language: language,
    DisplayTitle: displayTitle,
    IsExternal: stream.IsExternal,
    IsDefault: Boolean(stream.IsDefault),
  };
}

function streamType(stream: MediaStream): string {
  return (stream.Type || "").trim().toLowerCase();
}

function formatChannelLayout(channels?: number | null): string {
  if (!channels) return "";
  if (channels === 1) return "mono";
  if (channels === 2) return "stereo";
  if (channels === 6) return "5.1";
  if (channels === 8) return "7.1";
  return `${channels}ch`;
}

function formatCompactTechLine(streams: MediaStream[]): string {
  const parts: string[] = [];

  const video = streams.find((s) => streamType(s) === "video");
  if (video) {
    const codec = (video.Codec || "").toUpperCase();
    // Height is a standard Jellyfin field not declared in our type
    const height = (video as MediaStream & { Height?: number }).Height;
    const res = height ? `${height}p` : null;
    parts.push(`视频 ${[res, codec].filter(Boolean).join(" ")}`);
  }

  const audio = streams.find((s) => streamType(s) === "audio" && s.IsDefault);
  const audioFallback = audio || streams.find((s) => streamType(s) === "audio");
  if (audioFallback) {
    const codec = (audioFallback.Codec || "").toUpperCase();
    const layout = formatChannelLayout(audioFallback.Channels);
    const suffix = audioFallback.IsDefault ? "（默认）" : "";
    parts.push(`音频 ${[codec, layout].filter(Boolean).join(" ")}${suffix}`);
  }

  return parts.join("　");
}

function DetailSkeleton() {
  return (
    <div className="animate-pulse space-y-8">
      <section
        className="relative overflow-hidden bg-black"
        style={{
          marginLeft: "calc(-50vw + 50%)",
          marginRight: "calc(-50vw + 50%)",
          width: "100vw",
        }}
      >
        <div className="min-h-[60vh] md:min-h-[65vh]">
          <div className="absolute inset-0 bg-gradient-to-r from-black/90 via-black/60 to-black/30" />
          <div className="absolute inset-0 bg-gradient-to-t from-black via-black/60 to-transparent" />
          <div className="relative mx-auto flex min-h-[65vh] max-w-7xl items-end px-4 pb-10 sm:px-6 md:min-h-[75vh] lg:px-8">
            <div className="hidden w-[150px] shrink-0 md:block">
              <div className="aspect-[2/3] rounded-lg bg-neutral-800/70" />
            </div>
            <div className="flex-1 space-y-4 md:ml-6">
              <div className="h-10 w-2/3 rounded bg-neutral-800/70" />
              <div className="h-5 w-1/3 rounded bg-neutral-800/50" />
              <div className="h-4 w-1/4 rounded bg-neutral-800/40" />
              <div className="flex gap-3">
                <div className="h-10 w-28 rounded bg-neutral-800/70" />
                <div className="h-10 w-10 rounded-full bg-neutral-800/55" />
              </div>
              <div className="h-16 w-full max-w-xl rounded bg-neutral-800/30" />
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}

interface ItemDetailProps {
  itemId?: string;
}

export function ItemDetail({ itemId: itemIdProp }: ItemDetailProps) {
  const itemId = itemIdProp || getRouteParam("item");
  const { session, ready } = useAuthSession();
  const [item, setItem] = useState<BaseItem | null>(null);
  const [playbackInfo, setPlaybackInfo] = useState<PlaybackInfo | null>(null);
  const [subtitles, setSubtitles] = useState<SubtitleTrack[]>([]);
  const [seasons, setSeasons] = useState<Season[]>([]);
  const [episodes, setEpisodes] = useState<BaseItem[]>([]);
  const [selectedSeasonId, setSelectedSeasonId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [episodesLoading, setEpisodesLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [favoritePending, setFavoritePending] = useState(false);
  const [playlistModalOpen, setPlaylistModalOpen] = useState(false);
  const [playerPickerOpen, setPlayerPickerOpen] = useState(false);
  const [playerStreamUrl, setPlayerStreamUrl] = useState("");
  const [posterFailed, setPosterFailed] = useState(false);
  const [heroFallbackToPoster, setHeroFallbackToPoster] = useState(false);
  const [heroImageFailed, setHeroImageFailed] = useState(false);
  const [personImageFailures, setPersonImageFailures] = useState<Record<string, true>>({});
  const [techExpanded, setTechExpanded] = useState(false);
  const [adminModalOpen, setAdminModalOpen] = useState(false);

  async function reloadCurrentItem() {
    if (!session) {
      return;
    }

    const latest = await getUserItem(session.user.Id, itemId);
    setItem(latest);

    if (latest.Type === "Series") {
      const seasonsResult = await getShowSeasons(latest.Id);
      setSeasons(seasonsResult.Items);
      if (seasonsResult.Items.length > 0) {
        setSelectedSeasonId((current) =>
          current && seasonsResult.Items.some((season) => season.Id === current)
            ? current
            : seasonsResult.Items[0]!.Id
        );
      } else {
        setSelectedSeasonId(null);
      }
    } else {
      setSeasons([]);
      setEpisodes([]);
      setSelectedSeasonId(null);
    }
  }

  useEffect(() => {
    if (!ready || !session) {
      return;
    }

    let cancelled = false;
    setLoading(true);

    Promise.all([
      getUserItem(session.user.Id, itemId),
      getItemSubtitles(itemId).catch(() => []),
      getPlaybackInfo(itemId, session.user.Id).catch(() => null),
    ])
      .then(async ([itemResult, subtitlesResult, playbackResult]) => {
        if (cancelled) {
          return;
        }

        setItem(itemResult);
        setSubtitles(subtitlesResult);
        setPlaybackInfo(playbackResult);
        setPosterFailed(false);
        setHeroFallbackToPoster(false);
        setHeroImageFailed(false);
        setPersonImageFailures({});
        setError(null);

        if (itemResult.Type === "Series") {
          const seasonsResult = await getShowSeasons(itemResult.Id);
          if (!cancelled) {
            setSeasons(seasonsResult.Items);
            if (seasonsResult.Items.length > 0) {
              setSelectedSeasonId(seasonsResult.Items[0]!.Id);
            }
          }
        }
      })
      .catch((cause) => {
        if (cancelled) {
          return;
        }

        const apiError = cause as ApiError;
        setError(apiError.message || "加载条目详情失败");
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [itemId, ready, session]);

  useEffect(() => {
    if (!item || item.Type !== "Series" || !selectedSeasonId) {
      return;
    }

    let cancelled = false;
    setEpisodesLoading(true);

    getShowEpisodes(item.Id, { seasonId: selectedSeasonId })
      .then((result) => {
        if (!cancelled) {
          setEpisodes(result.Items);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setEpisodes([]);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setEpisodesLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [item, selectedSeasonId]);

  function onPlay() {
    if (!session || !item) {
      return;
    }

    const url = buildStreamUrl(item.Id, session.token);
    setPlayerStreamUrl(url);
    setPlayerPickerOpen(true);
  }

  async function onToggleFavorite() {
    if (!session || !item || favoritePending) {
      return;
    }

    const nextFavorite = !item.UserData?.IsFavorite;
    setFavoritePending(true);

    setItem((prev) => {
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
        await addFavoriteItem(session.user.Id, item.Id);
        toast.success("已加入收藏");
      } else {
        await removeFavoriteItem(session.user.Id, item.Id);
        toast.info("已取消收藏");
      }
    } catch {
      setItem((prev) => {
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
      toast.error("收藏状态更新失败");
    } finally {
      setFavoritePending(false);
    }
  }

  if (!ready || loading) {
    return <DetailSkeleton />;
  }

  if (error) {
    return <ErrorState title="详情加载失败" description={error} />;
  }

  if (!item) {
    return <EmptyState title="找不到条目" description="该媒体可能已经被删除或 ID 无效。" />;
  }

  const token = session?.token;
  const posterUrl = item.ImagePrimaryUrl || buildItemImageUrl(item.Id, token);
  const backdropUrl = buildItemBackdropUrl(item.Id, token, 0);
  const heroImageUrl = heroFallbackToPoster ? posterUrl : backdropUrl;
  const isFavorite = Boolean(item.UserData?.IsFavorite);
  const isAdmin = canAccessAdmin(session?.user || null);
  const people = item.People || [];
  const actors = uniquePeople(people.filter(isActor));
  const directors = uniquePeople(people.filter(isDirector));
  const writers = uniquePeople(people.filter(isWriter));

  const mediaSource: MediaSource | null =
    playbackInfo?.MediaSources?.[0] || item.MediaSources?.[0] || null;
  const mediaStreams = mediaSource?.MediaStreams || [];
  const audioStreams = mediaStreams.filter((stream) => streamType(stream) === "audio");
  const subtitleStreams = mediaStreams.filter((stream) => streamType(stream) === "subtitle");
  const subtitleItems = subtitles.length
    ? subtitles
    : subtitleStreams.map((stream) => subtitleTrackFromStream(stream));
  const compactTechLine = formatCompactTechLine(mediaStreams);

  return (
    <>
      <div className="space-y-8">
        {/* ── Hero: fullscreen backdrop ── */}
        <section
          className="relative overflow-hidden bg-black"
          style={{
            marginLeft: "calc(-50vw + 50%)",
            marginRight: "calc(-50vw + 50%)",
            width: "100vw",
          }}
        >
          {!heroImageFailed ? (
            <img
              src={heroImageUrl}
              alt={item.Name}
              className="absolute inset-0 h-full w-full object-cover"
              style={{ animation: "hero-zoom 28s ease-out infinite alternate" }}
              onError={() => {
                if (!heroFallbackToPoster) {
                  setHeroFallbackToPoster(true);
                  return;
                }
                setHeroImageFailed(true);
              }}
            />
          ) : (
            <div className="absolute inset-0 bg-gradient-to-r from-slate-950 via-slate-900 to-slate-800" />
          )}
          <div className="absolute inset-0 bg-gradient-to-r from-black/92 via-black/68 to-black/28" />
          <div className="absolute inset-0 bg-gradient-to-t from-black via-black/52 to-transparent" />
          <div className="absolute inset-0 bg-[radial-gradient(circle_at_15%_25%,rgba(255,255,255,0.14),transparent_45%)]" />
          <div className="from-background pointer-events-none absolute inset-x-0 bottom-0 h-24 bg-gradient-to-t to-transparent" />

          <div className="relative mx-auto flex min-h-[70vh] max-w-7xl items-end gap-6 px-4 pb-10 sm:px-6 md:min-h-[80vh] lg:px-8">
            {/* Poster — hidden on mobile */}
            <div className="hidden w-[150px] shrink-0 overflow-hidden rounded-lg shadow-2xl md:block">
              {!posterFailed ? (
                <img
                  src={posterUrl}
                  alt={item.Name}
                  className="aspect-[2/3] w-full object-cover"
                  onError={() => setPosterFailed(true)}
                />
              ) : (
                <div className="flex aspect-[2/3] items-center justify-center bg-gradient-to-br from-neutral-800 to-neutral-950 text-center text-xs text-neutral-300">
                  海报加载失败
                </div>
              )}
            </div>

            <div className="min-w-0 flex-1 space-y-3 text-white">
              <p className="text-xs tracking-[0.14em] text-white/58 uppercase">
                {item.Type === "Series" ? "Series" : item.Type === "Movie" ? "Movie" : item.Type}
              </p>
              <h1 className="text-4xl font-semibold tracking-tight md:text-6xl">{item.Name}</h1>

              {/* Meta row */}
              <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-neutral-300">
                {item.ProductionYear ? <span>{item.ProductionYear}</span> : null}
                {item.ProductionYear &&
                (item.Type === "Series"
                  ? Boolean(formatSeriesEpisodeCount(item, episodes.length))
                  : Boolean(item.RunTimeTicks)) ? (
                  <span className="text-neutral-500">·</span>
                ) : null}
                {item.Type === "Series" ? (
                  formatSeriesEpisodeCount(item, episodes.length) ? (
                    <span>{formatSeriesEpisodeCount(item, episodes.length)}</span>
                  ) : null
                ) : item.RunTimeTicks ? (
                  <span>{formatDuration(item.RunTimeTicks)}</span>
                ) : null}
                {typeof item.CommunityRating === "number" ? (
                  <>
                    <span className="text-neutral-500">·</span>
                    <span className="text-white/88">{item.CommunityRating.toFixed(1)}</span>
                  </>
                ) : null}
                {item.OfficialRating ? (
                  <>
                    <span className="text-neutral-500">·</span>
                    <Badge variant="glass" className="text-[11px]">
                      {item.OfficialRating}
                    </Badge>
                  </>
                ) : null}
              </div>

              {/* Genres */}
              {item.Genres && item.Genres.length > 0 ? (
                <p className="text-sm text-neutral-400">{item.Genres.join(" / ")}</p>
              ) : null}

              {/* Compact tech line */}
              {compactTechLine ? (
                <p className="text-xs tracking-wide text-neutral-500">{compactTechLine}</p>
              ) : null}

              {/* Buttons */}
              <div className="flex items-center gap-3 pt-1">
                <Button onClick={onPlay} className="gap-2" variant="immersive">
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 20 20"
                    fill="currentColor"
                    className="h-4 w-4"
                    aria-hidden="true"
                  >
                    <path d="M6.3 2.84A1.5 1.5 0 004 4.11v11.78a1.5 1.5 0 002.3 1.27l9.344-5.891a1.5 1.5 0 000-2.538L6.3 2.841z" />
                  </svg>
                  播放
                </Button>
                <button
                  type="button"
                  onClick={onToggleFavorite}
                  disabled={favoritePending}
                  className={`flex h-10 w-10 items-center justify-center rounded-full border border-white/20 bg-white/10 text-white transition-colors disabled:opacity-50 ${
                    isFavorite ? "bg-white/22 hover:bg-white/30" : "hover:bg-white/16"
                  }`}
                  aria-label={isFavorite ? "取消收藏" : "收藏"}
                >
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 20 20"
                    fill="currentColor"
                    className="h-5 w-5"
                    aria-hidden="true"
                  >
                    <path d="M9.653 16.915l-.005-.003-.019-.01a20.759 20.759 0 01-1.162-.682 22.045 22.045 0 01-2.582-1.9C4.045 12.733 2 10.352 2 7.5a4.5 4.5 0 018-2.828A4.5 4.5 0 0118 7.5c0 2.852-2.044 5.233-3.885 6.82a22.049 22.049 0 01-3.744 2.582l-.019.01-.005.003h-.002a.723.723 0 01-.692 0h-.002z" />
                  </svg>
                  <span className="sr-only">{isFavorite ? "已收藏（点击取消）" : "收藏"}</span>
                </button>
                <button
                  type="button"
                  onClick={() => setPlaylistModalOpen(true)}
                  className="flex h-10 w-10 items-center justify-center rounded-full border border-white/20 bg-white/10 text-white transition-colors hover:bg-white/16"
                  aria-label="添加到列表"
                >
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 20 20"
                    fill="currentColor"
                    className="h-5 w-5"
                    aria-hidden="true"
                  >
                    <path d="M3 5a1 1 0 011-1h8a1 1 0 110 2H4a1 1 0 01-1-1zm0 5a1 1 0 011-1h8a1 1 0 110 2H4a1 1 0 01-1-1zm1 4a1 1 0 100 2h5a1 1 0 100-2H4zm11-2a1 1 0 10-2 0v1h-1a1 1 0 100 2h1v1a1 1 0 102 0v-1h1a1 1 0 100-2h-1v-1z" />
                  </svg>
                </button>
                {isAdmin ? (
                  <button
                    type="button"
                    onClick={() => setAdminModalOpen(true)}
                    className="flex h-10 w-10 items-center justify-center rounded-full border border-amber-300/40 bg-amber-300/12 text-amber-100 transition-colors hover:bg-amber-300/20"
                    aria-label="管理员编辑"
                  >
                    <svg
                      xmlns="http://www.w3.org/2000/svg"
                      viewBox="0 0 20 20"
                      fill="currentColor"
                      className="h-5 w-5"
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

              {/* Overview — max 3 lines */}
              <p className="line-clamp-3 max-w-2xl text-[15px] leading-relaxed text-white/72">
                {item.Overview || "暂无简介。"}
              </p>
            </div>
          </div>
        </section>

        {/* ── Episodes (Series only) ── */}
        {item.Type === "Series" ? (
          <section className="space-y-4">
            <div className="flex items-center gap-3">
              <h2 className="text-lg font-semibold">剧集</h2>
              {seasons.length > 1 ? (
                <Select
                  value={selectedSeasonId || ""}
                  onChange={(e) => setSelectedSeasonId(e.target.value)}
                  className="h-8 w-auto text-xs"
                >
                  {seasons.map((season) => (
                    <option key={season.Id} value={season.Id}>
                      {season.Name}
                    </option>
                  ))}
                </Select>
              ) : null}
            </div>

            {episodesLoading ? (
              <p className="text-muted-foreground text-sm">加载剧集中...</p>
            ) : episodes.length === 0 ? (
              <p className="text-muted-foreground text-sm">暂无剧集信息。</p>
            ) : (
              <div
                className="scrollbar-hide flex gap-4 overflow-x-auto pb-2"
                style={{ overscrollBehaviorX: "contain" }}
              >
                {episodes.map((episode) => {
                  const epThumb = buildItemImageUrl(episode.Id, token);
                  return (
                    <a
                      key={episode.Id}
                      href={`/app/item/${episode.Id}`}
                      className="group w-[120px] shrink-0 space-y-2 text-center"
                    >
                      <div className="relative overflow-hidden rounded-lg transition-transform group-hover:scale-105">
                        <img
                          src={epThumb}
                          alt={episode.Name}
                          className="aspect-[2/3] w-full object-cover"
                          onError={(e) => {
                            (e.target as HTMLImageElement).style.display = "none";
                          }}
                        />
                        {episode.UserData?.Played ? (
                          <div className="absolute top-1.5 right-1.5">
                            <Badge variant="success" className="text-[10px]">
                              已看
                            </Badge>
                          </div>
                        ) : null}
                      </div>
                      <p className="line-clamp-2 text-sm group-hover:underline">
                        {formatEpisodeCode(episode.ParentIndexNumber, episode.IndexNumber)}
                      </p>
                      <p className="text-muted-foreground line-clamp-1 text-xs">{episode.Name}</p>
                    </a>
                  );
                })}
              </div>
            )}
          </section>
        ) : null}

        {/* ── Cast & Crew ── */}
        <section className="space-y-4">
          <h2 className="text-2xl font-semibold tracking-tight">演职员</h2>
          {actors.length === 0 && directors.length === 0 && writers.length === 0 ? (
            <p className="text-muted-foreground text-sm">暂无演职员信息。</p>
          ) : (
            <>
              {actors.length > 0 ? (
                <div className="space-y-3">
                  <h3 className="light:text-foreground/60 text-sm tracking-wide text-white/70 uppercase">
                    演员
                  </h3>
                  <div
                    className="scrollbar-hide flex gap-4 overflow-x-auto pb-2"
                    style={{ overscrollBehaviorX: "contain" }}
                  >
                    {actors.map((person) => {
                      const personKey = person.Id || person.Name;
                      const personImage = person.Id ? buildPersonImageUrl(person.Id, token) : null;
                      const avatarFailed = Boolean(personImageFailures[personKey]);

                      return (
                        <a
                          key={personKey}
                          href={buildPersonHref(person)}
                          className="group w-[120px] shrink-0 space-y-2 text-center"
                        >
                          <div className="relative overflow-hidden rounded-lg transition-transform group-hover:scale-105">
                            {personImage && !avatarFailed ? (
                              <img
                                src={personImage}
                                alt={person.Name}
                                className="aspect-[2/3] w-full object-cover"
                                onError={() => {
                                  setPersonImageFailures((prev) => ({
                                    ...prev,
                                    [personKey]: true,
                                  }));
                                }}
                              />
                            ) : (
                              <div className="flex aspect-[2/3] items-center justify-center bg-neutral-800 text-sm text-neutral-400">
                                {person.Name.slice(0, 2)}
                              </div>
                            )}
                            <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/90 via-black/50 to-transparent px-2 pt-8 pb-2">
                              <p className="line-clamp-1 text-sm text-white">{person.Name}</p>
                            </div>
                          </div>
                          <p className="text-muted-foreground line-clamp-1 text-xs">
                            {person.Role || "演员"}
                          </p>
                        </a>
                      );
                    })}
                  </div>
                </div>
              ) : null}

              {directors.length > 0 ? (
                <div className="space-y-3">
                  <h3 className="light:text-foreground/60 text-sm tracking-wide text-white/70 uppercase">
                    导演
                  </h3>
                  <div
                    className="scrollbar-hide flex gap-4 overflow-x-auto pb-2"
                    style={{ overscrollBehaviorX: "contain" }}
                  >
                    {directors.map((person) => {
                      const personKey = `${person.Id || person.Name}-director`;
                      const personImage = person.Id ? buildPersonImageUrl(person.Id, token) : null;
                      const avatarFailed = Boolean(personImageFailures[personKey]);

                      return (
                        <a
                          key={personKey}
                          href={buildPersonHref(person)}
                          className="group w-[120px] shrink-0 space-y-2 text-center"
                        >
                          <div className="relative overflow-hidden rounded-lg transition-transform group-hover:scale-105">
                            {personImage && !avatarFailed ? (
                              <img
                                src={personImage}
                                alt={person.Name}
                                className="aspect-[2/3] w-full object-cover"
                                onError={() => {
                                  setPersonImageFailures((prev) => ({
                                    ...prev,
                                    [personKey]: true,
                                  }));
                                }}
                              />
                            ) : (
                              <div className="flex aspect-[2/3] items-center justify-center bg-neutral-800 text-sm text-neutral-400">
                                {person.Name.slice(0, 2)}
                              </div>
                            )}
                            <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/90 via-black/50 to-transparent px-2 pt-8 pb-2">
                              <p className="line-clamp-1 text-sm text-white">{person.Name}</p>
                            </div>
                          </div>
                          <p className="text-muted-foreground text-xs">导演</p>
                        </a>
                      );
                    })}
                  </div>
                </div>
              ) : null}

              {writers.length > 0 ? (
                <div className="space-y-2">
                  <h3 className="light:text-foreground/60 text-sm tracking-wide text-white/70 uppercase">
                    编剧
                  </h3>
                  <div className="flex flex-wrap gap-2">
                    {writers.map((person) => (
                      <a
                        key={`${person.Id || person.Name}-writer`}
                        href={buildPersonHref(person)}
                        className="light:border-black/10 light:bg-black/[0.05] light:text-foreground/70 light:hover:border-black/20 light:hover:bg-black/[0.08] inline-flex items-center rounded-full border border-white/15 bg-white/8 px-2.5 py-0.5 text-xs font-medium text-white/85 transition-colors hover:border-white/30 hover:bg-white/15"
                      >
                        {person.Name}
                      </a>
                    ))}
                  </div>
                </div>
              ) : null}
            </>
          )}
        </section>

        {/* ── Collapsible Media Info ── */}
        <section className="space-y-3">
          <button
            type="button"
            onClick={() => setTechExpanded((prev) => !prev)}
            className="flex items-center gap-2 text-lg font-semibold transition-colors hover:text-neutral-300"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 20 20"
              fill="currentColor"
              className={`h-4 w-4 transition-transform ${techExpanded ? "rotate-90" : ""}`}
              aria-hidden="true"
            >
              <path
                fillRule="evenodd"
                d="M7.21 14.77a.75.75 0 01.02-1.06L11.168 10 7.23 6.29a.75.75 0 111.04-1.08l4.5 4.25a.75.75 0 010 1.08l-4.5 4.25a.75.75 0 01-1.06-.02z"
                clipRule="evenodd"
              />
            </svg>
            媒体技术信息
          </button>

          {techExpanded ? (
            <div className="space-y-4 pl-6">
              <div className="flex flex-wrap gap-x-6 gap-y-2 text-sm">
                <span>时长: {formatDuration(mediaSource?.RunTimeTicks ?? item.RunTimeTicks)}</span>
                <span>码率: {formatBitrate(mediaSource?.Bitrate ?? item.Bitrate)}</span>
                <span>容器: {(mediaSource?.Container || "未知").toUpperCase()}</span>
              </div>

              <div className="space-y-2">
                <h3 className="text-sm font-medium">音频轨</h3>
                {audioStreams.length === 0 ? (
                  <p className="text-muted-foreground text-sm">暂无音频轨信息。</p>
                ) : (
                  <div className="space-y-1.5">
                    {audioStreams.map((stream) => (
                      <div
                        key={`audio-${stream.Index}`}
                        className="flex flex-wrap items-center gap-2 text-sm"
                      >
                        <span>
                          {formatLanguage(stream.Language)}{" "}
                          {(stream.Codec || "unknown").toUpperCase()}{" "}
                          {stream.Channels ? `${stream.Channels}ch` : ""}
                        </span>
                        {stream.BitRate ? (
                          <span className="text-muted-foreground">
                            {formatBitrate(stream.BitRate)}
                          </span>
                        ) : null}
                        {stream.IsDefault ? <Badge variant="success">默认</Badge> : null}
                        <Badge variant="outline">{stream.IsExternal ? "外挂" : "内嵌"}</Badge>
                      </div>
                    ))}
                  </div>
                )}
              </div>

              <div className="space-y-2">
                <h3 className="text-sm font-medium">字幕轨</h3>
                {subtitleItems.length === 0 ? (
                  <p className="text-muted-foreground text-sm">暂无字幕信息。</p>
                ) : (
                  <div className="space-y-1.5">
                    {subtitleItems.map((subtitle) => (
                      <div
                        key={`subtitle-${subtitle.Index}`}
                        className="flex flex-wrap items-center gap-2 text-sm"
                      >
                        <span>
                          {subtitle.DisplayTitle?.trim()
                            ? subtitle.DisplayTitle
                            : `${formatLanguage(subtitle.Language)} ${(subtitle.Codec || "subtitle").toUpperCase()}`}
                        </span>
                        {subtitle.IsDefault ? <Badge variant="success">默认</Badge> : null}
                        <Badge variant="outline">{subtitle.IsExternal ? "外挂" : "内嵌"}</Badge>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          ) : null}
        </section>
      </div>

      <AddToPlaylistModal
        open={playlistModalOpen}
        item={item}
        onClose={() => setPlaylistModalOpen(false)}
      />

      <AdminItemModal
        open={adminModalOpen}
        item={item}
        onClose={() => setAdminModalOpen(false)}
        onSuccess={() => void reloadCurrentItem()}
        onDeleted={() => {
          setItem(null);
          setSeasons([]);
          setEpisodes([]);
          setSelectedSeasonId(null);
        }}
      />

      <PlayerPickerModal
        open={playerPickerOpen}
        onClose={() => setPlayerPickerOpen(false)}
        streamUrl={playerStreamUrl}
        title={item.Name}
      />
    </>
  );
}
