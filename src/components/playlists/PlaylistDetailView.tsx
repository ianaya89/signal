import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useParams } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";

import { SelectionBar } from "@/components/library/SelectionBar";
import { TrackRow } from "@/components/library/TrackRow";
import { TrackTableHeader } from "@/components/library/TrackTableHeader";
import { EditableText } from "@/components/ui/EditableText";
import { Loading } from "@/components/ui/States";
import { BTN } from "@/components/ui/controls";
import { useMainTitle } from "@/hooks/useMainTitle";
import { useMultiSelect } from "@/hooks/useMultiSelect";
import { useTrackSort } from "@/hooks/useTrackSort";
import { useVirtualWindow } from "@/hooks/useVirtualWindow";
import { api } from "@/ipc/invoke";
import type { Track } from "@/ipc/types";
import { registerListHandler } from "@/lib/keyboard";
import { pickSavePath } from "@/lib/pickFolder";
import { cn, errText } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";
import { toast } from "@/stores/toastStore";

export function PlaylistDetailView() {
  const { kind, playlistId } = useParams({
    from: "/playlists/$kind/$playlistId",
  });
  const id = Number(playlistId);
  const smart = kind === "smart";
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [cursor, setCursor] = useState(0);

  const { data, isLoading } = useQuery({
    queryKey: ["playlist", kind, id],
    queryFn: () => api.playlistGet(id, smart),
  });

  useMainTitle(data ? `playlist · ${data.name}` : undefined, data?.tracks.length);
  const sort = useTrackSort(data?.tracks ?? []);
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
  const virtual = useVirtualWindow(tracks.length, 28, containerRef, tracks.length > 300);
  const virtualRef = useRef(virtual);
  virtualRef.current = virtual;

  const playFrom = (index: number) => {
    const ids = tracksRef.current.map((t) => t.id);
    if (ids.length > 0) void api.playContext(ids, index);
  };

  const removeAt = (index: number) => {
    if (smart) return;
    const track = tracksRef.current[index];
    if (!track) return;
    void api
      .playlistRemoveTrack(id, track.id)
      .then(() =>
        queryClient.invalidateQueries({ queryKey: ["playlist", kind, id] }),
      );
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
      remove: () => removeAt(cursorRef.current),
      back: () => {
        if (selectedRef.current.size > 0) {
          clearRef.current();
          return;
        }
        void navigate({ to: "/playlists" });
      },
    });
    // handlers read refs only
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [navigate, smart, id]);

  if (isLoading || !data) {
    return <Loading />;
  }

  return (
    <div className="flex h-full flex-col">
      <header className="flex shrink-0 items-center gap-2 border-b border-subtle p-3">
        <h1 className="text-[16px] text-primary">
          {data.smart ? (
            data.name
          ) : (
            <EditableText
              value={data.name}
              inputClassName="w-64 text-[16px] text-primary"
              onSave={async (name) => {
                await api.playlistRename(id, name);
                await queryClient.invalidateQueries();
              }}
            />
          )}
        </h1>
        {data.smart && (
          <span className="bg-raised px-1.5 py-0.5 text-[10px] text-hires">
            smart
          </span>
        )}
        <span className="text-[12px] text-muted">
          {tracks.length} tracks
        </span>
        <button
          type="button"
          onClick={() => {
            void (async () => {
              const dest = await pickSavePath(`${data.name}.m3u`, "m3u");
              if (!dest) return;
              const count = await api.exportM3u(id, smart, dest);
              toast.ok(`exported ${count} tracks`);
            })().catch((e) => toast.error(errText(e)));
          }}
          className={cn("ml-auto", BTN)}
        >
          export m3u
        </button>
      </header>
      <div ref={containerRef} className="relative min-h-0 flex-1 overflow-auto">
        {tracks.length === 0 ? (
          <p className="p-3 text-[12px] text-muted">
            {data.smart
              ? "no tracks match the rules yet"
              : "empty — stage tracks with a, then save the queue here, or use add-to-playlist"}
          </p>
        ) : (
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
                    key={`${track.id}-${i}`}
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
        )}
        <SelectionBar selected={selected} onClear={clear} />
      </div>
    </div>
  );
}
