import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useParams } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";

import { SelectionBar } from "@/components/library/SelectionBar";
import { TrackRow } from "@/components/library/TrackRow";
import { Loading } from "@/components/ui/States";
import { useListCursor } from "@/hooks/useListMemory";
import { useMainTitle } from "@/hooks/useMainTitle";
import { useMultiSelect } from "@/hooks/useMultiSelect";
import { useVirtualWindow } from "@/hooks/useVirtualWindow";
import { api } from "@/ipc/invoke";
import type { GenreSummary, Track } from "@/ipc/types";
import { registerListHandler } from "@/lib/keyboard";
import { revealTrack } from "@/lib/reveal";
import { cn } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";

export function GenresView() {
  const navigate = useNavigate();
  const listRef = useRef<HTMLDivElement>(null);
  const { data: genres, isLoading } = useQuery({
    queryKey: ["genres"],
    queryFn: api.listGenres,
  });

  const [cursor, setCursor] = useListCursor("genres", genres?.length);
  useMainTitle("genres", genres?.length);

  const genresRef = useRef<GenreSummary[]>(genres ?? []);
  genresRef.current = genres ?? [];
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
              Math.max(genresRef.current.length - 1, 0),
            ),
          ),
        ),
      top: () => setCursor(goTo(0)),
      bottom: () => setCursor(goTo(Math.max(genresRef.current.length - 1, 0))),
      open: () => {
        const genre = genresRef.current[cursorRef.current];
        if (genre) {
          void navigate({
            to: "/genres/$genreId",
            params: { genreId: String(genre.id) },
          });
        }
      },
    });
  }, [navigate]);

  if (isLoading) {
    return <Loading />;
  }
  if (!genres || genres.length === 0) {
    return <p className="p-3 text-muted">no genres yet — scan your library first</p>;
  }

  const max = genres[0]?.trackCount ?? 1;

  return (
    <div ref={listRef} className="flex flex-col gap-px py-1">
      {genres.map((genre, i) => (
        <Link
          key={genre.id}
          to="/genres/$genreId"
          params={{ genreId: String(genre.id) }}
          onClick={() => setCursor(i)}
          className={cn(
            "group flex h-7 items-center gap-3 border-l-2 px-3",
            i === cursor
              ? "border-focus bg-raised"
              : "border-transparent hover:bg-raised/50",
          )}
        >
          <span className="w-40 shrink-0 truncate text-[12px] text-primary group-hover:text-accent">
            {genre.name}
          </span>
          <div className="h-2 flex-1 bg-raised">
            <div
              className="h-full bg-accent-dim group-hover:bg-accent"
              style={{ width: `${(genre.trackCount / max) * 100}%` }}
            />
          </div>
          <span className="w-14 shrink-0 text-right text-[11px] text-muted">
            {genre.trackCount}
          </span>
        </Link>
      ))}
    </div>
  );
}

export function GenreDetailView() {
  const { genreId } = useParams({ from: "/genres/$genreId" });
  const id = Number(genreId);
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [cursor, setCursor] = useState(0);

  const { data: genres } = useQuery({ queryKey: ["genres"], queryFn: api.listGenres });
  const genreName = genres?.find((g) => g.id === id)?.name;

  const { data: tracks, isLoading } = useQuery({
    queryKey: ["genre-tracks", id],
    queryFn: () => api.genreTracks(id),
  });

  useMainTitle(genreName ? `genre · ${genreName}` : undefined, tracks?.length);

  const list = tracks ?? [];
  const tracksRef = useRef<Track[]>(list);
  tracksRef.current = list;
  const cursorRef = useRef(cursor);
  cursorRef.current = cursor;
  const containerRef = useRef<HTMLDivElement>(null);
  const { selected, handleRowClick, clear } = useMultiSelect(list);
  const selectedRef = useRef(selected);
  selectedRef.current = selected;
  const clearRef = useRef(clear);
  clearRef.current = clear;
  const virtual = useVirtualWindow(list.length, 28, containerRef, list.length > 300);
  const virtualRef = useRef(virtual);
  virtualRef.current = virtual;

  const playFrom = (index: number) => {
    const ids = tracksRef.current.map((t) => t.id);
    if (ids.length > 0) void api.playContext(ids, index);
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
      jump: () => {
        const index = tracksRef.current.findIndex(
          (t) => t.id === usePlayerStore.getState().trackId,
        );
        if (index < 0) return false;
        setCursor(index);
        virtualRef.current.ensureVisible(index);
        return true;
      },
      open: () => playFrom(cursorRef.current),
      stage: () => {
        const track = tracksRef.current[cursorRef.current];
        if (track) void api.queueAdd(track.id);
      },
      reveal: () => revealTrack(tracksRef.current[cursorRef.current]),
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
      back: () => {
        if (selectedRef.current.size > 0) {
          clearRef.current();
          return;
        }
        void navigate({ to: "/genres" });
      },
    });
    // playFrom reads refs only
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [navigate]);

  if (isLoading) {
    return <Loading />;
  }

  return (
    <div ref={containerRef} className="relative min-h-0 flex-1 overflow-auto">
      <table className="w-full border-collapse">
        <tbody>
          {virtual.padTop > 0 && <tr style={{ height: virtual.padTop }} aria-hidden />}
          {list.slice(virtual.start, virtual.end).map((track, offset) => {
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
  );
}
