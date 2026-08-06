import { useQuery } from "@tanstack/react-query";
import { useNavigate, useParams } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";

import { Failed, Loading } from "@/components/ui/States";
import { useMainTitle } from "@/hooks/useMainTitle";
import { api } from "@/ipc/invoke";
import type { SubsonicArtist } from "@/ipc/types";
import { registerListHandler } from "@/lib/keyboard";
import { cn } from "@/lib/utils";

export function RemoteArtistsView() {
  const { sourceId } = useParams({ from: "/remote/$sourceId" });
  const id = Number(sourceId);
  const navigate = useNavigate();
  const [cursor, setCursor] = useState(0);

  const { data: sources } = useQuery({
    queryKey: ["remote-sources"],
    queryFn: api.remoteSourceList,
  });
  const source = sources?.find((s) => s.id === id);
  useMainTitle(source ? `remote · ${source.name}` : "remote");

  const { data, isLoading, error } = useQuery({
    queryKey: ["remote-artists", id],
    queryFn: () => api.remoteArtists(id),
  });

  // the wire groups artists into initial-letter buckets; the list is flat
  const artists: SubsonicArtist[] = (data?.index ?? []).flatMap((b) => b.artist);
  const artistsRef = useRef<SubsonicArtist[]>(artists);
  artistsRef.current = artists;
  const cursorRef = useRef(cursor);
  cursorRef.current = cursor;

  useEffect(() => {
    return registerListHandler({
      move: (delta) =>
        setCursor((c) =>
          Math.min(Math.max(c + delta, 0), artistsRef.current.length - 1),
        ),
      top: () => setCursor(0),
      bottom: () => setCursor(artistsRef.current.length - 1),
      open: () => {
        const artist = artistsRef.current[cursorRef.current];
        if (artist) {
          void navigate({
            to: "/remote/$sourceId/artists/$artistId",
            params: { sourceId: String(id), artistId: artist.id },
          });
        }
      },
      back: () => void navigate({ to: "/remote" }),
    });
  }, [navigate, id]);

  if (isLoading) return <Loading />;
  if (error) {
    return <Failed error={error} />;
  }
  if (artists.length === 0) {
    return <p className="p-3 text-muted">no artists on this server</p>;
  }

  return (
    <div className="py-1">
      {artists.map((artist, i) => (
        <div
          key={artist.id}
          onClick={() => {
            setCursor(i);
            void navigate({
              to: "/remote/$sourceId/artists/$artistId",
              params: { sourceId: String(id), artistId: artist.id },
            });
          }}
          className={cn(
            "flex h-7 cursor-pointer items-center gap-3 border-l-2 px-3",
            i === cursor
              ? "border-focus bg-raised"
              : "border-transparent hover:bg-raised",
          )}
        >
          <span className="min-w-0 flex-1 truncate text-[12px] text-primary">
            {artist.name}
          </span>
          <span className="text-[11px] text-muted">
            {artist.albumCount} albums
          </span>
        </div>
      ))}
    </div>
  );
}
