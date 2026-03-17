import { useCallback, useEffect, useRef, useState } from "react";

import { EmptyState } from "@/components/domain/DataState";
import { PosterItemCard } from "@/components/domain/PosterItemCard";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { getItems } from "@/lib/api/items";
import type { ApiError } from "@/lib/api/client";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import { resolveMediaItemHref } from "@/lib/media/item-href";
import { toast } from "@/lib/notifications/toast-store";
import type { BaseItem } from "@/lib/types/jellyfin";

interface SearchCenterProps {
  initialQuery?: string;
  initialType?: string;
  initialPersonId?: string;
  initialPersonName?: string;
}

interface SearchSuggestion {
  item: BaseItem;
}

const DEBOUNCE_MS = 300;
const DEFAULT_INCLUDE_TYPES = "Movie,Series,Person";

const TYPE_OPTIONS = [
  { label: "全部", value: DEFAULT_INCLUDE_TYPES },
  { label: "电影", value: "Movie" },
  { label: "剧集", value: "Series" },
  { label: "演职员", value: "Person" },
];

function readSearchParam(key: string, fallback: string): string {
  if (typeof window === "undefined") return fallback;
  return new URLSearchParams(window.location.search).get(key) || fallback;
}

export function SearchCenter({
  initialQuery,
  initialType,
  initialPersonId,
  initialPersonName,
}: SearchCenterProps) {
  const { session, ready } = useAuthSession();
  const resolvedQuery = initialQuery ?? readSearchParam("q", "");
  const resolvedType = initialType ?? readSearchParam("type", "");
  const resolvedPersonId = initialPersonId ?? readSearchParam("personId", "");
  const resolvedPersonName = initialPersonName ?? readSearchParam("personName", "");
  const normalizedInitialPersonId = resolvedPersonId.trim();
  const normalizedInitialPersonName = resolvedPersonName.trim();
  const resolvedInitialType = resolvedType || DEFAULT_INCLUDE_TYPES;

  const [searchTerm, setSearchTerm] = useState(resolvedQuery);
  const [includeType, setIncludeType] = useState<string>(resolvedInitialType);
  const [personFilterId, setPersonFilterId] = useState(normalizedInitialPersonId);
  const [personFilterName, setPersonFilterName] = useState(normalizedInitialPersonName);
  const [suggestions, setSuggestions] = useState<SearchSuggestion[]>([]);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [items, setItems] = useState<BaseItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [suggestionLoading, setSuggestionLoading] = useState(false);
  const [hasSearched, setHasSearched] = useState(false);
  const [highlightedIndex, setHighlightedIndex] = useState(-1);
  const formRef = useRef<HTMLFormElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const suggestionsRef = useRef<HTMLDivElement>(null);
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const initialSearchTriggeredRef = useRef(false);

  // Auto-focus input on mount
  useEffect(() => {
    if (ready && session) {
      inputRef.current?.focus();
    }
  }, [ready, session]);

  // Click outside to close suggestions
  useEffect(() => {
    if (!showSuggestions) return;

    function handleClickOutside(event: MouseEvent) {
      const target = event.target as Node;
      if (formRef.current?.contains(target)) {
        return;
      }
      setShowSuggestions(false);
    }

    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [showSuggestions]);

  const fetchSuggestions = useCallback(
    async (term: string, type: string, personId: string) => {
      if (!term.trim() || !session) {
        setSuggestions([]);
        return;
      }

      setSuggestionLoading(true);
      try {
        const result = await getItems({
          searchTerm: term,
          includeItemTypes: type || undefined,
          personIds: personId || undefined,
          limit: 5,
          startIndex: 0,
        });
        setSuggestions(result.Items.map((item) => ({ item })));
      } catch {
        setSuggestions([]);
      } finally {
        setSuggestionLoading(false);
      }
    },
    [session]
  );

  const runFullSearch = useCallback(
    async (term: string, type: string, personId?: string) => {
      if (!session) {
        return;
      }

      const normalizedTerm = term.trim();
      const normalizedPersonId = (personId || "").trim();
      if (!normalizedTerm && !normalizedPersonId) {
        toast.warning("请输入搜索关键词");
        return;
      }

      setLoading(true);
      setHasSearched(true);
      setShowSuggestions(false);

      try {
        const result = await getItems({
          searchTerm: normalizedTerm || undefined,
          includeItemTypes: type || DEFAULT_INCLUDE_TYPES,
          personIds: normalizedPersonId || undefined,
          limit: 60,
          startIndex: 0,
        });
        setItems(result.Items);
      } catch (cause) {
        const apiError = cause as ApiError;
        toast.error(apiError.message || "搜索失败");
        setItems([]);
      } finally {
        setLoading(false);
      }
    },
    [session]
  );

  // Initial search if URL has query/person
  useEffect(() => {
    if (!ready || !session || initialSearchTriggeredRef.current) {
      return;
    }

    const initialTerm = resolvedQuery.trim();
    if (!initialTerm && !normalizedInitialPersonId) {
      return;
    }

    initialSearchTriggeredRef.current = true;
    void runFullSearch(initialTerm, resolvedInitialType, normalizedInitialPersonId || undefined);
  }, [
    ready,
    session,
    resolvedQuery,
    normalizedInitialPersonId,
    resolvedInitialType,
    runFullSearch,
  ]);

  // Debounced suggestions
  useEffect(() => {
    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
    }

    if (!searchTerm.trim()) {
      setSuggestions([]);
      return;
    }

    debounceTimerRef.current = setTimeout(() => {
      void fetchSuggestions(searchTerm, includeType, personFilterId);
    }, DEBOUNCE_MS);

    return () => {
      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
      }
    };
  }, [searchTerm, includeType, personFilterId, fetchSuggestions]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    runFullSearch(searchTerm, includeType, personFilterId || undefined);
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setSearchTerm(e.target.value);
    setShowSuggestions(true);
    setHighlightedIndex(-1);
    setHasSearched(false);
  };

  const handleSuggestionClick = (suggestion: SearchSuggestion) => {
    setShowSuggestions(false);
    window.location.href = resolveMediaItemHref(suggestion.item);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setHighlightedIndex((prev) => (prev + 1 < suggestions.length ? prev + 1 : 0));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setHighlightedIndex((prev) => (prev - 1 >= 0 ? prev - 1 : suggestions.length - 1));
    } else if (e.key === "Enter") {
      if (highlightedIndex >= 0 && suggestions[highlightedIndex]) {
        e.preventDefault();
        handleSuggestionClick(suggestions[highlightedIndex]);
      } else {
        // Let form submit handle it
        setShowSuggestions(false);
      }
    } else if (e.key === "Escape") {
      setShowSuggestions(false);
    }
  };

  const clearPersonFilter = () => {
    setPersonFilterId("");
    setPersonFilterName("");

    if (searchTerm.trim()) {
      void runFullSearch(searchTerm, includeType, undefined);
      return;
    }

    setItems([]);
    setHasSearched(false);
  };

  if (!ready || !session) {
    return (
      <div className="flex min-h-[60vh] items-center justify-center">
        <div className="border-muted-foreground/30 border-t-muted-foreground h-6 w-6 animate-spin rounded-full border-2" />
      </div>
    );
  }

  const showResults = hasSearched && !loading;

  return (
    <div className="space-y-10">
      <div className="flex flex-col items-center pt-6 pb-4">
        <div className="mb-7 text-center">
          <p className="light:text-foreground/45 text-xs tracking-[0.16em] text-white/55 uppercase">
            Search
          </p>
          <h1 className="light:text-foreground mt-2 text-3xl font-semibold tracking-tight text-white/92 sm:text-4xl">
            发现你想看的内容
          </h1>
          <p className="light:text-foreground/50 mt-2 text-sm text-white/58">
            搜索电影、剧集和演职员
          </p>
        </div>

        <div className="relative w-full max-w-2xl">
          <form ref={formRef} onSubmit={handleSubmit} className="relative">
            <div className="light:border-black/[0.1] light:bg-white/80 light:text-foreground light:focus-within:border-black/20 light:focus-within:bg-white/90 light:hover:border-black/15 light:hover:bg-white/85 flex items-center gap-2 rounded-full border border-white/[0.12] bg-black/45 py-1.5 pr-2 pl-3 text-white/85 backdrop-blur-md transition-all duration-200 ease-out focus-within:border-white/30 focus-within:bg-black/60 hover:border-white/24 hover:bg-black/55">
              <div className="light:text-foreground/45 flex-shrink-0 text-white/55">
                <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                  />
                </svg>
              </div>

              <input
                ref={inputRef}
                type="text"
                value={searchTerm}
                onChange={handleInputChange}
                onKeyDown={handleKeyDown}
                placeholder="输入片名、剧名、演职员..."
                className="light:text-foreground light:placeholder:text-foreground/40 min-w-0 flex-1 bg-transparent text-base text-white/92 placeholder:text-white/45 focus:outline-none"
              />

              {suggestionLoading && (
                <div className="border-muted-foreground/30 border-t-muted-foreground h-4 w-4 flex-shrink-0 animate-spin rounded-full border-2" />
              )}

              {searchTerm && (
                <button
                  type="button"
                  onClick={() => {
                    setSearchTerm("");
                    setSuggestions([]);
                    inputRef.current?.focus();
                  }}
                  className="light:text-foreground/45 light:hover:text-foreground/80 flex-shrink-0 text-white/55 transition-colors hover:text-white/90"
                >
                  <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M6 18L18 6M6 6l12 12"
                    />
                  </svg>
                </button>
              )}

              <div className="light:bg-black/15 h-4 w-px bg-white/20" />

              <select
                value={includeType}
                onChange={(e) => setIncludeType(e.target.value)}
                className="light:text-foreground/60 light:hover:text-foreground flex-shrink-0 cursor-pointer appearance-none bg-transparent py-1 pr-1 pl-1 text-center text-xs text-white/70 transition-colors hover:text-white focus:outline-none"
              >
                {TYPE_OPTIONS.map((option) => (
                  <option
                    key={option.value || "all"}
                    value={option.value}
                    className="bg-black text-white"
                  >
                    {option.label}
                  </option>
                ))}
              </select>

              <button
                type="submit"
                disabled={loading || (!searchTerm.trim() && !personFilterId)}
                className="light:bg-black/8 light:text-foreground/80 light:hover:bg-black/14 flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full bg-white/14 text-white/95 transition-all duration-200 hover:bg-white/22 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {loading ? (
                  <span className="border-primary-foreground/30 border-t-primary-foreground h-4 w-4 animate-spin rounded-full border-2" />
                ) : (
                  <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                    />
                  </svg>
                )}
              </button>
            </div>

            {/* Suggestions Dropdown */}
            {showSuggestions && searchTerm.trim() && (
              <div
                ref={suggestionsRef}
                className="light:border-black/[0.08] light:bg-white/95 light:shadow-black/10 absolute top-full right-0 left-0 z-50 mt-2 overflow-hidden rounded-2xl border border-white/[0.1] bg-neutral-900/92 shadow-2xl shadow-black/35 backdrop-blur-2xl"
              >
                {suggestions.length > 0 ? (
                  <div className="py-2">
                    <div className="text-muted-foreground px-4 py-1.5 text-xs font-medium tracking-wider uppercase">
                      建议结果
                    </div>
                    {suggestions.map((suggestion, index) => (
                      <button
                        key={suggestion.item.Id}
                        type="button"
                        onClick={() => handleSuggestionClick(suggestion)}
                        className={`light:hover:bg-black/[0.04] flex w-full items-center gap-3 px-4 py-2.5 text-left transition-colors hover:bg-white/7 ${highlightedIndex === index ? "light:bg-black/[0.06] bg-white/10" : ""} `}
                      >
                        {/* Type Icon */}
                        <div className="text-muted-foreground/50">
                          {suggestion.item.Type === "Movie" ? (
                            <svg className="h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
                              <path d="M18 4l2 4h-3l-2-4h-2l2 4h-3l-2-4H8l2 4H7L5 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V4h-4z" />
                            </svg>
                          ) : suggestion.item.Type === "Series" ? (
                            <svg className="h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
                              <path d="M4 6h4v2H4zm0 5h4v2H4zm0 5h4v2H4zm6-10h10v2H10zm0 5h10v2H10zm0 5h10v2H10z" />
                            </svg>
                          ) : (
                            <svg className="h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
                              <path d="M9 11.24V7.5C9 6.12 10.12 5 11.5 5S14 6.12 14 7.5v3.74c1.21-.81 2-2.18 2-3.74C16 5.01 13.99 3 11.5 3S7 5.01 7 7.5c0 1.56.79 2.93 2 3.74zm9.04 4.88c-.92-.53-1.96-.88-3.04-.88-1.53 0-2.93.64-3.93 1.66l-2.42-2.42C8.27 11.31 6.91 10 5 10c-2.21 0-4 1.79-4 4s1.79 4 4 4c1.53 0 2.93-.64 3.93-1.66l2.42 2.42c-1 1.02-2.4 1.66-3.93 1.66-1.08 0-2.12-.35-3.04-.88l-1.72 1.72C6.56 20.42 8.71 21 11 21c2.29 0 4.44-.58 6.32-1.67l-1.72-1.72z" />
                            </svg>
                          )}
                        </div>

                        {/* Title */}
                        <div className="min-w-0 flex-1">
                          <span className="text-foreground/90 truncate text-sm">
                            {suggestion.item.Name}
                          </span>
                          {suggestion.item.ProductionYear && (
                            <span className="text-muted-foreground ml-2 text-xs">
                              ({suggestion.item.ProductionYear})
                            </span>
                          )}
                        </div>

                        {/* Arrow */}
                        <svg
                          className="text-muted-foreground/30 h-4 w-4"
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
                      </button>
                    ))}
                  </div>
                ) : !suggestionLoading ? (
                  <div className="px-4 py-6 text-center">
                    <p className="text-muted-foreground text-sm">未找到相关结果</p>
                  </div>
                ) : null}
              </div>
            )}
          </form>

          {personFilterId && (
            <div className="mt-3 flex flex-wrap items-center justify-center gap-2">
              <Badge variant="glass">人物筛选: {personFilterName || personFilterId}</Badge>
              <Button type="button" variant="glass" size="sm" onClick={clearPersonFilter}>
                清除人物筛选
              </Button>
            </div>
          )}
        </div>
      </div>

      {/* Results Section */}
      {showResults && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="light:text-foreground text-lg font-medium text-white/90">
              {personFilterId ? `${personFilterName || "该人物"} 相关作品` : "搜索结果"}
              {items.length > 0 && (
                <span className="ml-2 text-sm text-white/45">({items.length})</span>
              )}
            </h2>
            <Badge variant="glass">{items.length} 条</Badge>
          </div>

          {items.length === 0 ? (
            <EmptyState
              title="暂无结果"
              description={
                personFilterId
                  ? "该人物当前没有匹配到电影或剧集，尝试清除人物筛选后继续搜索"
                  : "尝试更换关键词或类型后重新搜索"
              }
            />
          ) : (
            <div className="flex flex-wrap gap-4">
              {items.map((item) => (
                <PosterItemCard
                  key={item.Id}
                  item={item}
                  href={resolveMediaItemHref(item)}
                  token={session.token}
                  userId={session.user.Id}
                />
              ))}
            </div>
          )}
        </div>
      )}

      {/* Initial Empty State */}
      {!showResults && !loading && !hasSearched && (
        <div className="flex flex-col items-center justify-center py-16 text-center">
          <div className="light:border-black/10 light:bg-black/[0.04] mb-4 flex h-16 w-16 items-center justify-center rounded-full border border-white/12 bg-white/[0.04]">
            <svg
              className="light:text-foreground/40 h-8 w-8 text-white/50"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1.5}
                d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
              />
            </svg>
          </div>
          <p className="light:text-foreground/50 text-sm text-white/62">输入关键词开始搜索</p>
          <p className="light:text-foreground/35 mt-1 text-xs text-white/40">
            支持片名、人名、拼音首字母搜索
          </p>
        </div>
      )}
    </div>
  );
}
