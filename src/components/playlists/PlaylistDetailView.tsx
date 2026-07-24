import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useParams } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";

import { TrackRow } from "@/components/library/TrackRow";
import { EditableText } from "@/components/ui/EditableText";
import { api } from "@/ipc/invoke";
import type { Track } from "@/ipc/types";
import { registerListHandler } from "@/lib/keyboard";

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

  const tracks = data?.tracks ?? [];
  const tracksRef = useRef<Track[]>(tracks);
  tracksRef.current = tracks;
  const cursorRef = useRef(cursor);
  cursorRef.current = cursor;

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
        setCursor((c) =>
          Math.min(Math.max(c + delta, 0), tracksRef.current.length - 1),
        ),
      top: () => setCursor(0),
      bottom: () => setCursor(tracksRef.current.length - 1),
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
      back: () => void navigate({ to: "/playlists" }),
    });
    // handlers read refs only
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [navigate, smart, id]);

  if (isLoading || !data) {
    return <p className="p-3 text-muted">loading…</p>;
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
          <span className="rounded-[var(--radius-sm)] bg-raised px-1.5 py-0.5 text-[10px] text-hires">
            smart
          </span>
        )}
        <span className="text-[12px] text-muted">
          {tracks.length} tracks
        </span>
      </header>
      <div className="min-h-0 flex-1 overflow-auto">
        {tracks.length === 0 ? (
          <p className="p-3 text-[12px] text-muted">
            {data.smart
              ? "no tracks match the rules yet"
              : "empty — stage tracks with a, then save the queue here, or use add-to-playlist"}
          </p>
        ) : (
          <table className="w-full border-collapse">
            <tbody>
              {tracks.map((track, i) => (
                <TrackRow
                  key={`${track.id}-${i}`}
                  track={track}
                  selected={i === cursor}
                  onSelect={() => setCursor(i)}
                  onPlay={() => playFrom(i)}
                />
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
