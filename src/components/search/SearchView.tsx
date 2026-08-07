import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";

import { TrackRow } from "@/components/library/TrackRow";
import { Failed } from "@/components/ui/States";
import { useMainTitle } from "@/hooks/useMainTitle";
import { api } from "@/ipc/invoke";
import type { Track } from "@/ipc/types";
import { registerListHandler, useKeyboardStore } from "@/lib/keyboard";
import { revealTrack } from "@/lib/reveal";
import { usePlayerStore } from "@/stores/playerStore";

export function SearchView() {
  const [query, setQuery] = useState("");
  const [debounced, setDebounced] = useState("");
  const [cursor, setCursor] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const queryClient = useQueryClient();
  const mode = useKeyboardStore((s) => s.mode);
  const setMode = useKeyboardStore((s) => s.setMode);

  useEffect(() => {
    const t = setTimeout(() => setDebounced(query), 120);
    return () => clearTimeout(t);
  }, [query]);

  useEffect(() => {
    if (mode === "search") {
      inputRef.current?.focus();
    }
  }, [mode]);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const { data: results, error } = useQuery({
    queryKey: ["search", debounced],
    queryFn: () => api.search(debounced),
    enabled: debounced.trim().length > 0,
    placeholderData: (prev) => prev,
  });

  useMainTitle("search", debounced.trim() ? (results?.length ?? 0) : undefined);

  const resultsRef = useRef<Track[]>(results ?? []);
  resultsRef.current = results ?? [];
  const cursorRef = useRef(cursor);
  cursorRef.current = cursor;

  useEffect(() => {
    setCursor(0);
  }, [debounced]);

  useEffect(() => {
    const playFrom = (index: number) => {
      const ids = resultsRef.current.map((t) => t.id);
      if (ids.length > 0) void api.playContext(ids, index);
    };
    return registerListHandler({
      move: (delta) =>
        setCursor((c) =>
          Math.min(
            Math.max(c + delta, 0),
            Math.max(resultsRef.current.length - 1, 0),
          ),
        ),
      top: () => setCursor(0),
      bottom: () => setCursor(Math.max(resultsRef.current.length - 1, 0)),
      jump: () => {
        const index = resultsRef.current.findIndex(
          (t) => t.id === usePlayerStore.getState().trackId,
        );
        if (index < 0) return false;
        setCursor(index);
        return true;
      },
      open: () => playFrom(cursorRef.current),
      stage: () => {
        const track = resultsRef.current[cursorRef.current];
        if (track) void api.queueAdd(track.id);
      },
      reveal: () => revealTrack(resultsRef.current[cursorRef.current]),
      fav: () => {
        const track = resultsRef.current[cursorRef.current];
        if (track) {
          void api
            .toggleFavorite(track.id)
            .then(() => queryClient.invalidateQueries());
        }
      },
      rate: (rating) => {
        const track = resultsRef.current[cursorRef.current];
        if (track) {
          void api
            .setRating(track.id, rating)
            .then(() => queryClient.invalidateQueries());
        }
      },
      // esc from the results goes back to the query box
      back: () => inputRef.current?.focus(),
    });
  }, [queryClient]);

  return (
    <div className="flex h-full flex-col">
      <div className="shrink-0 border-b border-subtle p-2">
        <input
          ref={inputRef}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onFocus={() => setMode("search")}
          onBlur={() => setMode("normal")}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              inputRef.current?.blur();
            }
          }}
          placeholder="search — artist:cerati year>1998 codec:flac sampleRate>48k"
          spellCheck={false}
          className="w-full border border-subtle bg-base/60 px-2 py-1 text-[12px] text-primary outline-none focus:border-focus"
        />
      </div>
      <div className="min-h-0 flex-1 overflow-auto">
        {error ? (
          <Failed error={error} />
        ) : !results || results.length === 0 ? (
          <p className="p-3 text-[12px] text-muted">
            {debounced.trim() ? "no matches" : "type to search the library"}
          </p>
        ) : (
          <table className="w-full border-collapse">
            <tbody>
              {results.map((track, i) => (
                <TrackRow
                  key={track.id}
                  track={track}
                  selected={i === cursor}
                  onSelect={() => setCursor(i)}
                  onPlay={() =>
                    void api.playContext(
                      results.map((t) => t.id),
                      i,
                    )
                  }
                />
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

