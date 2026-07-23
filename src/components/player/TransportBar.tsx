import { useQuery } from "@tanstack/react-query";
import { useRef } from "react";

import { api } from "@/ipc/invoke";
import { fmtDuration } from "@/lib/format";
import { usePlayerStore } from "@/stores/playerStore";

export function TransportBar() {
  const status = usePlayerStore((s) => s.status);
  const trackId = usePlayerStore((s) => s.trackId);
  const positionMs = usePlayerStore((s) => s.positionMs);
  const durationMs = usePlayerStore((s) => s.durationMs);

  if (status === "stopped" || trackId === null) {
    return <span className="shrink-0 text-muted">■ stopped</span>;
  }

  return (
    <div className="flex min-w-0 flex-1 items-center gap-3">
      <button
        type="button"
        onClick={() => void api.prev()}
        title="restart ({)"
        className="shrink-0 text-secondary hover:text-accent"
      >
        ⏮
      </button>
      <button
        type="button"
        onClick={() => void api.toggle()}
        className="shrink-0 text-accent hover:text-primary"
      >
        {status === "playing" ? "⏸" : "▶"}
      </button>
      <button
        type="button"
        onClick={() => void api.next()}
        title="next from queue (})"
        className="shrink-0 text-secondary hover:text-accent"
      >
        ⏭
      </button>
      <NowPlayingTitle trackId={trackId} />
      <span className="shrink-0 text-[11px] text-muted">
        {fmtDuration(positionMs)}
      </span>
      <SeekBar positionMs={positionMs} durationMs={durationMs} />
      <span className="shrink-0 text-[11px] text-muted">
        {fmtDuration(durationMs)}
      </span>
    </div>
  );
}

function NowPlayingTitle({ trackId }: { trackId: number }) {
  const { data } = useQuery({
    queryKey: ["track", trackId],
    queryFn: () => api.getTrack(trackId),
    staleTime: Infinity,
  });
  return (
    <span className="min-w-0 truncate text-[12px] text-primary">
      {data ? `${data.artistName} — ${data.track.title}` : "…"}
    </span>
  );
}

function SeekBar({
  positionMs,
  durationMs,
}: {
  positionMs: number;
  durationMs: number;
}) {
  const barRef = useRef<HTMLDivElement>(null);
  const pct = durationMs > 0 ? (positionMs / durationMs) * 100 : 0;

  const seek = (e: React.MouseEvent) => {
    const bar = barRef.current;
    if (!bar || durationMs === 0) return;
    const rect = bar.getBoundingClientRect();
    const frac = Math.min(Math.max((e.clientX - rect.left) / rect.width, 0), 1);
    void api.seek(Math.round(frac * durationMs));
  };

  return (
    <div
      ref={barRef}
      onClick={seek}
      className="group relative h-4 min-w-24 flex-1 cursor-pointer"
    >
      <div className="absolute top-1/2 h-0.5 w-full -translate-y-1/2 bg-subtle" />
      <div
        className="absolute top-1/2 h-0.5 -translate-y-1/2 bg-accent"
        style={{ width: `${pct}%` }}
      />
    </div>
  );
}
