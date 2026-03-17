import type { BaseItem } from "@/lib/types/jellyfin";

type ItemLinkTarget = Pick<BaseItem, "Id" | "Type" | "Name">;

export function resolveMediaItemHref(item: ItemLinkTarget): string {
  if (item.Type === "CollectionFolder" || item.Type === "Series") {
    return `/app/library/${item.Id}`;
  }

  if (item.Type === "Person") {
    return `/app/person/${item.Id}`;
  }

  return `/app/item/${item.Id}`;
}
