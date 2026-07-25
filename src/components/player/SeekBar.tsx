import { useRef, useState } from "react";

import { api } from "@/ipc/invoke";
import { fmtDuration } from "@/lib/format";
import { cn } from "@/lib/utils";

/** Draggable seek bar: optimistic position while dragging (single seek on
 *  release), 250ms linear fill between progress ticks, hover thumb and a
 *  time bubble while scrubbing. */
export function SeekBar({
  positionMs,
  durationMs,
  className,
  thick = false,
}: {
  positionMs: number;
  durationMs: number;
  className?: string;
  thick?: boolean;
}) {
  const barRef = useRef<HTMLDivElement>(null);
  const [dragFrac, setDragFrac] = useState<number | null>(null);
  const dragRef = useRef<number | null>(null);

  const fracFromEvent = (clientX: number) => {
    const bar = barRef.current;
    if (!bar) return 0;
    const rect = bar.getBoundingClientRect();
    return Math.min(Math.max((clientX - rect.left) / rect.width, 0), 1);
  };

  const beginDrag = (e: React.MouseEvent) => {
    if (durationMs === 0 || e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    const frac = fracFromEvent(e.clientX);
    dragRef.current = frac;
    setDragFrac(frac);

    const onMove = (ev: MouseEvent) => {
      const f = fracFromEvent(ev.clientX);
      dragRef.current = f;
      setDragFrac(f);
    };
    const onUp = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      const f = dragRef.current;
      dragRef.current = null;
      setDragFrac(null);
      if (f !== null) {
        void api.seek(Math.round(f * durationMs));
      }
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  };

  // suppress the width transition while dragging for 1:1 tracking
  const dragging = dragFrac !== null;
  const frac =
    dragFrac ?? (durationMs > 0 ? positionMs / durationMs : 0);

  return (
    <div
      ref={barRef}
      data-no-drag
      onMouseDown={beginDrag}
      className={cn(
        "group/seek relative select-none",
        thick ? "h-2" : "h-4",
        durationMs > 0 ? "cursor-pointer" : "cursor-default",
        className,
      )}
    >
      <div
        className={cn(
          "absolute w-full bg-subtle",
          thick ? "inset-y-0" : "top-1/2 h-0.5 -translate-y-1/2",
        )}
      />
      <div
        className={cn(
          "absolute bg-accent",
          thick ? "inset-y-0 left-0" : "top-1/2 h-0.5 -translate-y-1/2",
          !dragging && "transition-[width] duration-250 ease-linear",
        )}
        style={{ width: `${frac * 100}%` }}
      />
      {/* thumb: visible on hover / drag */}
      {durationMs > 0 && (
        <div
          className={cn(
            "absolute top-1/2 h-2.5 w-1 -translate-y-1/2 bg-primary",
            dragging ? "opacity-100" : "opacity-0 group-hover/seek:opacity-100",
          )}
          style={{ left: `calc(${frac * 100}% - 2px)` }}
        />
      )}
      {/* scrub time bubble */}
      {dragging && (
        <div
          className="pointer-events-none absolute -top-5 -translate-x-1/2 border border-focus bg-raised px-1 text-[10px] tabular-nums text-accent"
          style={{ left: `${frac * 100}%` }}
        >
          {fmtDuration(Math.round(frac * durationMs))}
        </div>
      )}
    </div>
  );
}
