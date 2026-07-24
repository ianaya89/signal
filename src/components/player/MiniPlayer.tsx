import { useQuery } from "@tanstack/react-query";
import { useRef, useState } from "react";

import { api } from "@/ipc/invoke";
import { artworkUrl } from "@/lib/artwork";
import { fmtDuration } from "@/lib/format";
import { exitMiniWindow } from "@/lib/miniMode";
import { cn } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";
import { useUiStore } from "@/stores/uiStore";

/** Winamp-style compact player: artwork, title, transport, seek. */
export function MiniPlayer() {
  const { status, trackId, positionMs, durationMs } = usePlayerStore();
  const setMiniMode = useUiStore((s) => s.setMiniMode);

  const { data } = useQuery({
    queryKey: ["track", trackId],
    queryFn: () => api.getTrack(trackId ?? -1),
    enabled: trackId !== null,
    staleTime: Infinity,
  });

  const expand = () => {
    void exitMiniWindow().then(() => setMiniMode(false));
  };

  return (
    <div
      data-tauri-drag-region
      className="flex h-full select-none flex-col border border-focus bg-surface"
    >
      <div data-tauri-drag-region className="flex min-h-0 flex-1 items-center gap-2 p-2">
        <MiniArt albumId={data?.track.albumId ?? null} />
        <div data-tauri-drag-region className="min-w-0 flex-1">
          <div className="truncate text-[12px] text-primary">
            {data?.track.title ?? "nothing playing"}
          </div>
          <div className="truncate text-[11px] text-muted">
            {data ? data.artistName : "signal"}
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2 text-[14px]">
          <button
            type="button"
            onClick={() => void api.prev()}
            className="text-secondary hover:text-accent"
          >
            ⏮
          </button>
          <button
            type="button"
            onClick={() => void api.toggle()}
            className="text-accent hover:text-primary"
          >
            {status === "playing" ? "⏸" : "▶"}
          </button>
          <button
            type="button"
            onClick={() => void api.next()}
            className="text-secondary hover:text-accent"
          >
            ⏭
          </button>
          <button
            type="button"
            onClick={expand}
            title="expand"
            className="ml-1 text-[11px] text-muted hover:text-accent"
          >
            ⤢
          </button>
        </div>
      </div>
      <MiniSeek positionMs={positionMs} durationMs={durationMs} />
    </div>
  );
}

function MiniArt({ albumId }: { albumId: number | null }) {
  const [err, setErr] = useState(false);
  return (
    <div className="h-14 w-14 shrink-0 overflow-hidden border border-subtle bg-raised">
      {albumId !== null && albumId > 0 && !err ? (
        <img
          src={artworkUrl(albumId)}
          alt=""
          onError={() => setErr(true)}
          className="h-full w-full object-cover"
        />
      ) : (
        <div className="flex h-full w-full items-center justify-center text-muted">
          ♪
        </div>
      )}
    </div>
  );
}

function MiniSeek({
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
    <div className="flex shrink-0 items-center gap-2 border-t border-subtle px-2 py-1">
      <span className="text-[10px] text-muted">{fmtDuration(positionMs)}</span>
      <div
        ref={barRef}
        onClick={seek}
        className={cn("relative h-3 flex-1", durationMs > 0 && "cursor-pointer")}
      >
        <div className="absolute top-1/2 h-0.5 w-full -translate-y-1/2 bg-subtle" />
        <div
          className="absolute top-1/2 h-0.5 -translate-y-1/2 bg-accent"
          style={{ width: `${pct}%` }}
        />
      </div>
      <span className="text-[10px] text-muted">{fmtDuration(durationMs)}</span>
    </div>
  );
}
