import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";

import { EditableText } from "@/components/ui/EditableText";
import { api } from "@/ipc/invoke";
import type { Track } from "@/ipc/types";
import { fmtDuration, fmtQuality, isHires, isLossy } from "@/lib/format";
import { useKeyboardStore } from "@/lib/keyboard";
import { cn } from "@/lib/utils";
import { useMainTitle } from "@/hooks/useMainTitle";
import { usePlayerStore } from "@/stores/playerStore";

export function SearchView() {
  useMainTitle("search");
  const [query, setQuery] = useState("");
  const [debounced, setDebounced] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);
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
          <p className="p-3 text-[12px] text-error">{String(error)}</p>
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

function ResultRow({ track, onPlay }: { track: Track; onPlay: () => void }) {
  const t = track.technical;
  const playing = usePlayerStore((s) => s.trackId === track.id);
  const queryClient = useQueryClient();
  return (
    <tr
      onDoubleClick={onPlay}
      className={cn("h-7 hover:bg-raised", playing ? "bg-raised" : undefined)}
    >
      <td
        className={cn(
          "w-6 border-l-2 pl-2 text-[11px]",
          playing ? "border-accent text-accent" : "border-transparent text-muted",
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
