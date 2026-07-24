import { useEffect, useRef } from "react";

import { api } from "@/ipc/invoke";
import type { Track } from "@/ipc/types";
import { fmtDuration, fmtQuality, isHires, isLossy } from "@/lib/format";
import { cn } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";

export function TrackRow({
  track,
  selected = false,
  onSelect,
  onPlay,
}: {
  track: Track;
  selected?: boolean;
  onSelect?: () => void;
  /** Defaults to playing the bare track with no follow-on context. */
  onPlay?: () => void;
}) {
  const t = track.technical;
  const playing = usePlayerStore((s) => s.trackId === track.id);
  const ref = useRef<HTMLTableRowElement>(null);

  useEffect(() => {
    if (selected) {
      ref.current?.scrollIntoView({ block: "nearest" });
    }
  }, [selected]);

  return (
    <tr
      ref={ref}
      onClick={onSelect}
      onDoubleClick={() => (onPlay ? onPlay() : void api.play(track.id))}
      className={cn(
        "h-7 cursor-default",
        selected ? "bg-raised" : "hover:bg-raised/50",
      )}
    >
      <td
        className={cn(
          "w-10 border-l-2 pr-2 text-right text-[11px]",
          playing
            ? "border-accent text-accent"
            : selected
              ? "border-focus text-secondary"
              : "border-transparent text-muted",
        )}
      >
        {playing ? "▶" : (track.trackNo ?? "—")}
      </td>
      <td
        className={cn(
          "truncate pr-2 text-[12px]",
          playing ? "text-accent" : "text-primary",
        )}
      >
        {track.title}
      </td>
      <td className="w-32 pr-2">
        <span
          className={cn(
            "text-[11px]",
            isLossy(t.codec)
              ? "text-lossy"
              : isHires(t.bitDepth, t.sampleRateHz)
                ? "text-hires"
                : "text-secondary",
          )}
        >
          [{t.codec}] [{fmtQuality(t.bitDepth, t.sampleRateHz)}]
        </span>
      </td>
      <td className="w-12 pr-3 text-right text-[11px] text-muted">
        {fmtDuration(track.durationMs)}
      </td>
      <td className="w-8 pr-2 text-right">
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            void api.queueAdd(track.id);
          }}
          title="add to queue (a)"
          className="text-[11px] text-muted hover:text-accent"
        >
          +
        </button>
      </td>
    </tr>
  );
}
