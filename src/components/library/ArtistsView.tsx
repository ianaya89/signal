import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";

import { EditableText } from "@/components/ui/EditableText";
import { GroupHeader } from "@/components/ui/GroupHeader";
import { Loading } from "@/components/ui/States";
import { useMainTitle } from "@/hooks/useMainTitle";
import { api } from "@/ipc/invoke";
import type { ArtistSummary } from "@/ipc/types";
import { groupRuns, headerAt, initialOf, worthGrouping } from "@/lib/grouping";
import { registerListHandler } from "@/lib/keyboard";
import { cn } from "@/lib/utils";

type ArtistSort = "name" | "albums" | "tracks";

const SORT_KEY = "artists.sort";
const SORTS: { key: ArtistSort; label: string }[] = [
  { key: "name", label: "name" },
  { key: "albums", label: "albums" },
  { key: "tracks", label: "tracks" },
];

function sortArtists(artists: ArtistSummary[], sort: ArtistSort) {
  if (sort === "name") return artists; // backend already orders by name
  const sorted = [...artists];
  sorted.sort((a, b) =>
    sort === "albums"
      ? b.albumCount - a.albumCount
      : b.trackCount - a.trackCount,
  );
  return sorted;
}

export function ArtistsView() {
  useMainTitle("artists");
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [cursor, setCursor] = useState(0);
  const [sort, setSort] = useState<ArtistSort>(
    () => (localStorage.getItem(SORT_KEY) as ArtistSort) || "name",
  );
  const listRef = useRef<HTMLDivElement>(null);
  const { data, isLoading } = useQuery({
    queryKey: ["artists"],
    queryFn: api.listArtists,
  });

  const artists = sortArtists(data ?? [], sort);
  // only the name sort has contiguous initials; counting sorts get no headers
  const groups = worthGrouping(
    groupRuns(artists, sort === "name" ? (a) => initialOf(a.name) : null),
    artists,
  );

  const artistsRef = useRef<ArtistSummary[]>(artists);
  artistsRef.current = artists;
  const cursorRef = useRef(cursor);
  cursorRef.current = cursor;

  useEffect(() => {
    const goTo = (index: number) => {
      listRef.current
        ?.querySelector(`[data-idx="${index}"]`)
        ?.scrollIntoView({ block: "nearest" });
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
  if (artists.length === 0) {
    return (
      <p className="p-3 text-[12px] text-muted">
        no artists yet — scan your library first
      </p>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex h-7 shrink-0 items-center gap-1 border-b border-subtle px-3 text-[10px]">
        <span className="text-muted">sort:</span>
        {SORTS.map(({ key, label }) => (
          <button
            key={key}
            type="button"
            onClick={() => {
              setSort(key);
              setCursor(0);
              localStorage.setItem(SORT_KEY, key);
            }}
            className={cn(
              "px-1.5 py-0.5",
              sort === key
                ? "bg-raised text-accent"
                : "text-muted hover:text-secondary",
            )}
          >
            {label}
          </button>
        ))}
      </div>
      <div ref={listRef} className="min-h-0 flex-1 overflow-auto">
        {artists.map((artist, i) => {
          const header = headerAt(groups, i);
          return (
            <div key={artist.id}>
              {header && (
                <GroupHeader label={header.label} count={header.count} />
              )}
              <div
                data-idx={i}
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
                <span className="shrink-0 text-[11px] text-muted tabular-nums">
                  {artist.albumCount} albums · {artist.trackCount} tracks
                </span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
