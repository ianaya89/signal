import { useQueries, useQuery } from "@tanstack/react-query";
import { Link, useNavigate, useParams } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";

import { TrackRow } from "@/components/library/TrackRow";
import { api } from "@/ipc/invoke";
import type { Track } from "@/ipc/types";
import { registerListHandler } from "@/lib/keyboard";

export function ArtistDetailView() {
  const { artistId } = useParams({ from: "/artists/$artistId" });
  const id = Number(artistId);
  const navigate = useNavigate();
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
      back: () => void navigate({ to: "/artists" }),
    });
  }, [navigate]);

  if (isLoading || !data) {
    return <p className="p-3 text-muted">loading…</p>;
  }

  let flatIndex = -1;

  return (
    <div className="flex h-full flex-col">
      <header className="shrink-0 border-b border-subtle p-3">
        <h1 className="text-[16px] text-primary">{data.artist.name}</h1>
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
              className="flex h-8 items-center gap-2 border-b border-subtle bg-surface px-3 hover:text-accent"
            >
              <span className="text-[12px] text-primary">
                {section.album.name}
              </span>
              <span className="text-[11px] text-muted">
                {section.album.year ?? ""}
              </span>
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
