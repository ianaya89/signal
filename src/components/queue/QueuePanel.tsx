import { useRef, useState } from "react";

import { api } from "@/ipc/invoke";
import { fmtDuration } from "@/lib/format";
import { cn } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";
import { useQueueStore } from "@/stores/queueStore";

export function QueuePanel() {
  const entries = useQueueStore((s) => s.entries);
  const playing = usePlayerStore((s) => s.status === "playing");
  const dragIndex = useRef<number | null>(null);
  const [dragOver, setDragOver] = useState<number | null>(null);

  if (entries.length === 0) {
    return (
      <p className="px-2 py-2 text-[11px] text-muted">
        queue empty — press <kbd className="rounded-sm bg-raised px-1">a</kbd> on
        a track to stage it
      </p>
    );
  }

  const move = (index: number, delta: number) => {
    const next = [...entries];
    const target = index + delta;
    if (target < 0 || target >= next.length) return;
    const item = next[index];
    if (!item) return;
    next.splice(index, 1);
    next.splice(target, 0, item);
    void api.queueMove(next.map((e) => e.item.id));
  };

  return (
    <div className="flex flex-col">
      <div className="flex h-6 items-center justify-between px-2 text-[10px] text-muted">
        <span>{entries.length} staged</span>
        <div className="flex gap-2">
          {!playing && entries.length > 0 && (
            <button
              type="button"
              onClick={() => void api.queuePlayNext()}
              className="hover:text-accent"
            >
              play
            </button>
          )}
          <button
            type="button"
            onClick={() => {
              const name = prompt("playlist name:");
              if (name?.trim()) void api.queueSaveAsPlaylist(name.trim());
            }}
            title="save queue as playlist (w)"
            className="hover:text-accent"
          >
            save
          </button>
          <button
            type="button"
            onClick={() => void api.queueClear()}
            className="hover:text-error"
          >
            clear
          </button>
        </div>
      </div>
      <ul>
        {entries.map((entry, i) => (
          <li
            key={entry.item.id}
            draggable
            onDragStart={() => {
              dragIndex.current = i;
            }}
            onDragOver={(e) => {
              e.preventDefault();
              setDragOver(i);
            }}
            onDragLeave={() => setDragOver((d) => (d === i ? null : d))}
            onDrop={(e) => {
              e.preventDefault();
              const from = dragIndex.current;
              dragIndex.current = null;
              setDragOver(null);
              if (from === null || from === i) return;
              const next = [...entries];
              const moved = next.splice(from, 1)[0];
              if (!moved) return;
              next.splice(i, 0, moved);
              void api.queueMove(next.map((x) => x.item.id));
            }}
            onDragEnd={() => {
              dragIndex.current = null;
              setDragOver(null);
            }}
            className={cn(
              "group flex h-6 cursor-grab items-center gap-2 px-2 hover:bg-raised active:cursor-grabbing",
              dragOver === i ? "border-t border-accent" : undefined,
            )}
          >
            <span className="w-4 shrink-0 text-right text-[10px] text-muted">
              {i + 1}
            </span>
            <span className="min-w-0 flex-1 truncate text-[11px] text-secondary">
              {entry.track.title}
            </span>
            <span className="shrink-0 text-[10px] text-muted">
              {fmtDuration(entry.track.durationMs)}
            </span>
            <span className="hidden shrink-0 gap-1 text-[10px] group-hover:flex">
              <button
                type="button"
                onClick={() => move(i, -1)}
                className="text-muted hover:text-accent"
              >
                ↑
              </button>
              <button
                type="button"
                onClick={() => move(i, 1)}
                className="text-muted hover:text-accent"
              >
                ↓
              </button>
              <button
                type="button"
                onClick={() => void api.queueRemove(entry.item.id)}
                className="text-muted hover:text-error"
              >
                ✕
              </button>
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}
