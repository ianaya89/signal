import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useParams } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";

import { SelectionBar } from "@/components/library/SelectionBar";
import { TrackRow } from "@/components/library/TrackRow";
import { TrackTableHeader } from "@/components/library/TrackTableHeader";
import { EditableText } from "@/components/ui/EditableText";
import { api } from "@/ipc/invoke";
import type { Track } from "@/ipc/types";
import { artworkUrl } from "@/lib/artwork";
import { useMainTitle } from "@/hooks/useMainTitle";
import { useMultiSelect } from "@/hooks/useMultiSelect";
import { useTrackSort } from "@/hooks/useTrackSort";
import { registerListHandler } from "@/lib/keyboard";
import { pickImage } from "@/lib/pickFolder";

export function AlbumDetailView() {
  const { albumId } = useParams({ from: "/albums/$albumId" });
  const id = Number(albumId);
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [cursor, setCursor] = useState(0);
  const [artVersion, setArtVersion] = useState(0);
  const { data, isLoading } = useQuery({
    queryKey: ["album", id],
    queryFn: () => api.getAlbum(id),
  });

  useMainTitle(data ? `album · ${data.album.name}` : undefined);
  const sort = useTrackSort(data?.tracks ?? []);
  const tracks = sort.sorted;
  const tracksRef = useRef<Track[]>(tracks);
  tracksRef.current = tracks;
  const cursorRef = useRef(cursor);
  cursorRef.current = cursor;
  const { selected, handleRowClick, clear } = useMultiSelect(tracks);
  const selectedRef = useRef(selected);
  selectedRef.current = selected;
  const clearRef = useRef(clear);
  clearRef.current = clear;

  const playFrom = (index: number) => {
    const ids = tracksRef.current.map((t) => t.id);
    if (ids.length > 0) void api.playContext(ids, index);
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
      back: () => {
        if (selectedRef.current.size > 0) {
          clearRef.current();
          return;
        }
        void navigate({ to: "/" });
      },
    });
    // playFrom reads refs only
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [navigate]);

  if (isLoading || !data) {
    return <p className="p-3 text-muted">loading…</p>;
  }

  const { album } = data;

  return (
    <div className="flex h-full flex-col">
      <header className="flex shrink-0 items-end gap-3 border-b border-subtle p-3">
        <AlbumArt
          albumId={album.id}
          hasArt={album.artworkPath !== null}
          version={artVersion}
          onPick={async () => {
            const image = await pickImage();
            if (!image) return;
            await api.setAlbumArtwork(album.id, image);
            setArtVersion((v) => v + 1);
            await queryClient.invalidateQueries();
          }}
        />
        <div className="min-w-0">
          <h1 className="text-[16px] text-primary">
            <EditableText
              value={album.name}
              inputClassName="w-72 text-[16px] text-primary"
              onSave={async (name) => {
                await api.renameAlbum(album.id, name);
                await queryClient.invalidateQueries();
              }}
            />
          </h1>
          <p className="flex items-center gap-1 text-[12px] text-secondary">
            <EditableText
              value={album.artistName}
              inputClassName="w-48 text-[12px] text-secondary"
              onSave={async (name) => {
                await api.renameArtist(album.artistId, name);
                await queryClient.invalidateQueries();
              }}
            />
            <span className="shrink-0 text-muted">
              {album.year ? ` · ${album.year}` : ""} · {album.trackCount} tracks
            </span>
          </p>
        </div>
      </header>
      <div className="relative min-h-0 flex-1 overflow-auto">
        <table className="w-full border-collapse">
          <TrackTableHeader sort={sort} />
          <tbody>
            {tracks.map((track, i) => (
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
            ))}
          </tbody>
        </table>
        <SelectionBar selected={selected} onClear={clear} />
      </div>
    </div>
  );
}

function AlbumArt({
  albumId,
  hasArt,
  version,
  onPick,
}: {
  albumId: number;
  hasArt: boolean;
  version: number;
  onPick: () => Promise<void>;
}) {
  const [artError, setArtError] = useState(false);
  return (
    <button
      type="button"
      onClick={() => void onPick()}
      title="change artwork"
      className="group/art relative h-20 w-20 shrink-0 overflow-hidden rounded-[var(--radius-sm)] border border-subtle bg-raised hover:border-focus"
    >
      {(hasArt || version > 0) && !artError ? (
        <img
          src={`${artworkUrl(albumId)}?v=${version}`}
          alt=""
          onError={() => setArtError(true)}
          className="h-full w-full object-cover"
        />
      ) : (
        <div className="flex h-full w-full items-center justify-center text-xl text-muted">
          ♪
        </div>
      )}
      <span className="absolute inset-0 hidden items-center justify-center bg-black/50 text-[10px] text-primary group-hover/art:flex">
        change
      </span>
    </button>
  );
}
