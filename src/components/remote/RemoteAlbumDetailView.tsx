import { useQuery } from "@tanstack/react-query";
import { useNavigate, useParams } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";

import { RemoteCover } from "@/components/remote/RemoteCover";
import { Failed, Loading } from "@/components/ui/States";
import { BTN_PRIMARY } from "@/components/ui/controls";
import { useMainTitle } from "@/hooks/useMainTitle";
import { api } from "@/ipc/invoke";
import type { SubsonicChild } from "@/ipc/types";
import { fmtDuration } from "@/lib/format";
import { registerListHandler } from "@/lib/keyboard";
import { cn, errText } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";
import { toast } from "@/stores/toastStore";

export function RemoteAlbumDetailView() {
  const { sourceId, albumId } = useParams({
    from: "/remote/$sourceId/albums/$albumId",
  });
  const id = Number(sourceId);
  const navigate = useNavigate();
  const [cursor, setCursor] = useState(0);
  const playingId = usePlayerStore((s) => s.trackId);

  const { data, isLoading, error } = useQuery({
    queryKey: ["remote-album", id, albumId],
    queryFn: () => api.remoteAlbum(id, albumId),
  });

  // remote ids are negative and opaque to the frontend, so the playing row is
  // matched by title — the same query the transport bar already caches
  const { data: nowPlaying } = useQuery({
    queryKey: ["track", playingId],
    queryFn: () => api.getTrack(playingId ?? -1),
    enabled: playingId !== null && playingId < 0,
  });

  useMainTitle(data ? `remote · ${data.name}` : undefined);

  const songs: SubsonicChild[] = data?.song ?? [];
  const songsRef = useRef<SubsonicChild[]>(songs);
  songsRef.current = songs;
  const cursorRef = useRef(cursor);
  cursorRef.current = cursor;

  const playFrom = (index: number) => {
    if (songsRef.current.length === 0) return;
    void api
      .remotePlayContext(id, songsRef.current, index)
      .catch((err) => toast.error(errText(err)));
  };

  useEffect(() => {
    return registerListHandler({
      move: (delta) =>
        setCursor((c) =>
          Math.min(Math.max(c + delta, 0), songsRef.current.length - 1),
        ),
      top: () => setCursor(0),
      bottom: () => setCursor(songsRef.current.length - 1),
      open: () => playFrom(cursorRef.current),
      back: () => void navigate({ to: "/remote" }),
    });
    // playFrom reads refs only
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [navigate]);

  if (isLoading) return <Loading />;
  if (error) {
    return <Failed error={error} />;
  }
  if (!data) return null;

  return (
    <div className="flex h-full flex-col">
      <header className="flex shrink-0 items-center gap-3 border-b border-subtle p-3">
        <RemoteCover
          sourceId={id}
          coverArt={data.coverArt}
          name={data.name}
          className="h-16 w-16"
        />
        <div className="min-w-0">
          <h1 className="truncate text-[16px] text-primary">{data.name}</h1>
          <p className="truncate text-[12px] text-secondary">
            {data.artist}
            {data.year ? ` · ${data.year}` : ""} · {songs.length} tracks
          </p>
        </div>
        <button
          type="button"
          onClick={() => playFrom(0)}
          className={cn("ml-auto", BTN_PRIMARY)}
        >
          play album
        </button>
      </header>
      <div className="min-h-0 flex-1 overflow-auto">
        <ul>
          {songs.map((song, i) => (
            <li
              key={song.id}
              onClick={() => setCursor(i)}
              onDoubleClick={() => playFrom(i)}
              className={cn(
                "flex h-7 cursor-pointer items-center gap-3 border-l-2 px-3",
                i === cursor
                  ? "border-focus bg-raised"
                  : "border-transparent hover:bg-raised",
              )}
            >
              <span className="w-6 shrink-0 text-right text-[10px] text-muted">
                {song.track ?? i + 1}
              </span>
              <span
                className={cn(
                  "min-w-0 flex-1 truncate text-[11px]",
                  nowPlaying?.track.title === song.title
                    ? "text-accent"
                    : "text-primary",
                )}
              >
                {song.title}
              </span>
              <span className="shrink-0 text-[10px] uppercase text-muted">
                {song.suffix}
              </span>
              <span className="w-10 shrink-0 text-right text-[10px] text-muted">
                {fmtDuration(song.duration * 1000)}
              </span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
