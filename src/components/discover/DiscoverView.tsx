import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";

import { TrackRow } from "@/components/library/TrackRow";
import { useMainTitle } from "@/hooks/useMainTitle";
import { api } from "@/ipc/invoke";
import type { Track } from "@/ipc/types";
import { registerListHandler } from "@/lib/keyboard";

/** Recommendation shelves computed from listening signal — plain SQL over
 *  ratings, favorites, play counts and recent history. */
export function DiscoverView() {
  useMainTitle("discover");
  const queryClient = useQueryClient();
  const [cursor, setCursor] = useState(0);
  const { data, isLoading } = useQuery({
    queryKey: ["discover"],
    queryFn: api.discover,
    staleTime: 60_000,
  });

  const shelves: { tracks: Track[] }[] = data
    ? [
        { tracks: data.onRepeat },
        { tracks: data.rediscover },
        { tracks: data.fromYourArtists },
        { tracks: data.neverPlayed },
      ]
    : [];
  // flat cursor across every shelf; playback still uses the shelf as context
  const flat = shelves.flatMap((shelf, shelfIndex) =>
    shelf.tracks.map((track, index) => ({ track, shelfIndex, index })),
  );
  const flatRef = useRef(flat);
  flatRef.current = flat;
  const shelvesRef = useRef(shelves);
  shelvesRef.current = shelves;
  const cursorRef = useRef(cursor);
  cursorRef.current = cursor;

  useEffect(() => {
    // TrackRow scrolls itself into view when it becomes the cursor row
    const current = () => flatRef.current[cursorRef.current];
    return registerListHandler({
      move: (delta) =>
        setCursor((c) =>
          Math.min(Math.max(c + delta, 0), Math.max(flatRef.current.length - 1, 0)),
        ),
      top: () => setCursor(0),
      bottom: () => setCursor(Math.max(flatRef.current.length - 1, 0)),
      open: () => {
        const entry = current();
        const shelf = entry && shelvesRef.current[entry.shelfIndex];
        if (entry && shelf) {
          void api.playContext(
            shelf.tracks.map((t) => t.id),
            entry.index,
          );
        }
      },
      stage: () => {
        const entry = current();
        if (entry) void api.queueAdd(entry.track.id);
      },
      fav: () => {
        const entry = current();
        if (entry) {
          void api
            .toggleFavorite(entry.track.id)
            .then(() => queryClient.invalidateQueries());
        }
      },
      rate: (rating) => {
        const entry = current();
        if (entry) {
          void api
            .setRating(entry.track.id, rating)
            .then(() => queryClient.invalidateQueries());
        }
      },
    });
  }, [queryClient]);

  if (isLoading || !data) {
    return <p className="p-3 text-muted">reading your signal…</p>;
  }

  const empty = flat.length === 0;
  if (empty) {
    return (
      <p className="p-3 text-[12px] text-muted">
        nothing to recommend yet — play some music, ✦ what you like
      </p>
    );
  }

  let offset = 0;
  const offsets = shelves.map((shelf) => {
    const start = offset;
    offset += shelf.tracks.length;
    return start;
  });

  return (
    <div className="flex flex-col gap-5 p-3">
      <Shelf
        title="on repeat"
        hint="most played, last 30 days"
        tracks={data.onRepeat}
        indexOffset={offsets[0] ?? 0}
        cursor={cursor}
        onFocus={setCursor}
      />
      <Shelf
        title="rediscover"
        hint="loved but not heard lately"
        tracks={data.rediscover}
        indexOffset={offsets[1] ?? 0}
        cursor={cursor}
        onFocus={setCursor}
      />
      <Shelf
        title="from your artists"
        hint="unheard tracks by artists you play"
        tracks={data.fromYourArtists}
        indexOffset={offsets[2] ?? 0}
        cursor={cursor}
        onFocus={setCursor}
      />
      <Shelf
        title="never played"
        hint="random unplayed corners"
        tracks={data.neverPlayed}
        indexOffset={offsets[3] ?? 0}
        cursor={cursor}
        onFocus={setCursor}
      />
    </div>
  );
}

function Shelf({
  title,
  hint,
  tracks,
  indexOffset,
  cursor,
  onFocus,
}: {
  title: string;
  hint: string;
  tracks: Track[];
  indexOffset: number;
  cursor: number;
  onFocus: (index: number) => void;
}) {
  if (tracks.length === 0) return null;
  const ids = tracks.map((t) => t.id);
  return (
    <section>
      <div className="mb-1 flex items-baseline gap-2">
        <h2 className="text-[10px] uppercase tracking-wider text-accent">
          {title}
        </h2>
        <span className="text-[10px] text-muted">· {hint}</span>
      </div>
      <table className="w-full border-collapse">
        <tbody>
          {tracks.map((track, i) => {
            const index = indexOffset + i;
            return (
              <TrackRow
                key={track.id}
                track={track}
                selected={index === cursor}
                onSelect={() => onFocus(index)}
                onPlay={() => void api.playContext(ids, i)}
              />
            );
          })}
        </tbody>
      </table>
    </section>
  );
}
