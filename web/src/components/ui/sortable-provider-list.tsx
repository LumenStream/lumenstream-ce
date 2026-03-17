import React, { useEffect, useState } from "react";
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import type { DragEndEvent } from "@dnd-kit/core";
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  verticalListSortingStrategy,
  useSortable,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { GripVertical } from "lucide-react";
import { cn } from "@/lib/utils";

interface SortableItemProps {
  id: string;
  item: { id: string; label: string };
  disabled?: boolean;
  isActive: boolean;
  onToggle: (id: string, active: boolean) => void;
}

function SortableItem({ id, item, disabled, isActive, onToggle }: SortableItemProps) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id,
    disabled,
  });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={cn(
        "flex items-center gap-3 rounded-md border px-3 py-2 text-sm shadow-sm transition-colors",
        isActive ? "bg-card border-primary/20" : "bg-muted/30 border-transparent opacity-80",
        isDragging && "bg-card ring-primary z-10 scale-[1.02] opacity-100 ring-2",
        disabled && "cursor-not-allowed opacity-50"
      )}
    >
      <button
        type="button"
        className={cn(
          "hover:bg-muted -ml-1 flex shrink-0 touch-none items-center justify-center rounded p-1",
          disabled
            ? "text-muted-foreground/30 cursor-not-allowed"
            : "text-muted-foreground hover:text-foreground cursor-grab active:cursor-grabbing"
        )}
        {...attributes}
        {...listeners}
        disabled={disabled}
      >
        <GripVertical className="h-4 w-4" />
      </button>

      <span
        className={cn(
          "flex-1 truncate font-medium transition-colors select-none",
          isActive ? "text-foreground" : "text-muted-foreground"
        )}
      >
        {item.label}
      </span>

      <button
        type="button"
        role="switch"
        aria-checked={isActive}
        disabled={disabled}
        onClick={() => onToggle(id, !isActive)}
        className={cn(
          "peer focus-visible:ring-ring focus-visible:ring-offset-background inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50",
          isActive ? "bg-emerald-600 dark:bg-emerald-500" : "bg-slate-300 dark:bg-slate-700"
        )}
      >
        <span
          className={cn(
            "pointer-events-none block h-4 w-4 rounded-full bg-white shadow-lg ring-0 transition-transform",
            isActive ? "translate-x-4" : "translate-x-0"
          )}
        />
      </button>
    </div>
  );
}

export interface ProviderItem {
  id: string;
  label: string;
}

interface SortableProviderListProps {
  providers: ProviderItem[];
  activeIds: string[];
  onChange: (activeIds: string[]) => void;
  disabled?: boolean;
  className?: string;
}

export function SortableProviderList({
  providers,
  activeIds,
  onChange,
  disabled,
  className,
}: SortableProviderListProps) {
  const [orderedIds, setOrderedIds] = useState<string[]>(() => {
    const active = activeIds.filter((id) => providers.some((p) => p.id === id));
    const inactive = providers.map((p) => p.id).filter((id) => !active.includes(id));
    return [...active, ...inactive];
  });

  useEffect(() => {
    const validProviders = providers.map((p) => p.id);
    const missingItems = validProviders.filter((id) => !orderedIds.includes(id));
    const extraItems = orderedIds.filter((id) => !validProviders.includes(id));

    if (missingItems.length > 0 || extraItems.length > 0) {
      const active = activeIds.filter((id) => validProviders.includes(id));
      const inactive = validProviders.filter((id) => !active.includes(id));
      setOrderedIds([...active, ...inactive]);
    }
  }, [activeIds, providers, orderedIds]);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 5,
      },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;

    if (over && active.id !== over.id) {
      const oldIndex = orderedIds.indexOf(active.id as string);
      const newIndex = orderedIds.indexOf(over.id as string);

      if (oldIndex !== -1 && newIndex !== -1) {
        const newOrderedIds = arrayMove(orderedIds, oldIndex, newIndex);
        setOrderedIds(newOrderedIds);

        const newActiveIds = newOrderedIds.filter((id) => activeIds.includes(id));
        onChange(newActiveIds);
      }
    }
  };

  const handleToggle = (id: string, isActive: boolean) => {
    if (isActive) {
      const newActiveIds = orderedIds.filter(
        (itemId) => itemId === id || activeIds.includes(itemId)
      );
      onChange(newActiveIds);
    } else {
      onChange(activeIds.filter((itemId) => itemId !== id));
    }
  };

  return (
    <div className={cn("space-y-2", className)}>
      <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
        <SortableContext items={orderedIds} strategy={verticalListSortingStrategy}>
          <div className="min-h-[42px] space-y-2">
            {orderedIds.map((id) => {
              const item = providers.find((p) => p.id === id);
              if (!item) return null;

              return (
                <SortableItem
                  key={id}
                  id={id}
                  item={item}
                  isActive={activeIds.includes(id)}
                  onToggle={handleToggle}
                  disabled={disabled}
                />
              );
            })}
            {orderedIds.length === 0 && (
              <div className="text-muted-foreground bg-muted/20 rounded-md border border-dashed p-4 text-center text-xs">
                当前无可用刮削器
              </div>
            )}
          </div>
        </SortableContext>
      </DndContext>
    </div>
  );
}
