interface DataStateProps {
  title: string;
  description?: string;
}

function SkeletonPulse({ className }: { className?: string }) {
  return (
    <div
      className={`from-muted/50 via-muted to-muted/50 animate-pulse bg-gradient-to-r bg-[length:200%_100%] ${className}`}
      style={{ animation: "shimmer 2s infinite" }}
    />
  );
}

export function LoadingState({
  title = "加载中...",
  description,
}: Partial<{ title: string; description?: string }>) {
  return (
    <div className="space-y-6">
      <div className="sr-only">
        {title}
        {description ? `，${description}` : ""}
      </div>
      {/* Hero Skeleton */}
      <section className="space-y-3">
        <div className="relative overflow-hidden rounded-2xl p-6 sm:p-10">
          <SkeletonPulse className="absolute inset-0 opacity-30" />
          <div className="relative space-y-5">
            <div className="flex items-center gap-2">
              <SkeletonPulse className="h-5 w-32 rounded-full" />
              <SkeletonPulse className="h-5 w-24 rounded-full" />
            </div>
            <div className="space-y-2">
              <SkeletonPulse className="h-10 w-3/4 max-w-md rounded-lg" />
              <SkeletonPulse className="h-4 w-2/3 max-w-sm rounded" />
              <SkeletonPulse className="h-4 w-1/2 max-w-xs rounded" />
            </div>
            <div className="flex gap-2">
              <SkeletonPulse className="h-6 w-20 rounded-full" />
              <SkeletonPulse className="h-6 w-20 rounded-full" />
              <SkeletonPulse className="h-6 w-20 rounded-full" />
            </div>
            <div className="flex gap-2 pt-2">
              <SkeletonPulse className="h-10 w-28 rounded-md" />
              <SkeletonPulse className="h-10 w-28 rounded-md" />
            </div>
          </div>
        </div>

        {/* Thumbnail Row Skeleton */}
        <div className="flex gap-3">
          {Array.from({ length: 6 }).map((_, i) => (
            <div key={i} className="w-[122px] shrink-0 overflow-hidden rounded-xl">
              <SkeletonPulse className="aspect-[2/3] w-full rounded-xl" />
            </div>
          ))}
        </div>
      </section>

      {/* Stats Card Skeleton */}
      <div className="space-y-4">
        <div className="space-y-2">
          <SkeletonPulse className="h-6 w-24 rounded" />
          <SkeletonPulse className="h-4 w-48 rounded" />
        </div>
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <div key={i} className="rounded-xl p-4">
              <SkeletonPulse className="h-3 w-16 rounded" />
              <SkeletonPulse className="mt-2 h-8 w-12 rounded" />
              <SkeletonPulse className="mt-1 h-3 w-12 rounded" />
            </div>
          ))}
        </div>
      </div>

      {/* Poster Rows Skeleton */}
      {Array.from({ length: 3 }).map((_, rowIndex) => (
        <section key={rowIndex} className="space-y-3">
          <div className="flex items-end justify-between gap-3">
            <div className="space-y-1">
              <SkeletonPulse className="h-6 w-32 rounded" />
              <SkeletonPulse className="h-3 w-20 rounded" />
            </div>
            <SkeletonPulse className="h-5 w-16 rounded-full" />
          </div>
          <div className="flex gap-3 overflow-hidden">
            {Array.from({ length: 8 }).map((_, i) => (
              <div key={i} className="w-[170px] shrink-0 sm:w-[190px]">
                <SkeletonPulse className="aspect-[2/3] w-full rounded-xl" />
              </div>
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}

export function EmptyState({ title, description }: { title: string; description?: string }) {
  return (
    <div className="flex flex-col items-center justify-center rounded-xl px-6 py-12 text-center">
      <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-white/5">
        <svg
          className="text-muted-foreground h-6 w-6"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={1.5}
            d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"
          />
        </svg>
      </div>
      <p className="text-foreground/80 text-sm font-medium">{title}</p>
      {description ? <p className="text-muted-foreground mt-1 text-xs">{description}</p> : null}
    </div>
  );
}

export function ErrorState({ title, description }: DataStateProps) {
  return (
    <div className="rounded-xl bg-rose-500/[0.06] px-6 py-4">
      <p className="text-rose-300">{title}</p>
      {description ? <p className="mt-1 text-sm text-rose-200/70">{description}</p> : null}
    </div>
  );
}
