import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { SeekBar } from "@/components/player/SeekBar";
import { api } from "@/ipc/invoke";
import { artworkUrl } from "@/lib/artwork";
import { fmtDuration, fmtQuality, isHires, isLossy } from "@/lib/format";
import { dragWindow } from "@/lib/drag";
import { setWindowMode } from "@/lib/miniMode";
import { cn } from "@/lib/utils";
import { EqBars, HeartEqualizer } from "@/components/ui/HeartEqualizer";
import { usePlayModeStore } from "@/stores/playModeStore";
import { usePlayerStore } from "@/stores/playerStore";

/** Compact always-on-top player. Double-click anywhere inert or hit ⤢/Esc
 *  to restore the full window. */
export function MiniPlayer() {
  const { status, trackId, positionMs, durationMs, volume, bitPerfect } =
    usePlayerStore();
  const { shuffle, repeat, toggleShuffle, cycleRepeat } = usePlayModeStore();

  const { data } = useQuery({
    queryKey: ["track", trackId],
    queryFn: () => api.getTrack(trackId ?? -1),
    enabled: trackId !== null,
    staleTime: Infinity,
  });

  const restore = () => {
    void setWindowMode("full");
  };

  const t = data?.track.technical;

  return (
    <div
      onMouseDown={dragWindow}
      className="flex h-full cursor-grab select-none flex-col overflow-hidden border border-focus bg-surface active:cursor-grabbing"
    >
      <div className="flex min-h-0 flex-1">
        <MiniArt albumId={data?.track.albumId ?? null} playing={status === "playing"} />

        <div className="flex min-w-0 flex-1 flex-col justify-between px-2 py-1.5">
          {/* title row + restore */}
          <div className="flex items-start gap-1">
            <div className="min-w-0 flex-1">
              <Scrolling
                text={data?.track.title ?? "nothing playing"}
                className="text-[12px] leading-tight text-primary"
              />
              <Scrolling
                text={
                  data
                    ? `${data.artistName}${data.albumName ? ` — ${data.albumName}` : ""}`
                    : "stage something and press play"
                }
                className="text-[10px] text-secondary"
              />
            </div>
            {data && (
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  void api
                    .toggleFavorite(data.track.id)
                    .then(() => void 0);
                }}
                title="favorite"
                className={cn(
                  "shrink-0 text-[12px]",
                  data.track.favorite
                    ? "text-accent"
                    : "text-muted hover:text-accent",
                )}
              >
                {data.track.favorite ? "♥" : "♡"}
              </button>
            )}
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                void setWindowMode("dot");
              }}
              title="collapse to floating dot"
              className="shrink-0 px-0.5 text-[12px] text-secondary hover:text-accent"
            >
              ▪
            </button>
            <button
              type="button"
              onClick={restore}
              title="restore full window (esc)"
              className="shrink-0 px-0.5 text-[12px] text-secondary hover:text-accent"
            >
              ⤢
            </button>
          </div>

          {/* technical line */}
          <div className="flex items-center gap-2 text-[10px]">
            {t ? (
              <>
                <span
                  className={cn(
                    isLossy(t.codec)
                      ? "text-lossy"
                      : isHires(t.bitDepth, t.sampleRateHz)
                        ? "text-hires"
                        : "text-secondary",
                  )}
                >
                  [{t.codec}] [{fmtQuality(t.bitDepth, t.sampleRateHz)}]
                </span>
                {bitPerfect && <span className="text-bitperfect">● bit-perfect</span>}
                <EqBars playing={status === "playing"} className="ml-auto" />
              </>
            ) : (
              <span className="text-muted">—</span>
            )}
          </div>

          {/* controls row */}
          <div className="flex items-center gap-2">
            <div className="flex items-center gap-1.5 text-[13px]">
              <button
                type="button"
                onClick={() => void api.prev()}
                title="previous"
                className="text-secondary hover:text-accent"
              >
                ⏮
              </button>
              <button
                type="button"
                onClick={() => void api.toggle()}
                title="play / pause (space)"
                className="text-accent hover:text-primary"
              >
                {status === "playing" ? "⏸" : "▶"}
              </button>
              <button
                type="button"
                onClick={() => void api.next()}
                title="next"
                className="text-secondary hover:text-accent"
              >
                ⏭
              </button>
            </div>
            <span className="flex items-center gap-1 text-[12px]">
              <button
                type="button"
                onClick={toggleShuffle}
                title="shuffle"
                className={shuffle ? "text-accent" : "text-muted hover:text-secondary"}
              >
                ⇄
              </button>
              <button
                type="button"
                onClick={cycleRepeat}
                title={`repeat: ${repeat}`}
                className={repeat === "off" ? "text-muted hover:text-secondary" : "text-accent"}
              >
                {repeat === "one" ? "⟳¹" : "⟳"}
              </button>
            </span>
            <input
              type="range"
              min={0}
              max={100}
              value={Math.round(volume * 100)}
              onChange={(e) => void api.setVolume(Number(e.target.value))}
              onDoubleClick={(e) => e.stopPropagation()}
              title={`volume ${Math.round(volume * 100)}%`}
              className="vol-slider w-14"
            />
            <span
             
              className="ml-auto shrink-0 text-[10px] tabular-nums text-muted"
            >
              {fmtDuration(positionMs)} / {fmtDuration(durationMs)}
            </span>
          </div>
        </div>
      </div>

      <SeekBar positionMs={positionMs} durationMs={durationMs} thick className="w-full shrink-0" />
    </div>
  );
}

/** Marquee only when the text plausibly overflows; static otherwise. */
function Scrolling({ text, className }: { text: string; className?: string }) {
  const long = text.length > 34;
  return (
    <div className={cn(long ? "marquee" : "truncate", className)}>
      <span style={long ? { ["--marquee-shift" as never]: "-45%" } : undefined}>
        {text}
      </span>
    </div>
  );
}

function MiniArt({
  albumId,
  playing,
}: {
  albumId: number | null;
  playing: boolean;
}) {
  const [err, setErr] = useState(false);
  return (
    <div
     
      className="h-full w-[108px] shrink-0 border-r border-subtle bg-raised"
    >
      {albumId !== null && albumId > 0 && !err ? (
        <img
          src={artworkUrl(albumId)}
          alt=""
          onError={() => setErr(true)}
          className="pointer-events-none h-full w-full object-cover"
        />
      ) : (
        <div className="flex h-full w-full items-center justify-center">
          <HeartEqualizer size={72} playing={playing} />
        </div>
      )}
    </div>
  );
}

