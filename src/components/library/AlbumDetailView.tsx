import { useQuery } from "@tanstack/react-query";
import { useNavigate, useParams } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";

import { api } from "@/ipc/invoke";
import type { Track } from "@/ipc/types";
import { artworkUrl } from "@/lib/artwork";
import { fmtDuration, fmtQuality, isHires, isLossy } from "@/lib/format";
import { registerListHandler } from "@/lib/keyboard";
import { cn } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";

export function AlbumDetailView() {
  const { albumId } = useParams({ from: "/albums/$albumId" });
  const id = Number(albumId);
  const navigate = useNavigate();
  const [cursor, setCursor] = useState(0);
  const { data, isLoading } = useQuery({
    queryKey: ["album", id],
    queryFn: () => api.getAlbum(id),
  });

  const tracks = data?.tracks ?? [];
  const tracksRef = useRef<Track[]>(tracks);
  tracksRef.current = tracks;

  useEffect(() => {
    return registerListHandler({
      move: (delta) =>
        setCursor((c) =>
          Math.min(Math.max(c + delta, 0), tracksRef.current.length - 1),
        ),
      top: () => setCursor(0),
      bottom: () => setCursor(tracksRef.current.length - 1),
      open: () => {
        const track = tracksRef.current[cursorRef.current];
        if (track) void api.play(track.id);
      },
      stage: () => {
        const track = tracksRef.current[cursorRef.current];
        if (track) void api.queueAdd(track.id);
      },
      back: () => void navigate({ to: "/" }),
    });
  }, [navigate]);

  const cursorRef = useRef(cursor);
  cursorRef.current = cursor;

  if (isLoading || !data) {
    return <p className="p-3 text-muted">loading…</p>;
  }

  const { album } = data;

  return (
    <div className="flex h-full flex-col">
      <header className="flex shrink-0 items-end gap-3 border-b border-subtle p-3">
        <AlbumArt albumId={album.id} hasArt={album.artworkPath !== null} />
        <div className="min-w-0">
          <h1 className="truncate text-[16px] text-primary">{album.name}</h1>
          <p className="truncate text-[12px] text-secondary">
            {album.artistName}
            {album.year ? ` · ${album.year}` : ""} · {album.trackCount} tracks
          </p>
        </div>
      </header>
      <div className="min-h-0 flex-1 overflow-auto">
        <table className="w-full border-collapse">
          <tbody>
            {tracks.map((track, i) => (
              <TrackRow
                key={track.id}
                track={track}
                selected={i === cursor}
                onSelect={() => setCursor(i)}
              />
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function AlbumArt({ albumId, hasArt }: { albumId: number; hasArt: boolean }) {
  const [artError, setArtError] = useState(false);
  return (
    <div className="h-20 w-20 shrink-0 overflow-hidden border border-subtle bg-raised">
      {hasArt && !artError ? (
        <img
          src={artworkUrl(albumId)}
          alt=""
          onError={() => setArtError(true)}
          className="h-full w-full object-cover"
        />
      ) : (
        <div className="flex h-full w-full items-center justify-center text-xl text-muted">
          ♪
        </div>
      )}
    </div>
  );
}

function TrackRow({
  track,
  selected,
  onSelect,
}: {
  track: Track;
  selected: boolean;
  onSelect: () => void;
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
      onDoubleClick={() => void api.play(track.id)}
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
    </tr>
  );
}
