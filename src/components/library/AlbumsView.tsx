import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";

import { ScanForm } from "@/components/library/ScanForm";
import { CoverPlaceholder } from "@/components/ui/CoverPlaceholder";
import { EditableText } from "@/components/ui/EditableText";
import { EqBars } from "@/components/ui/HeartEqualizer";
import { Loading } from "@/components/ui/States";
import { useMainTitle } from "@/hooks/useMainTitle";
import { api } from "@/ipc/invoke";
import type { AlbumSummary } from "@/ipc/types";
import { artworkUrl } from "@/lib/artwork";
import { registerListHandler } from "@/lib/keyboard";
import { cn } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";
import { useScanStore } from "@/stores/scanStore";

type AlbumSort = "artist" | "year" | "added" | "name";

const SORT_KEY = "albums.sort";
const SORTS: { key: AlbumSort; label: string }[] = [
  { key: "artist", label: "artist" },
  { key: "year", label: "year" },
  { key: "added", label: "recent" },
  { key: "name", label: "name" },
];

/** Live column count of the auto-fill grid, so j/k step a whole row. */
function gridColumns(el: HTMLElement | null): number {
  if (!el) return 1;
  const template = getComputedStyle(el).gridTemplateColumns;
  return Math.max(template.split(" ").filter(Boolean).length, 1);
}

function sortAlbums(albums: AlbumSummary[], sort: AlbumSort): AlbumSummary[] {
  const sorted = [...albums];
  switch (sort) {
    case "year":
      sorted.sort((a, b) => (b.year ?? 0) - (a.year ?? 0));
      break;
    case "added":
      sorted.sort((a, b) => b.addedAt.localeCompare(a.addedAt));
      break;
    case "name":
      sorted.sort((a, b) => a.name.localeCompare(b.name));
      break;
    default:
      break; // backend order: artist, year, name
  }
  return sorted;
}

export function AlbumsView() {
  useMainTitle("albums");
  const scanning = useScanStore((s) => s.scanning);
  const status = usePlayerStore((s) => s.status);
  const trackId = usePlayerStore((s) => s.trackId);
  const navigate = useNavigate();
  const [cursor, setCursor] = useState(0);
  const gridRef = useRef<HTMLDivElement>(null);
  const [sort, setSort] = useState<AlbumSort>(
    () => (localStorage.getItem(SORT_KEY) as AlbumSort) || "artist",
  );
  const { data: albums, isLoading } = useQuery({
    queryKey: ["albums"],
    queryFn: api.listAlbums,
  });
  const { data: nowPlaying } = useQuery({
    queryKey: ["track", trackId],
    queryFn: () => api.getTrack(trackId ?? -1),
    enabled: trackId !== null,
    staleTime: Infinity,
  });
  const playingAlbumId =
    status !== "stopped" ? nowPlaying?.track.albumId : undefined;

  const sorted = sortAlbums(albums ?? [], sort);
  const sortedRef = useRef<AlbumSummary[]>(sorted);
  sortedRef.current = sorted;
  const cursorRef = useRef(cursor);
  cursorRef.current = cursor;

  useEffect(() => {
    const step = (delta: number, byRow: boolean) =>
      setCursor((c) => {
        const size = byRow ? gridColumns(gridRef.current) : 1;
        const next = Math.min(
          Math.max(c + delta * size, 0),
          Math.max(sortedRef.current.length - 1, 0),
        );
        gridRef.current?.children[next]?.scrollIntoView({ block: "nearest" });
        return next;
      });

    return registerListHandler({
      move: (delta) => step(delta, true),
      moveCol: (delta) => step(delta, false),
      top: () => setCursor(0),
      bottom: () => setCursor(Math.max(sortedRef.current.length - 1, 0)),
      open: () => {
        const album = sortedRef.current[cursorRef.current];
        if (album) {
          void navigate({
            to: "/albums/$albumId",
            params: { albumId: String(album.id) },
          });
        }
      },
      stage: () => {
        const album = sortedRef.current[cursorRef.current];
        if (!album) return;
        void api.getAlbum(album.id).then(async (detail) => {
          for (const track of detail.tracks) {
            await api.queueAdd(track.id);
          }
        });
      },
    });
  }, [navigate]);

  if (isLoading) {
    return <Loading />;
  }
  if (!albums || albums.length === 0) {
    return scanning ? (
      <p className="p-3 text-muted">scanning…</p>
    ) : (
      <ScanForm />
    );
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex h-7 shrink-0 items-center gap-1 border-b border-subtle px-3 text-[10px]">
        <span className="text-muted">sort:</span>
        {SORTS.map(({ key, label }) => (
          <button
            key={key}
            type="button"
            onClick={() => {
              setSort(key);
              localStorage.setItem(SORT_KEY, key);
            }}
            className={cn(
              "px-1.5 py-0.5",
              sort === key
                ? "bg-raised text-accent"
                : "text-muted hover:text-secondary",
            )}
          >
            {label}
          </button>
        ))}
      </div>
      <div
        ref={gridRef}
        className="grid min-h-0 flex-1 grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-3 overflow-auto p-3"
      >
        {sorted.map((album, i) => (
          <AlbumCard
            key={album.id}
            album={album}
            playing={album.id === playingAlbumId}
            animate={status === "playing"}
            focused={i === cursor}
          />
        ))}
      </div>
    </div>
  );
}

function AlbumCard({
  album,
  playing,
  animate,
  focused,
}: {
  album: AlbumSummary;
  playing: boolean;
  animate: boolean;
  focused: boolean;
}) {
  const [artError, setArtError] = useState(false);
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const open = () =>
    void navigate({
      to: "/albums/$albumId",
      params: { albumId: String(album.id) },
    });

  return (
    <div className="group flex cursor-pointer flex-col gap-1" onClick={open}>
      <div
        className={cn(
          "relative aspect-square overflow-hidden border bg-raised",
          focused
            ? "border-focus ring-1 ring-focus"
            : playing
              ? "border-accent"
              : "border-subtle group-hover:border-focus",
        )}
      >
        {album.artworkPath && !artError ? (
          <img
            src={artworkUrl(album.id)}
            alt=""
            loading="lazy"
            onError={() => setArtError(true)}
            className="h-full w-full object-cover"
          />
        ) : (
          <CoverPlaceholder name={album.name} className="text-2xl" />
        )}
        {playing && (
          <span
            title="now playing"
            className="absolute bottom-1 left-1 flex items-center border border-subtle bg-base/90 px-1 py-0.5"
          >
            <EqBars playing={animate} />
          </span>
        )}
        {album.artistCount > 1 && (
          <span
            title={`compilation · ${album.artistCount} artists`}
            className="absolute right-1 top-1 border border-subtle bg-base/90 px-1 text-[9px] text-hires"
          >
            VA
          </span>
        )}
      </div>
      <EditableText
        value={album.name}
        className={cn(
          "text-[12px] group-hover:text-accent",
          playing ? "text-accent" : "text-primary",
        )}
        inputClassName="w-full text-[12px] text-primary"
        onSave={async (name) => {
          await api.renameAlbum(album.id, name);
          await queryClient.invalidateQueries();
        }}
      />
      <span className="truncate text-[11px] text-muted">
        {album.artistName}
        {album.year ? ` · ${album.year}` : ""}
      </span>
    </div>
  );
}
