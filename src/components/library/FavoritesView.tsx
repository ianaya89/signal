import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";

import { SelectionBar } from "@/components/library/SelectionBar";
import { TrackRow } from "@/components/library/TrackRow";
import { TrackTableHeader } from "@/components/library/TrackTableHeader";
import { useMainTitle } from "@/hooks/useMainTitle";
import { useMultiSelect } from "@/hooks/useMultiSelect";
import { useTrackSort } from "@/hooks/useTrackSort";
import { useVirtualWindow } from "@/hooks/useVirtualWindow";
import { api } from "@/ipc/invoke";
import type { Track } from "@/ipc/types";
import { registerListHandler } from "@/lib/keyboard";

export function FavoritesView() {
  useMainTitle("favorites");
  const queryClient = useQueryClient();
  const [cursor, setCursor] = useState(0);

  const { data, isLoading } = useQuery({
    queryKey: ["favorites"],
    queryFn: api.listFavorites,
  });

  const sort = useTrackSort(data ?? []);
  const tracks = sort.sorted;
  const tracksRef = useRef<Track[]>(tracks);
  tracksRef.current = tracks;
  const cursorRef = useRef(cursor);
  cursorRef.current = cursor;
  const containerRef = useRef<HTMLDivElement>(null);
  const { selected, handleRowClick, clear } = useMultiSelect(tracks);
  const selectedRef = useRef(selected);
  selectedRef.current = selected;
  const clearRef = useRef(clear);
  clearRef.current = clear;
  const virtual = useVirtualWindow(
    tracks.length,
    28,
    containerRef,
    tracks.length > 300,
  );
  const virtualRef = useRef(virtual);
  virtualRef.current = virtual;

  const playFrom = (index: number) => {
    const ids = tracksRef.current.map((t) => t.id);
    if (ids.length > 0) void api.playContext(ids, index);
  };

  const unfavorite = () => {
    const track = tracksRef.current[cursorRef.current];
    if (!track) return;
    void api.toggleFavorite(track.id).then(() => {
      setCursor((c) => Math.max(Math.min(c, tracksRef.current.length - 2), 0));
      return queryClient.invalidateQueries();
    });
  };

  useEffect(() => {
    return registerListHandler({
      move: (delta) =>
        setCursor((c) => {
          const next = Math.min(
            Math.max(c + delta, 0),
            tracksRef.current.length - 1,
          );
          virtualRef.current.ensureVisible(next);
          return next;
        }),
      top: () => setCursor(0),
      bottom: () => setCursor(tracksRef.current.length - 1),
      open: () => playFrom(cursorRef.current),
      stage: () => {
        const track = tracksRef.current[cursorRef.current];
        if (track) void api.queueAdd(track.id);
      },
      fav: unfavorite,
      remove: unfavorite,
      rate: (rating) => {
        const track = tracksRef.current[cursorRef.current];
        if (track) {
          void api
            .setRating(track.id, rating)
            .then(() => queryClient.invalidateQueries());
        }
      },
      back: () => {
        if (selectedRef.current.size > 0) clearRef.current();
      },
    });
    // handlers read refs only
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (isLoading) {
    return <p className="p-3 text-muted">loading…</p>;
  }
  if (tracks.length === 0) {
    return (
      <p className="p-3 text-[12px] text-muted">
        nothing favorited yet — press f on a track, or click its ♡
      </p>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex h-7 shrink-0 items-center gap-2 border-b border-subtle px-3 text-[11px]">
        <span className="text-accent">♥</span>
        <span className="text-muted">{tracks.length} favorites</span>
        <button
          type="button"
          onClick={() => playFrom(0)}
          className="ml-auto border border-subtle px-2 text-[10px] text-secondary hover:border-focus hover:text-accent"
        >
          play all
        </button>
        <button
          type="button"
          onClick={() => {
            for (const track of tracks) void api.queueAdd(track.id);
          }}
          title="stage every favorite to the queue"
          className="border border-subtle px-2 text-[10px] text-secondary hover:border-focus hover:text-accent"
        >
          queue all
        </button>
      </div>
      <div ref={containerRef} className="relative min-h-0 flex-1 overflow-auto">
        <table className="w-full border-collapse">
          <TrackTableHeader sort={sort} />
          <tbody>
            {virtual.padTop > 0 && (
              <tr style={{ height: virtual.padTop }} aria-hidden />
            )}
            {tracks.slice(virtual.start, virtual.end).map((track, offset) => {
              const i = virtual.start + offset;
              return (
                <TrackRow
                  key={track.id}
                  track={track}
                  selected={i === cursor}
                  multiSelected={selected.has(track.id)}
                  onSelect={(e) => {
                    if (!handleRowClick(i, e)) setCursor(i);
                  }}
                  onPlay={() => playFrom(i)}
                />
              );
            })}
            {virtual.padBottom > 0 && (
              <tr style={{ height: virtual.padBottom }} aria-hidden />
            )}
          </tbody>
        </table>
        <SelectionBar selected={selected} onClear={clear} />
      </div>
    </div>
  );
}
