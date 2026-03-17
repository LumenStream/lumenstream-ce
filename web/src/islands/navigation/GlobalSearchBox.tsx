import React, { useCallback, useEffect, useId, useRef, useState } from "react";

interface GlobalSearchBoxProps {
  onNavigate?: (href: string) => void;
}

export function GlobalSearchBox({ onNavigate }: GlobalSearchBoxProps) {
  const [searchTerm, setSearchTerm] = useState("");
  const [includeType, setIncludeType] = useState<"" | "Movie" | "Series">("");
  const inputRef = useRef<HTMLInputElement>(null);
  const searchInputId = useId();
  const typeSelectId = useId();

  // Keyboard shortcuts: Cmd/Ctrl + K to focus
  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if ((event.metaKey || event.ctrlKey) && event.key === "k") {
        event.preventDefault();
        inputRef.current?.focus();
      }
    }

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, []);

  const handleSearch = useCallback(
    (term: string, type: "" | "Movie" | "Series") => {
      const params = new URLSearchParams();
      if (term.trim()) params.set("q", term.trim());
      if (type) params.set("type", type);

      const href = `/app/search?${params.toString()}`;
      if (onNavigate) {
        onNavigate(href);
      } else {
        window.location.href = href;
      }
    },
    [onNavigate]
  );

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    handleSearch(searchTerm, includeType);
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setSearchTerm(e.target.value);
  };

  return (
    <div className="relative w-full max-w-2xl">
      <form onSubmit={handleSubmit} className="relative" role="search" aria-label="全局搜索">
        <div className="group flex items-center gap-2 rounded-full border border-white/12 bg-black/45 py-1.5 pr-2.5 pl-3 text-white/85 backdrop-blur-md transition-all duration-200 ease-out focus-within:border-white/28 focus-within:bg-black/60 hover:border-white/24 hover:bg-black/55">
          <div className="flex-shrink-0 text-white/55 transition-colors group-focus-within:text-white/90">
            <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
              />
            </svg>
          </div>

          <label htmlFor={searchInputId} className="sr-only">
            搜索关键词
          </label>
          <input
            id={searchInputId}
            name="q"
            ref={inputRef}
            type="text"
            value={searchTerm}
            onChange={handleInputChange}
            placeholder="搜索影片、剧集、演员"
            autoComplete="off"
            className="min-w-0 flex-1 bg-transparent py-1.5 text-sm text-white/92 placeholder:text-white/45 focus:outline-none"
          />

          <span className="hidden rounded-full border border-white/20 px-2 py-0.5 text-[10px] font-medium tracking-wide text-white/65 lg:inline-flex">
            ⌘K
          </span>

          <div className="hidden h-4 w-px bg-white/20 sm:block" />

          <label htmlFor={typeSelectId} className="sr-only">
            媒体类型
          </label>
          <select
            id={typeSelectId}
            name="type"
            value={includeType}
            onChange={(e) => setIncludeType(e.target.value as "" | "Movie" | "Series")}
            className="hidden flex-shrink-0 cursor-pointer appearance-none bg-transparent py-1 pr-1 pl-1 text-center text-xs text-white/65 transition-colors hover:text-white/90 focus:outline-none sm:block"
          >
            <option value="" className="bg-black text-white">
              全部
            </option>
            <option value="Movie" className="bg-black text-white">
              电影
            </option>
            <option value="Series" className="bg-black text-white">
              剧集
            </option>
          </select>

          <button
            type="submit"
            disabled={!searchTerm.trim()}
            aria-label="执行搜索"
            className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full bg-white/14 text-white/95 transition-all duration-200 hover:bg-white/22 disabled:cursor-not-allowed disabled:opacity-50"
          >
            <svg className="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
              />
            </svg>
          </button>
        </div>
      </form>
    </div>
  );
}
