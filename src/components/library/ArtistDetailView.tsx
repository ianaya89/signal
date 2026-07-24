import { useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useParams } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";

import { TrackRow } from "@/components/library/TrackRow";
import { EditableText } from "@/components/ui/EditableText";
import { api } from "@/ipc/invoke";
import type { Track } from "@/ipc/types";
import { artworkUrl } from "@/lib/artwork";
import { registerListHandler } from "@/lib/keyboard";

function AlbumThumb({ albumId, hasArt }: { albumId: number; hasArt: boolean }) {
  const [artError, setArtError] = useState(false);
  return (
    <div className="h-10 w-10 shrink-0 overflow-hidden border border-subtle bg-raised group-hover:border-focus">
      {hasArt && !artError ? (
        <img
          src={artworkUrl(albumId)}
          alt=""
          loading="lazy"
          onError={() => setArtError(true)}
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

export function ArtistDetailView() {
  const { artistId } = useParams({ from: "/artists/$artistId" });
  const id = Number(artistId);
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [cursor, setCursor] = useState(0);

  const { data, isLoading } = useQuery({
    queryKey: ["artist", id],
    queryFn: () => api.getArtist(id),
  });

  const albumQueries = useQueries({
    queries: (data?.albums ?? []).map((album) => ({
      queryKey: ["album", album.id],
      queryFn: () => api.getAlbum(album.id),
    })),
  });

  const sections = albumQueries
    .map((q) => q.data)
    .filter((d): d is NonNullable<typeof d> => d != null);
  const allTracks: Track[] = sections.flatMap((s) => s.tracks);

  const tracksRef = useRef<Track[]>(allTracks);
  tracksRef.current = allTracks;
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
      fav: () => {
        const track = tracksRef.current[cursorRef.current];
        if (track) {
          void api
            .toggleFavorite(track.id)
            .then(() => queryClient.invalidateQueries());
        }
      },
      rate: (rating) => {
        const track = tracksRef.current[cursorRef.current];
        if (track) {
          void api
            .setRating(track.id, rating)
            .then(() => queryClient.invalidateQueries());
        }
      },
      back: () => void navigate({ to: "/artists" }),
    });
    // playFrom reads refs only
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [navigate]);

  if (isLoading || !data) {
    return <p className="p-3 text-muted">loading…</p>;
  }

  let flatIndex = -1;

  return (
    <div className="flex h-full flex-col">
      <header className="shrink-0 border-b border-subtle p-3">
        <h1 className="text-[16px] text-primary">
          <EditableText
            value={data.artist.name}
            inputClassName="w-72 text-[16px] text-primary"
            onSave={async (name) => {
              await api.renameArtist(data.artist.id, name);
              await queryClient.invalidateQueries();
            }}
          />
        </h1>
        <p className="text-[12px] text-secondary">
          {data.artist.albumCount} albums · {data.artist.trackCount} tracks
        </p>
      </header>
      <div className="min-h-0 flex-1 overflow-auto">
        {sections.map((section) => (
          <section key={section.album.id}>
            <Link
              to="/albums/$albumId"
              params={{ albumId: String(section.album.id) }}
              className="group flex h-14 items-center gap-3 border-b border-subtle bg-surface px-3"
            >
              <AlbumThumb
                albumId={section.album.id}
                hasArt={section.album.artworkPath !== null}
              />
              <div className="min-w-0">
                <div className="truncate text-[12px] text-primary group-hover:text-accent">
                  {section.album.name}
                </div>
                <div className="text-[11px] text-muted">
                  {section.album.year ? `${section.album.year} · ` : ""}
                  {section.album.trackCount} tracks
                </div>
              </div>
            </Link>
            <table className="w-full border-collapse">
              <tbody>
                {section.tracks.map((track) => {
                  flatIndex += 1;
                  const index = flatIndex;
                  return (
                    <TrackRow
                      key={track.id}
                      track={track}
                      selected={index === cursor}
                      onSelect={() => setCursor(index)}
                      onPlay={() => playFrom(index)}
                    />
                  );
                })}
              </tbody>
            </table>
          </section>
        ))}
      </div>
    </div>
  );
}
