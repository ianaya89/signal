import { exitDotMode, setWindowMode } from "@/lib/miniMode";
import { dragWindow } from "@/lib/drag";
import { HeartEqualizer } from "@/components/ui/HeartEqualizer";
import { api } from "@/ipc/invoke";
import { usePlayerStore } from "@/stores/playerStore";

/** Collapsed floating pulse: idle shows only the bar-heart equalizer over
 *  a transparent window; hover reveals the square panel with transport +
 *  expand. The heart itself is inert so dragging works anywhere;
 *  double-click restores the full window. */
export function DotPlayer() {
  const status = usePlayerStore((s) => s.status);
  const playing = status === "playing";

  return (
    <div
      onMouseDown={dragWindow}
      onDoubleClick={() => void setWindowMode("full")}
      className="group/dot relative flex h-full w-full cursor-grab items-center justify-center border border-transparent transition-colors duration-100 hover:border-subtle hover:bg-base/75 hover:backdrop-blur-sm active:cursor-grabbing"
    >
      <HeartEqualizer size={52} playing={playing} />
      <div
        className="absolute inset-x-1 bottom-1 flex items-center justify-center gap-2 bg-base/90 py-0.5 opacity-0 transition-opacity duration-100 group-hover/dot:opacity-100"
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
        <button
          type="button"
          onClick={() => void exitDotMode()}
          title="expand"
          className="text-[10px] leading-none text-secondary hover:text-accent"
        >
          ⤢
        </button>
      </div>
    </div>
  );
}
