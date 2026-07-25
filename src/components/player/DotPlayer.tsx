import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { api } from "@/ipc/invoke";
import { artworkUrl } from "@/lib/artwork";
import { setWindowMode } from "@/lib/miniMode";
import { cn } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";

/** Collapsed floating button: outer ring drags the window, the artwork
 *  face expands back to the mini player (double-click = full app). */
export function DotPlayer() {
  const { status, trackId } = usePlayerStore();
  const [artError, setArtError] = useState(false);

  const { data } = useQuery({
    queryKey: ["track", trackId],
    queryFn: () => api.getTrack(trackId ?? -1),
    enabled: trackId !== null,
    staleTime: Infinity,
  });

  const albumId = data?.track.albumId ?? 0;
  const playing = status === "playing";

  return (
    <div
      data-tauri-drag-region
      title={
        data
          ? `${data.track.title} — ${data.artistName} (click: mini · double-click: full)`
          : "signal (click: mini · double-click: full)"
      }
      className={cn(
        "flex h-full w-full items-center justify-center border bg-surface p-[7px]",
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
        className="relative h-full w-full overflow-hidden border border-subtle bg-raised"
      >
        {albumId > 0 && !artError ? (
          <img
            src={artworkUrl(albumId)}
            alt=""
            onError={() => setArtError(true)}
            className="pointer-events-none h-full w-full object-cover"
          />
        ) : (
          <HeartGlyph />
        )}
        {playing && (
          <span className="absolute bottom-0.5 right-0.5 bg-base/80 px-0.5 text-[8px] leading-none text-accent">
            ▶
          </span>
        )}
      </button>
    </div>
  );
}

/** Tiny static version of the bar-heart logo. */
function HeartGlyph() {
  const heights = [30, 55, 72, 60, 78, 60, 72, 55, 30];
  return (
    <span className="flex h-full w-full items-center justify-center gap-[2px]">
      {heights.map((h, i) => (
        <span
          key={i}
          className="w-[3px] rounded-full bg-accent"
          style={{ height: `${h}%`, opacity: 0.65 + 0.35 * Math.sin(i) ** 2 }}
        />
      ))}
    </span>
  );
}
