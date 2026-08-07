import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";

import { SelectionBar } from "@/components/library/SelectionBar";
import { TrackRow } from "@/components/library/TrackRow";
import { TrackTableHeader } from "@/components/library/TrackTableHeader";
import { PaneAction, PaneActions } from "@/components/ui/PaneActions";
import { Failed, Loading } from "@/components/ui/States";
import { useMainTitle } from "@/hooks/useMainTitle";
import { useMultiSelect } from "@/hooks/useMultiSelect";
import { useTrackSort } from "@/hooks/useTrackSort";
import { useVirtualWindow } from "@/hooks/useVirtualWindow";
import { api } from "@/ipc/invoke";
import type { Track } from "@/ipc/types";
import { registerListHandler } from "@/lib/keyboard";
import { cn } from "@/lib/utils";
import type { FavoritesFilter } from "@/router";

const FILTERS: { key: FavoritesFilter; label: string; tone: string }[] = [
  { key: "all", label: "all", tone: "var(--sec-about)" },
  { key: "fav", label: "♥ favorites", tone: "var(--error)" },
  { key: "liked", label: "✦ liked", tone: "var(--sec-server)" },
];

export function FavoritesView() {
  useMainTitle("favorites");
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const { filter = "all" } = useSearch({ from: "/favorites" });
  const [cursor, setCursor] = useState(0);

  const { data, isLoading, error } = useQuery({
    queryKey: ["loved"],
    queryFn: api.listLoved,
  });

  const all = data ?? [];
  const filtered = all.filter((t) =>
    filter === "fav" ? t.favorite : filter === "liked" ? (t.rating ?? 0) >= 4 : true,
  );
  const sort = useTrackSort(filtered);
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

  const clampCursor = () =>
    setCursor((c) => Math.max(Math.min(c, tracksRef.current.length - 2), 0));

  const toggleFav = () => {
    const track = tracksRef.current[cursorRef.current];
    if (!track) return;
    void api.toggleFavorite(track.id).then(() => {
      clampCursor();
      return queryClient.invalidateQueries();
    });
  };

  // 'x' drops the track out of this view: unheart it, or clear a 4-5★ rating
  const demote = () => {
    const track = tracksRef.current[cursorRef.current];
    if (!track) return;
    const drop = track.favorite
      ? api.toggleFavorite(track.id)
      : api.setRating(track.id, 0);
    void drop.then(() => {
      clampCursor();
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
      fav: toggleFav,
      remove: demote,
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
    return <Loading />;
  }
  if (error) {
    return (
      <Failed error={error} prefix="could not read favorites" />
    );
  }

  const counts = {
    all: all.length,
    fav: all.filter((t) => t.favorite).length,
    liked: all.filter((t) => (t.rating ?? 0) >= 4).length,
  };

  return (
    <div className="flex h-full flex-col">
      {/* same channel-strip treatment as the settings and system panes; these
          were already tabs in everything but styling, routed via ?filter= */}
      <div
        role="tablist"
        aria-label="favorites filter"
        className="flex h-8 shrink-0 items-stretch gap-px border-b border-subtle bg-base/40 px-2 text-[11px]"
      >
        {FILTERS.map(({ key, label, tone }) => {
          const on = filter === key;
          return (
            <button
              key={key}
              type="button"
              role="tab"
              aria-selected={on}
              onClick={() =>
                void navigate({ to: "/favorites", search: { filter: key } })
              }
              style={{ "--tone": tone } as React.CSSProperties}
              className={cn(
                "shrink-0 border-t-2 px-2 text-[10px] transition-colors",
                on
                  ? "border-t-[color:var(--tone)] bg-raised text-[color:var(--tone)]"
                  : "border-t-[color-mix(in_srgb,var(--tone)_25%,transparent)] text-muted hover:border-t-[color-mix(in_srgb,var(--tone)_65%,transparent)] hover:text-secondary",
              )}
            >
              {label} <span className="tabular-nums">{counts[key]}</span>
            </button>
          );
        })}
      </div>
      <PaneActions>
        <PaneAction
          tone="primary"
          onClick={() => playFrom(0)}
          disabled={tracks.length === 0}
          title="play everything in this filter"
        >
          ▶ play all
        </PaneAction>
        <PaneAction
          onClick={() => {
            for (const track of tracks) void api.queueAdd(track.id);
          }}
          disabled={tracks.length === 0}
          title="stage everything in this filter to the queue"
        >
          + queue all
        </PaneAction>
      </PaneActions>
      {tracks.length === 0 && (
        <p className="p-3 text-[12px] text-muted">
          {filter === "liked"
            ? "nothing rated 4★ or higher yet — press r then 4 or 5 on a track"
            : filter === "fav"
              ? "nothing hearted yet — press f on a track, or click its ♡"
              : "nothing marked yet — f hearts a track, r then 4/5 rates it"}
        </p>
      )}
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
