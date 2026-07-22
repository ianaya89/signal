import { useQuery } from "@tanstack/react-query";
import { useParams } from "@tanstack/react-router";
import { useState } from "react";

import { api } from "@/ipc/invoke";
import type { Track } from "@/ipc/types";
import { artworkUrl } from "@/lib/artwork";
import { fmtDuration, fmtQuality, isHires, isLossy } from "@/lib/format";
import { cn } from "@/lib/utils";

export function AlbumDetailView() {
  const { albumId } = useParams({ from: "/albums/$albumId" });
  const id = Number(albumId);
  const { data, isLoading } = useQuery({
    queryKey: ["album", id],
    queryFn: () => api.getAlbum(id),
  });

  if (isLoading || !data) {
    return <p className="p-3 text-muted">loading…</p>;
  }

  const { album, tracks } = data;

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
            {tracks.map((track) => (
              <TrackRow key={track.id} track={track} />
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

function TrackRow({ track }: { track: Track }) {
  const t = track.technical;
  return (
    <tr className="h-7 cursor-default hover:bg-raised">
      <td className="w-10 pr-2 text-right text-[11px] text-muted">
        {track.trackNo ?? "—"}
      </td>
      <td className="truncate pr-2 text-[12px] text-primary">{track.title}</td>
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
