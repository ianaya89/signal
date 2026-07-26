import { useQuery } from "@tanstack/react-query";

import { SeekBar } from "@/components/player/SeekBar";
import { EqBars } from "@/components/ui/HeartEqualizer";
import { api } from "@/ipc/invoke";
import { fmtDuration } from "@/lib/format";
import { cn } from "@/lib/utils";
import { usePlayModeStore } from "@/stores/playModeStore";
import { usePlayerStore } from "@/stores/playerStore";

export function TransportControls() {
  const status = usePlayerStore((s) => s.status);
  const idle = status === "stopped";
  return (
    <span className="flex shrink-0 items-center gap-2.5 text-[14px]">
      <button
        type="button"
        onClick={() => void api.prev()}
        title="previous ({)"
        className={cn("text-secondary hover:text-accent", idle && "opacity-40")}
      >
        ⏮
      </button>
      <button
        type="button"
        onClick={() => void api.toggle()}
        title="play / pause (space)"
        className={cn("text-accent hover:text-primary", idle && "opacity-40")}
      >
        {status === "playing" ? "⏸" : "▶"}
      </button>
      <button
        type="button"
        onClick={() => void api.next()}
        title="next (})"
        className={cn("text-secondary hover:text-accent", idle && "opacity-40")}
      >
        ⏭
      </button>
    </span>
  );
}

export function Timeline({ className }: { className?: string }) {
  const positionMs = usePlayerStore((s) => s.positionMs);
  const durationMs = usePlayerStore((s) => s.durationMs);
  return (
    <span className={cn("flex min-w-0 items-center gap-2", className)}>
      <span className="shrink-0 text-[11px] tabular-nums text-muted">
        {fmtDuration(positionMs)}
      </span>
      <SeekBar
        positionMs={positionMs}
        durationMs={durationMs}
        className="min-w-16 flex-1"
      />
      <span className="shrink-0 text-[11px] tabular-nums text-muted">
        {fmtDuration(durationMs)}
      </span>
    </span>
  );
}

export function ModeButtons() {
  const { shuffle, repeat, toggleShuffle, cycleRepeat } = usePlayModeStore();
  return (
    <span className="flex shrink-0 gap-1.5">
      <button
        type="button"
        onClick={toggleShuffle}
        title="shuffle"
        className={cn("text-[13px]", shuffle ? "text-accent" : "text-muted hover:text-secondary")}
      >
        ⇄
      </button>
      <button
        type="button"
        onClick={cycleRepeat}
        title={`repeat: ${repeat}`}
        className={cn(
          "text-[13px]",
          repeat === "off" ? "text-muted hover:text-secondary" : "text-accent",
        )}
      >
        {repeat === "one" ? "⟳¹" : "⟳"}
      </button>
    </span>
  );
}

export function VolumeSlider() {
  const volume = usePlayerStore((s) => s.volume);
  return (
    <span className="flex shrink-0 items-center gap-1">
      <span className="text-[11px] text-muted">{volume === 0 ? "🔇" : "vol"}</span>
      <input
        type="range"
        min={0}
        max={100}
        value={Math.round(volume * 100)}
        onChange={(e) => void api.setVolume(Number(e.target.value))}
        className="vol-slider w-16"
        style={{ "--vol-fill": `${Math.round(volume * 100)}%` } as React.CSSProperties}
        title={`${Math.round(volume * 100)}% (m mute, +/- adjust)`}
      />
    </span>
  );
}

/** Footer now-playing: live eq bars + artist — title. */
export function NowPlayingLabel() {
  const status = usePlayerStore((s) => s.status);
  const trackId = usePlayerStore((s) => s.trackId);
  const { data } = useQuery({
    queryKey: ["track", trackId],
    queryFn: () => api.getTrack(trackId ?? -1),
    enabled: trackId !== null,
    staleTime: Infinity,
  });

  if (status === "stopped" || trackId === null) {
    return <span className="shrink-0 text-muted">■ stopped</span>;
  }
  return (
    <span className="flex min-w-0 items-center gap-2">
      <EqBars playing={status === "playing"} className="shrink-0" />
      <span className="min-w-0 truncate text-primary">
        {data ? `${data.artistName} — ${data.track.title}` : "…"}
      </span>
    </span>
  );
}
