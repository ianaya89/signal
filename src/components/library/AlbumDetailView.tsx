import { useQuery } from "@tanstack/react-query";
import { useNavigate, useParams } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";

import { TrackRow } from "@/components/library/TrackRow";
import { api } from "@/ipc/invoke";
import type { Track } from "@/ipc/types";
import { artworkUrl } from "@/lib/artwork";
import { registerListHandler } from "@/lib/keyboard";

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
  const cursorRef = useRef(cursor);
  cursorRef.current = cursor;

  const playFrom = (index: number) => {
    const ids = tracksRef.current.map((t) => t.id);
    if (ids.length > 0) void api.playContext(ids, index);
  };

  useEffect(() => {
    return registerListHandler({
      move: (delta) =>
        setCursor((c) =>
          Math.min(Math.max(c + delta, 0), tracksRef.current.length - 1),
        ),
      top: () => setCursor(0),
      bottom: () => setCursor(tracksRef.current.length - 1),
      open: () => playFrom(cursorRef.current),
      stage: () => {
        const track = tracksRef.current[cursorRef.current];
        if (track) void api.queueAdd(track.id);
      },
      back: () => void navigate({ to: "/" }),
    });
    // playFrom reads refs only
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [navigate]);

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
                onPlay={() => playFrom(i)}
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
