import { setWindowMode } from "@/lib/miniMode";
import { dragWindow } from "@/lib/drag";
import { cn } from "@/lib/utils";
import { HeartEqualizer } from "@/components/ui/HeartEqualizer";
import { usePlayerStore } from "@/stores/playerStore";

/** Collapsed floating button: the bar-heart as a live equalizer. Drag
 *  anywhere on the ring; click expands to mini, double-click to full. */
export function DotPlayer() {
  const status = usePlayerStore((s) => s.status);
  const playing = status === "playing";

  return (
    <div
      onMouseDown={dragWindow}
      title="signal — drag to move · click: mini · double-click: full"
      className={cn(
        "flex h-full w-full cursor-grab items-center justify-center border bg-surface p-[6px] active:cursor-grabbing",
        playing ? "border-accent" : "border-subtle",
      )}
    >
      <button
        type="button"
        onClick={() => void setWindowMode("mini")}
        onDoubleClick={(e) => {
          e.stopPropagation();
          void setWindowMode("full");
        }}
        className="flex h-full w-full items-center justify-center overflow-hidden border border-subtle bg-base/60 hover:border-focus"
      >
        <HeartEqualizer size={50} playing={playing} />
      </button>
    </div>
  );
}
