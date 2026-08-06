import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";

import { EditableText } from "@/components/ui/EditableText";
import { Failed } from "@/components/ui/States";
import { useMainTitle } from "@/hooks/useMainTitle";
import { api } from "@/ipc/invoke";
import type { Track } from "@/ipc/types";
import { fmtDuration, fmtQuality, isHires, isLossy } from "@/lib/format";
import { registerListHandler, useKeyboardStore } from "@/lib/keyboard";
import { cn } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";

export function SearchView() {
  useMainTitle("search");
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
      open: () => playFrom(cursorRef.current),
      stage: () => {
        const track = resultsRef.current[cursorRef.current];
        if (track) void api.queueAdd(track.id);
      },
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
          className="w-full rounded-[var(--radius-sm)] border border-subtle bg-base/60 px-2 py-1 text-[12px] text-primary outline-none focus:border-focus"
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
                <ResultRow
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

function ResultRow({
  track,
  selected,
  onSelect,
  onPlay,
}: {
  track: Track;
  selected: boolean;
  onSelect: () => void;
  onPlay: () => void;
}) {
  const t = track.technical;
  const playing = usePlayerStore((s) => s.trackId === track.id);
  const queryClient = useQueryClient();
  const ref = useRef<HTMLTableRowElement>(null);

  useEffect(() => {
    if (selected) ref.current?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  return (
    <tr
      ref={ref}
      onClick={onSelect}
      onDoubleClick={onPlay}
      className={cn(
        "h-7 cursor-default",
        selected ? "bg-raised" : playing ? "bg-raised" : "hover:bg-raised/50",
      )}
    >
      <td
        className={cn(
          "w-6 border-l-2 pl-2 text-[11px]",
          playing
            ? "border-accent text-accent"
            : selected
              ? "border-focus text-secondary"
              : "border-transparent text-muted",
        )}
      >
        {playing ? "▶" : ""}
      </td>
      <td
        className={cn(
          "max-w-0 truncate pr-2 text-[12px]",
          playing ? "text-accent" : "text-primary",
        )}
      >
        <EditableText
          value={track.title}
          className="max-w-full"
          inputClassName="w-full text-[12px] text-primary"
          onSave={async (title) => {
            await api.renameTrack(track.id, title);
            await queryClient.invalidateQueries();
          }}
        />
      </td>
      <td className="w-28 pr-2">
        <span
          className={cn(
            "text-[11px]",
            isLossy(t.codec)
              ? "text-lossy"
              : isHires(t.bitDepth, t.sampleRateHz)
                ? "text-hires"
                : "text-secondary",
          )}
        >
          [{t.codec}] [{fmtQuality(t.bitDepth, t.sampleRateHz)}]
        </span>
      </td>
      <td className="w-12 pr-3 text-right text-[11px] text-muted">
        {fmtDuration(track.durationMs)}
      </td>
      <td className="w-8 pr-2 text-right">
        <button
          type="button"
          onClick={() => void api.queueAdd(track.id)}
          title="add to queue (a)"
          className="text-[11px] text-muted hover:text-accent"
        >
          +
        </button>
      </td>
    </tr>
  );
}
