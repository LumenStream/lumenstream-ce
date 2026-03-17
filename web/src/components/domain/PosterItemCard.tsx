import React, { useCallback, useEffect, useMemo, useState } from "react";

import { AddToPlaylistModal } from "@/components/domain/AddToPlaylistModal";
import { PlayerPickerModal } from "@/components/domain/PlayerPickerModal";
import { Badge } from "@/components/ui/badge";
import {
  addFavoriteItem,
  buildItemImageUrl,
  buildStreamUrl,
  removeFavoriteItem,
} from "@/lib/api/items";
import type { ApiError } from "@/lib/api/client";
import { useImageGlow } from "@/lib/hooks/use-image-glow";
import { formatEpisodeLabel } from "@/lib/media/episode-label";
import { toast } from "@/lib/notifications/toast-store";
import type { BaseItem } from "@/lib/types/jellyfin";

interface PosterItemCardProps {
  item: BaseItem;
  href: string;
  token?: string;
  userId?: string;
  /** Whether to show glow effect (default: true) */
  showGlow?: boolean;
}

const failedImageUrls = new Set<string>();

function formatDuration(runtimeTicks?: number | null): string {
  if (!runtimeTicks) {
    return "--";
  }

  const totalMinutes = Math.max(1, Math.floor(runtimeTicks / 10_000_000 / 60));
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;

  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }

  return `${minutes}m`;
}

function deriveRating(item: BaseItem): string {
  if (typeof item.CommunityRating === "number") {
    return item.CommunityRating.toFixed(1);
  }
  return "--";
}

function formatSeriesEpisodeCount(item: BaseItem): string {
  if (item.Type !== "Series") {
    return formatDuration(item.RunTimeTicks);
  }
  if (typeof item.ChildCount === "number" && item.ChildCount >= 0) {
    return `${item.ChildCount}集`;
  }
  return "剧集";
}

function fallbackGradient(itemId: string): string {
  const palette = [
    ["#9a3412", "#881337"],
    ["#1d4ed8", "#6d28d9"],
    ["#be123c", "#334155"],
    ["#0f766e", "#1d4ed8"],
    ["#7c2d12", "#14532d"],
  ];
  const index = [...itemId].reduce((acc, ch) => acc + ch.charCodeAt(0), 0) % palette.length;
  const pair = palette[index]!;
  return `linear-gradient(135deg, ${pair[0]} 0%, ${pair[1]} 100%)`;
}

export function PosterItemCard({
  item,
  href,
  token,
  userId,
  showGlow = true,
}: PosterItemCardProps) {
  const [isHovered, setIsHovered] = useState(false);
  const [imageError, setImageError] = useState(false);
  const [imageLoaded, setImageLoaded] = useState(false);
  const [isFavoriting, setIsFavoriting] = useState(false);
  const [isFavorite, setIsFavorite] = useState(Boolean(item.UserData?.IsFavorite));
  const [playlistModalOpen, setPlaylistModalOpen] = useState(false);
  const [playerPickerOpen, setPlayerPickerOpen] = useState(false);
  const [playerStreamUrl, setPlayerStreamUrl] = useState("");

  useEffect(() => {
    setIsFavorite(Boolean(item.UserData?.IsFavorite));
  }, [item.Id, item.UserData?.IsFavorite]);

  const imageSrc = useMemo(() => {
    if (item.ImagePrimaryUrl) {
      return item.ImagePrimaryUrl;
    }

    return buildItemImageUrl(item.Id, token);
  }, [item.Id, item.ImagePrimaryUrl, token]);

  useEffect(() => {
    setImageError(failedImageUrls.has(imageSrc));
    setImageLoaded(false);
  }, [imageSrc]);

  const handleImageError = useCallback(() => {
    failedImageUrls.add(imageSrc);
    setImageError(true);
  }, [imageSrc]);

  const { glowColor } = useImageGlow(imageError ? undefined : imageSrc, {
    enabled: showGlow && !imageError && isHovered,
  });

  function handlePlay(event: React.MouseEvent<HTMLButtonElement>) {
    event.preventDefault();
    event.stopPropagation();

    if (!token) {
      toast.warning("缺少登录态，暂时无法生成播放链接");
      return;
    }

    setPlayerStreamUrl(buildStreamUrl(item.Id, token));
    setPlayerPickerOpen(true);
  }

  async function handleFavorite(event: React.MouseEvent<HTMLButtonElement>) {
    event.preventDefault();
    event.stopPropagation();

    if (!userId) {
      toast.warning("缺少用户信息，无法更新喜欢状态");
      return;
    }

    const nextFavorite = !isFavorite;
    setIsFavorite(nextFavorite);
    setIsFavoriting(true);
    try {
      if (nextFavorite) {
        await addFavoriteItem(userId, item.Id);
        toast.success("已加入喜欢");
      } else {
        await removeFavoriteItem(userId, item.Id);
        toast.info("已取消喜欢");
      }
    } catch (cause) {
      setIsFavorite(!nextFavorite);
      const apiError = cause as ApiError;
      toast.error(apiError.message || "更新喜欢状态失败");
    } finally {
      setIsFavoriting(false);
    }
  }

  function handleOpenPlaylist(event: React.MouseEvent<HTMLButtonElement>) {
    event.preventDefault();
    event.stopPropagation();
    setPlaylistModalOpen(true);
  }

  return (
    <>
      <a
        className="relative block w-[170px] shrink-0 focus-within:z-50 hover:z-50 sm:w-[190px]"
        href={href}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
      >
        <div className="group relative block py-4" style={{ contain: "layout style" }}>
          {showGlow && isHovered && (
            <div
              className="pointer-events-none absolute z-0 -translate-y-2 scale-[1.02] animate-[glow-fade-in_300ms_ease-out_forwards]"
              style={{ top: "16px", bottom: "16px", left: "0", right: "0" }}
              aria-hidden="true"
            >
              <div
                className="absolute inset-0"
                style={{
                  backgroundImage: !imageError ? `url(${imageSrc})` : fallbackGradient(item.Id),
                  backgroundSize: "cover",
                  backgroundPosition: "center",
                  borderRadius: "16px",
                  filter: "blur(16px) saturate(130%) brightness(1.1)",
                  transform: "translateZ(0)",
                  opacity: 0.6,
                }}
              />
              <div
                className="absolute inset-0 rounded-xl"
                style={{
                  opacity: 0.45,
                  boxShadow: `0 0 0 1px ${glowColor ?? "rgba(148, 163, 184, 0.3)"}, 0 0 12px 2px ${glowColor ?? "rgba(148, 163, 184, 0.4)"}`,
                }}
              />
            </div>
          )}
          <div
            className="relative z-10 aspect-[2/3] overflow-hidden rounded-xl transition-all duration-300 ease-out group-hover:-translate-y-2 group-hover:scale-[1.02] group-hover:shadow-[0_25px_50px_-12px_rgba(0,0,0,0.6)]"
            style={{ willChange: "transform" }}
          >
            {!imageError ? (
              <img
                src={imageSrc}
                alt={item.Name}
                className={`h-full w-full object-cover transition-all duration-500 ${imageLoaded ? "scale-100 opacity-100" : "scale-105 opacity-0"}`}
                loading="lazy"
                onError={handleImageError}
                onLoad={() => setImageLoaded(true)}
              />
            ) : (
              <div
                className="flex h-full w-full items-end p-3"
                style={{ backgroundImage: fallbackGradient(item.Id) }}
              >
                <p className="line-clamp-3 text-sm font-semibold text-white/90">{item.Name}</p>
              </div>
            )}

            <div className="absolute inset-0 bg-gradient-to-t from-black via-black/40 to-transparent" />

            <div className="pointer-events-none absolute inset-0 z-20 bg-black/0 transition-colors duration-300 group-hover:bg-black/40" />
            <div className="absolute inset-0 z-30 flex items-center justify-center opacity-0 transition-opacity duration-200 group-hover:opacity-100">
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  className="bg-foreground text-background inline-flex h-10 w-10 items-center justify-center rounded-full shadow-lg transition-transform hover:scale-105 disabled:cursor-not-allowed disabled:opacity-60"
                  onClick={handlePlay}
                  aria-label="播放"
                >
                  <svg className="h-4 w-4" viewBox="0 0 20 20" fill="currentColor">
                    <path d="M6 4.5a1 1 0 011.53-.848l8 5a1 1 0 010 1.696l-8 5A1 1 0 016 14.5v-10z" />
                  </svg>
                </button>
                <button
                  type="button"
                  className={`inline-flex h-10 w-10 items-center justify-center rounded-full border shadow-lg transition-transform hover:scale-105 disabled:cursor-not-allowed disabled:opacity-60 ${
                    isFavorite
                      ? "border-rose-300/80 bg-rose-500 text-white"
                      : "border-white/40 bg-white/10 text-white"
                  }`}
                  onClick={(event) => void handleFavorite(event)}
                  disabled={isFavoriting}
                  aria-label="喜欢"
                >
                  <svg className="h-4 w-4" viewBox="0 0 20 20" fill="currentColor">
                    <path d="M3.172 5.172a4 4 0 015.656 0L10 6.343l1.172-1.171a4 4 0 015.656 5.656L10 17.657l-6.828-6.829a4 4 0 010-5.656z" />
                  </svg>
                </button>
                <button
                  type="button"
                  className="inline-flex h-10 w-10 items-center justify-center rounded-full border border-white/40 bg-white/10 text-white shadow-lg transition-transform hover:scale-105"
                  onClick={handleOpenPlaylist}
                  aria-label="添加到列表"
                >
                  <svg className="h-4 w-4" viewBox="0 0 20 20" fill="currentColor">
                    <path d="M3 5a1 1 0 011-1h8a1 1 0 110 2H4a1 1 0 01-1-1zm0 5a1 1 0 011-1h8a1 1 0 110 2H4a1 1 0 01-1-1zm1 4a1 1 0 100 2h5a1 1 0 100-2H4zm11-2a1 1 0 10-2 0v1h-1a1 1 0 100 2h1v1a1 1 0 102 0v-1h1a1 1 0 100-2h-1v-1z" />
                  </svg>
                </button>
              </div>
            </div>

            <div className="absolute top-2 left-2 flex items-center gap-1 rounded-full bg-black/60 px-2 py-0.5 text-[11px] font-medium text-white/80 backdrop-blur-sm">
              <svg className="h-3 w-3" viewBox="0 0 20 20" fill="currentColor">
                <path d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z" />
              </svg>
              {deriveRating(item)}
            </div>

            <div className="absolute top-2 right-2">
              <Badge
                variant="secondary"
                className="border-0 bg-white/15 text-[10px] font-medium text-white/90 backdrop-blur-sm hover:bg-white/20"
              >
                {item.Type === "Movie" ? "电影" : item.Type === "Series" ? "剧集" : item.Type}
              </Badge>
            </div>

            <div className="absolute inset-x-0 bottom-0 space-y-1.5 bg-gradient-to-t from-black/90 via-black/50 to-transparent p-3 pt-8">
              <p className="line-clamp-1 text-[15px] font-semibold tracking-wide text-white/95">
                {formatEpisodeLabel(item)}
              </p>
              <div className="flex items-center justify-between text-[11px] font-medium text-white/60">
                <span className="flex items-center gap-1.5">{item.ProductionYear || "—"}</span>
                <span>{formatSeriesEpisodeCount(item)}</span>
              </div>
            </div>
          </div>
        </div>
      </a>

      <AddToPlaylistModal
        open={playlistModalOpen}
        item={item}
        onClose={() => setPlaylistModalOpen(false)}
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
