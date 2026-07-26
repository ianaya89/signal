import { exitDotMode, setWindowMode } from "@/lib/miniMode";
import { dragWindow } from "@/lib/drag";
import { HeartEqualizer } from "@/components/ui/HeartEqualizer";
import { api } from "@/ipc/invoke";
import { usePlayerStore } from "@/stores/playerStore";

/** Collapsed floating pulse: just the bar-heart as a live equalizer over
 *  a transparent window. Drag anywhere; click expands back, double-click
 *  to full; tiny transport strip fades in on hover. */
export function DotPlayer() {
  const status = usePlayerStore((s) => s.status);
  const playing = status === "playing";

  return (
    <div
      onMouseDown={dragWindow}
      title="signal — drag to move · click: back · double-click: full"
      className="group/dot relative flex h-full w-full cursor-grab items-center justify-center bg-base/50 backdrop-blur-sm active:cursor-grabbing"
    >
      <button
        type="button"
        onClick={() => void exitDotMode()}
        onDoubleClick={(e) => {
          e.stopPropagation();
          void setWindowMode("full");
        }}
        className="flex h-full w-full cursor-grab items-center justify-center active:cursor-grabbing"
      >
        <HeartEqualizer size={56} playing={playing} />
      </button>
      <div
        className="absolute inset-x-2 bottom-0 flex items-center justify-center gap-2 border border-subtle bg-base/90 py-0.5 opacity-0 transition-opacity duration-100 group-hover/dot:opacity-100"
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
  );
}
