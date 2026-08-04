import { useQuery } from "@tanstack/react-query";
import { Link, useNavigate, useParams } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";

import { RemoteCover } from "@/components/remote/RemoteCover";
import { useMainTitle } from "@/hooks/useMainTitle";
import { api } from "@/ipc/invoke";
import type { SubsonicAlbum } from "@/ipc/types";
import { registerListHandler } from "@/lib/keyboard";
import { cn } from "@/lib/utils";

export function RemoteArtistDetailView() {
  const { sourceId, artistId } = useParams({
    from: "/remote/$sourceId/artists/$artistId",
  });
  const id = Number(sourceId);
  const navigate = useNavigate();
  const [cursor, setCursor] = useState(0);

  const { data, isLoading, error } = useQuery({
    queryKey: ["remote-artist", id, artistId],
    queryFn: () => api.remoteArtist(id, artistId),
  });

  useMainTitle(data ? `remote · ${data.name}` : undefined);

  const albums: SubsonicAlbum[] = data?.album ?? [];
  const albumsRef = useRef<SubsonicAlbum[]>(albums);
  albumsRef.current = albums;
  const cursorRef = useRef(cursor);
  cursorRef.current = cursor;

  useEffect(() => {
    return registerListHandler({
      move: (delta) =>
        setCursor((c) =>
          Math.min(Math.max(c + delta, 0), albumsRef.current.length - 1),
        ),
      top: () => setCursor(0),
      bottom: () => setCursor(albumsRef.current.length - 1),
      open: () => {
        const album = albumsRef.current[cursorRef.current];
        if (album) {
          void navigate({
            to: "/remote/$sourceId/albums/$albumId",
            params: { sourceId: String(id), albumId: album.id },
          });
        }
      },
      back: () =>
        void navigate({
          to: "/remote/$sourceId",
          params: { sourceId: String(id) },
        }),
    });
  }, [navigate, id]);

  if (isLoading) return <p className="p-3 text-muted">loading…</p>;
  if (error) {
    return <p className="p-3 text-[11px] text-error">{String(error)}</p>;
  }
  if (!data) return null;

  return (
    <div className="flex h-full flex-col">
      <header className="shrink-0 border-b border-subtle p-3">
        <h1 className="text-[16px] text-primary">{data.name}</h1>
        <p className="text-[12px] text-secondary">{albums.length} albums</p>
      </header>
      <div className="min-h-0 flex-1 overflow-auto py-1">
        {albums.map((album, i) => (
          <Link
            key={album.id}
            to="/remote/$sourceId/albums/$albumId"
            params={{ sourceId: String(id), albumId: album.id }}
            onClick={() => setCursor(i)}
            className={cn(
              "group flex h-14 items-center gap-3 border-l-2 px-3",
              i === cursor
                ? "border-focus bg-raised"
                : "border-transparent hover:bg-raised",
            )}
          >
            <RemoteCover
              sourceId={id}
              coverArt={album.coverArt}
              name={album.name}
              className="h-10 w-10"
            />
            <div className="min-w-0">
              <div className="truncate text-[12px] text-primary group-hover:text-accent">
                {album.name}
              </div>
              <div className="text-[11px] text-muted">
                {album.year ? `${album.year} · ` : ""}
                {album.songCount} tracks
              </div>
            </div>
          </Link>
        ))}
      </div>
    </div>
  );
}
