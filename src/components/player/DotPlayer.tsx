import { exitDotMode, setWindowMode } from "@/lib/miniMode";
import { dragWindow } from "@/lib/drag";
import { cn } from "@/lib/utils";
import { HeartEqualizer } from "@/components/ui/HeartEqualizer";
import { api } from "@/ipc/invoke";
import { usePlayerStore } from "@/stores/playerStore";

/** Collapsed floating button: the bar-heart as a live equalizer. Drag
 *  anywhere on the ring; click expands to mini, double-click to full;
 *  tiny transport strip fades in on hover. */
export function DotPlayer() {
  const status = usePlayerStore((s) => s.status);
  const playing = status === "playing";

  return (
    <div
      onMouseDown={dragWindow}
      title="signal — drag to move · click: back · double-click: full"
      className={cn(
        "group/dot flex h-full w-full cursor-grab items-center justify-center border bg-surface p-[6px] active:cursor-grabbing",
        playing ? "border-accent" : "border-subtle",
      )}
    >
      <div className="relative h-full w-full overflow-hidden border border-subtle bg-base/60 hover:border-focus">
        <button
          type="button"
          onClick={() => void exitDotMode()}
          onDoubleClick={(e) => {
            e.stopPropagation();
            void setWindowMode("full");
          }}
          className="flex h-full w-full items-center justify-center"
        >
          <HeartEqualizer size={50} playing={playing} />
        </button>
        <div
          className="absolute inset-x-0 bottom-0 flex items-center justify-center gap-1.5 bg-base/85 py-px opacity-0 transition-opacity duration-100 group-hover/dot:opacity-100"
          onMouseDown={(e) => e.stopPropagation()}
          onDoubleClick={(e) => e.stopPropagation()}
        >
          <button
            type="button"
            onClick={() => void api.prev()}
            title="previous"
            className="text-[10px] leading-none text-secondary hover:text-accent"
          >
            ⏮
          </button>
          <button
            type="button"
            onClick={() => void api.toggle()}
            title="play / pause"
            className="text-[11px] leading-none text-accent hover:text-primary"
          >
            {playing ? "⏸" : "▶"}
          </button>
          <button
            type="button"
            onClick={() => void api.next()}
            title="next"
            className="text-[10px] leading-none text-secondary hover:text-accent"
          >
            ⏭
          </button>
        </div>
      </div>
    </div>
  );
}
