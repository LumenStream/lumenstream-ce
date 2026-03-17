import { useEffect, useMemo, useState } from "react";

import { EmptyState, ErrorState, LoadingState } from "@/components/domain/DataState";
import { getRouteParam } from "@/lib/hooks/use-route-param";
import { buildItemImageUrl, buildPersonImageUrl, getPerson, getPersonItems } from "@/lib/api/items";
import type { ApiError } from "@/lib/api/client";
import { useAuthSession } from "@/lib/auth/use-auth-session";
import { resolveMediaItemHref } from "@/lib/media/item-href";
import type { BaseItem } from "@/lib/types/jellyfin";

interface PersonDetailProps {
  personId?: string;
}

function roleTextForItem(item: BaseItem, personId: string, personName: string): string | null {
  const people = item.People || [];
  const byId = people.find((person) => (person.Id || "").trim() === personId);
  if (byId?.Role && byId.Role.trim().length > 0) {
    return byId.Role.trim();
  }

  const byName = people.find(
    (person) => person.Name.trim().toLowerCase() === personName.toLowerCase()
  );
  if (byName?.Role && byName.Role.trim().length > 0) {
    return byName.Role.trim();
  }

  return null;
}

function metaForItem(item: BaseItem): string {
  const year = item.ProductionYear ? String(item.ProductionYear) : "年份未知";
  return `${year} · ${item.Type}`;
}

function roleLabelForItem(item: BaseItem, personId: string, personName: string): string {
  const role = roleTextForItem(item, personId, personName);
  if (!role) {
    return "角色待补充";
  }
  return `饰演 ${role}`;
}

export function PersonDetail({ personId: personIdProp }: PersonDetailProps) {
  const personId = personIdProp || getRouteParam("person");
  const { ready, session } = useAuthSession();
  const [person, setPerson] = useState<BaseItem | null>(null);
  const [items, setItems] = useState<BaseItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [avatarFailed, setAvatarFailed] = useState(false);
  const [posterFailures, setPosterFailures] = useState<Record<string, true>>({});

  useEffect(() => {
    if (!ready || !session) {
      return;
    }

    let cancelled = false;
    setLoading(true);

    Promise.all([
      getPerson(personId),
      getPersonItems(personId, {
        includeItemTypes: "Movie,Series,Episode",
        limit: 200,
        startIndex: 0,
      }),
    ])
      .then(([personResult, creditsResult]) => {
        if (cancelled) {
          return;
        }

        setPerson(personResult);
        setItems(creditsResult.Items);
        setAvatarFailed(false);
        setPosterFailures({});
        setError(null);
      })
      .catch((cause) => {
        if (cancelled) {
          return;
        }

        const apiError = cause as ApiError;
        setError(apiError.message || "加载人物详情失败");
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [personId, ready, session]);

  const sortedCredits = useMemo(
    () =>
      [...items].sort((left, right) => {
        const yearL = left.ProductionYear || 0;
        const yearR = right.ProductionYear || 0;
        if (yearL !== yearR) {
          return yearR - yearL;
        }
        return left.Name.localeCompare(right.Name);
      }),
    [items]
  );

  if (!ready || loading) {
    return <LoadingState title="加载人物详情中..." />;
  }

  if (!session) {
    return <ErrorState title="无法加载人物详情" description="登录态已失效，请重新登录后重试。" />;
  }

  if (error) {
    return <ErrorState title="人物详情加载失败" description={error} />;
  }

  if (!person) {
    return <EmptyState title="找不到人物" description="该人物可能不存在或已被删除。" />;
  }

  const token = session.token;
  const personImageUrl = buildPersonImageUrl(person.Id, token);
  const searchParams = new URLSearchParams({
    personId: person.Id,
    personName: person.Name,
    type: "Movie,Series",
  });

  return (
    <div className="space-y-8">
      <section className="flex flex-col gap-5 rounded-2xl border border-white/10 bg-black/35 p-5 md:flex-row md:items-start">
        <div className="mx-auto w-[140px] shrink-0 md:mx-0">
          {!avatarFailed ? (
            <img
              src={personImageUrl}
              alt={person.Name}
              className="aspect-[2/3] w-full rounded-xl object-cover"
              onError={() => setAvatarFailed(true)}
            />
          ) : (
            <div className="flex aspect-[2/3] w-full items-center justify-center rounded-xl bg-neutral-800 text-2xl text-neutral-300">
              {person.Name.slice(0, 2)}
            </div>
          )}
        </div>

        <div className="space-y-3 text-center md:text-left">
          <p className="text-xs tracking-[0.14em] text-white/55 uppercase">Person</p>
          <h1 className="text-3xl font-semibold tracking-tight md:text-4xl">{person.Name}</h1>
          <p className="max-w-3xl text-sm leading-relaxed text-white/72">
            {person.Overview || "暂无人物简介。"}
          </p>
          <a
            href={`/app/search?${searchParams.toString()}`}
            className="inline-flex rounded-full border border-white/20 bg-white/6 px-3 py-1 text-xs text-white/86 transition-colors hover:bg-white/14"
          >
            在搜索中查看全部相关条目
          </a>
        </div>
      </section>

      <section className="space-y-4">
        <h2 className="text-2xl font-semibold tracking-tight">参演作品</h2>
        {sortedCredits.length === 0 ? (
          <p className="text-muted-foreground text-sm">暂无该人物关联作品。</p>
        ) : (
          <div
            className="scrollbar-hide flex gap-4 overflow-x-auto pb-2"
            style={{ overscrollBehaviorX: "contain" }}
          >
            {sortedCredits.map((item) => {
              const itemKey = item.Id;
              const posterUrl = buildItemImageUrl(item.Id, token);
              const posterFailed = Boolean(posterFailures[itemKey]);
              return (
                <a
                  key={item.Id}
                  href={resolveMediaItemHref(item)}
                  className="group w-[150px] shrink-0 space-y-2 text-center"
                >
                  <div className="relative overflow-hidden rounded-lg transition-transform group-hover:scale-[1.03]">
                    {!posterFailed ? (
                      <img
                        src={posterUrl}
                        alt={item.Name}
                        className="aspect-[2/3] w-full object-cover"
                        onError={() => {
                          setPosterFailures((prev) => ({
                            ...prev,
                            [itemKey]: true,
                          }));
                        }}
                      />
                    ) : (
                      <div className="flex aspect-[2/3] items-center justify-center bg-neutral-800 text-sm text-neutral-400">
                        {item.Name.slice(0, 2)}
                      </div>
                    )}
                  </div>

                  <p className="line-clamp-2 text-sm text-white group-hover:underline">
                    {item.Name}
                  </p>
                  <p className="text-muted-foreground text-xs">{metaForItem(item)}</p>
                  <p className="text-xs text-white/75">
                    {roleLabelForItem(item, person.Id, person.Name)}
                  </p>
                </a>
              );
            })}
          </div>
        )}
      </section>
    </div>
  );
}
