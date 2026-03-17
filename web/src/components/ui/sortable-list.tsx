import React from "react";
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
  item: { id: string; label: string; [key: string]: unknown };
  disabled?: boolean;
}

function SortableItem({ id, item, disabled }: SortableItemProps) {
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
        "bg-card flex items-center gap-2 rounded-md border px-3 py-2 text-sm shadow-sm",
        isDragging && "ring-primary z-10 opacity-50 ring-1",
        disabled && "cursor-not-allowed opacity-50"
      )}
    >
      <button
        type="button"
        className={cn(
          "text-muted-foreground hover:text-foreground touch-none",
          disabled && "cursor-not-allowed"
        )}
        {...attributes}
        {...listeners}
        disabled={disabled}
      >
        <GripVertical className="h-4 w-4" />
      </button>
      <span className="flex-1 truncate font-medium">{item.label}</span>
    </div>
  );
}

interface SortableListProps {
  items: Array<{ id: string; label: string; [key: string]: unknown }>;
  onChange: (items: Array<{ id: string; label: string; [key: string]: unknown }>) => void;
  disabled?: boolean;
  className?: string;
}

export function SortableList({ items, onChange, disabled, className }: SortableListProps) {
  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;

    if (over && active.id !== over.id) {
      const oldIndex = items.findIndex((item) => item.id === active.id);
      const newIndex = items.findIndex((item) => item.id === over.id);
      onChange(arrayMove(items, oldIndex, newIndex));
    }
  };

  return (
    <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
      <SortableContext items={items.map((i) => i.id)} strategy={verticalListSortingStrategy}>
        <div className={cn("space-y-2", className)}>
          {items.map((item) => (
            <SortableItem key={item.id} id={item.id} item={item} disabled={disabled} />
          ))}
          {items.length === 0 && (
            <div className="text-muted-foreground bg-muted/50 rounded-md border border-dashed p-4 text-center text-xs">
              空列表
            </div>
          )}
        </div>
      </SortableContext>
    </DndContext>
  );
}
