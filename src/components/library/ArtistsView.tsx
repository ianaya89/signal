import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";

import { EditableText } from "@/components/ui/EditableText";
import { Loading } from "@/components/ui/States";
import { useMainTitle } from "@/hooks/useMainTitle";
import { api } from "@/ipc/invoke";
import type { ArtistSummary } from "@/ipc/types";
import { registerListHandler } from "@/lib/keyboard";
import { cn } from "@/lib/utils";

export function ArtistsView() {
  useMainTitle("artists");
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [cursor, setCursor] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);
  const { data: artists, isLoading } = useQuery({
    queryKey: ["artists"],
    queryFn: api.listArtists,
  });

  const artistsRef = useRef<ArtistSummary[]>(artists ?? []);
  artistsRef.current = artists ?? [];
  const cursorRef = useRef(cursor);
  cursorRef.current = cursor;

  useEffect(() => {
    const goTo = (index: number) => {
      listRef.current?.children[index]?.scrollIntoView({ block: "nearest" });
      return index;
    };
    return registerListHandler({
      move: (delta) =>
        setCursor((c) =>
          goTo(
            Math.min(
              Math.max(c + delta, 0),
              Math.max(artistsRef.current.length - 1, 0),
            ),
          ),
        ),
      top: () => setCursor(goTo(0)),
      bottom: () =>
        setCursor(goTo(Math.max(artistsRef.current.length - 1, 0))),
      open: () => {
        const artist = artistsRef.current[cursorRef.current];
        if (artist) {
          void navigate({
            to: "/artists/$artistId",
            params: { artistId: String(artist.id) },
          });
        }
      },
    });
  }, [navigate]);

  if (isLoading) {
    return <Loading />;
  }
  if (!artists || artists.length === 0) {
    return <p className="p-3 text-muted">no artists yet — scan your library first</p>;
  }

  return (
    <div ref={listRef} className="py-1">
      {artists.map((artist, i) => (
        <div
          key={artist.id}
          onClick={() => {
            setCursor(i);
            void navigate({
              to: "/artists/$artistId",
              params: { artistId: String(artist.id) },
            });
          }}
          className={cn(
            "flex h-7 cursor-pointer items-center justify-between border-l-2 px-3",
            i === cursor
              ? "border-focus bg-raised"
              : "border-transparent hover:bg-raised/50",
          )}
        >
          <EditableText
            value={artist.name}
            className="min-w-0 text-[12px] text-primary"
            inputClassName="w-56 text-[12px] text-primary"
            onSave={async (name) => {
              await api.renameArtist(artist.id, name);
              await queryClient.invalidateQueries();
            }}
          />
          <span className="shrink-0 text-[11px] text-muted">
            {artist.albumCount} albums · {artist.trackCount} tracks
          </span>
        </div>
      ))}
    </div>
  );
}
