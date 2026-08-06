import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";

import { TrackRow } from "@/components/library/TrackRow";
import { Loading } from "@/components/ui/States";
import { useMainTitle } from "@/hooks/useMainTitle";
import { api } from "@/ipc/invoke";
import type { Track } from "@/ipc/types";
import { registerListHandler } from "@/lib/keyboard";
import { cn, errText } from "@/lib/utils";
import { toast } from "@/stores/toastStore";

export function FoldersView() {
  const [path, setPath] = useState<string | null>(null);
  const [cursor, setCursor] = useState(0);
  const [confirmRemove, setConfirmRemove] = useState<string | null>(null);
  const queryClient = useQueryClient();

  const removeDir = async (dirPath: string) => {
    const removed = await api.removeFolder(dirPath);
    setConfirmRemove(null);
    toast.ok(`${removed} tracks removed from library (files stay)`);
    await queryClient.invalidateQueries();
  };

  const { data, isLoading, error } = useQuery({
    queryKey: ["folder", path],
    queryFn: () => api.browseFolder(path ?? undefined),
  });

  const relative =
    data && data.path !== data.root
      ? data.path.slice(data.root.length).replace(/^\//, "")
      : "";
  useMainTitle(relative ? `folders · ${relative}` : "folders");

  // one combined list for the vim cursor: dirs first, tracks after
  const dirs = data?.dirs ?? [];
  const tracks = data?.tracks ?? [];

  const stateRef = useRef({ dirs, tracks, cursor, path, root: data?.root });
  stateRef.current = { dirs, tracks, cursor, path, root: data?.root };

  const enterAt = (index: number) => {
    const s = stateRef.current;
    if (index < s.dirs.length) {
      const dir = s.dirs[index];
      if (dir) {
        setPath(dir.path);
        setCursor(0);
      }
      return;
    }
    const trackIdx = index - s.dirs.length;
    const ids = s.tracks.map((t: Track) => t.id);
    if (ids.length > 0 && s.tracks[trackIdx]) {
      void api.playContext(ids, trackIdx);
    }
  };

  const trackAtCursor = () => {
    const s = stateRef.current;
    return s.tracks[s.cursor - s.dirs.length];
  };

  const goUp = () => {
    const s = stateRef.current;
    if (!s.path || !s.root) return false;
    // at a root's own top, up means the root list — with several roots that is
    // a real level, and with one it just re-renders where you already are
    if (s.path === s.root) {
      setPath(null);
      setCursor(0);
      return true;
    }
    const parent = s.path.slice(0, s.path.lastIndexOf("/"));
    setPath(parent === s.root ? null : parent);
    setCursor(0);
    return true;
  };

  useEffect(() => {
    return registerListHandler({
      move: (delta) =>
        setCursor((c) => {
          const total =
            stateRef.current.dirs.length + stateRef.current.tracks.length;
          return Math.min(Math.max(c + delta, 0), Math.max(total - 1, 0));
        }),
      top: () => setCursor(0),
      bottom: () =>
        setCursor(
          Math.max(
            stateRef.current.dirs.length + stateRef.current.tracks.length - 1,
            0,
          ),
        ),
      open: () => enterAt(stateRef.current.cursor),
      stage: () => {
        const track = trackAtCursor();
        if (track) void api.queueAdd(track.id);
      },
      fav: () => {
        const track = trackAtCursor();
        if (track) {
          void api
            .toggleFavorite(track.id)
            .then(() => queryClient.invalidateQueries());
        }
      },
      rate: (rating) => {
        const track = trackAtCursor();
        if (track) {
          void api
            .setRating(track.id, rating)
            .then(() => queryClient.invalidateQueries());
        }
      },
      back: () => {
        goUp();
      },
    });
    // handlers read stateRef only
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (isLoading) {
    return <Loading />;
  }
  if (error || !data) {
    return (
      <p className="p-3 text-[12px] text-error">
        {error ? errText(error) : "no library folders yet — add one in settings"}
      </p>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <Breadcrumb
        root={data.root}
        path={data.path}
        onNavigate={(p) => {
          setPath(p);
          setCursor(0);
        }}
      />
      <div className="min-h-0 flex-1 overflow-auto">
        {dirs.map((dir, i) => (
          <div
            key={dir.path}
            onClick={() => setCursor(i)}
            onDoubleClick={() => enterAt(i)}
            className={cn(
              "group flex h-7 cursor-default items-center gap-2 border-l-2 px-2",
              cursor === i
                ? "border-focus bg-raised"
                : "border-transparent hover:bg-raised/50",
            )}
          >
            <span className="text-muted">▸</span>
            <span className="min-w-0 flex-1 truncate text-[12px] text-primary">
              {dir.name}
            </span>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                if (confirmRemove === dir.path) {
                  void removeDir(dir.path);
                } else {
                  setConfirmRemove(dir.path);
                }
              }}
              onMouseLeave={() =>
                setConfirmRemove((c) => (c === dir.path ? null : c))
              }
              title="remove this folder's tracks from the library (files stay on disk)"
              className={cn(
                "shrink-0 border px-1.5 text-[10px]",
                confirmRemove === dir.path
                  ? "border-error text-error"
                  : "border-subtle text-muted opacity-0 hover:text-error group-hover:opacity-100",
              )}
            >
              {confirmRemove === dir.path ? "sure? click again" : "remove"}
            </button>
            <span className="shrink-0 text-[11px] text-muted">
              {dir.trackCount} tracks
            </span>
          </div>
        ))}
        {tracks.length > 0 && (
          <table className="w-full border-collapse">
            <tbody>
              {tracks.map((track, i) => (
                <TrackRow
                  key={track.id}
                  track={track}
                  selected={cursor === dirs.length + i}
                  onSelect={() => setCursor(dirs.length + i)}
                  onPlay={() => enterAt(dirs.length + i)}
                />
              ))}
            </tbody>
          </table>
        )}
        {dirs.length === 0 && tracks.length === 0 && (
          <p className="p-3 text-[12px] text-muted">empty folder</p>
        )}
      </div>
    </div>
  );
}

function Breadcrumb({
  root,
  path,
  onNavigate,
}: {
  root: string;
  path: string;
  onNavigate: (path: string | null) => void;
}) {
  const relative = path.startsWith(root) ? path.slice(root.length) : path;
  const parts = relative.split("/").filter(Boolean);

  return (
    <div className="flex h-7 shrink-0 items-center gap-1 overflow-x-auto border-b border-subtle px-2 text-[11px]">
      <button
        type="button"
        onClick={() => onNavigate(null)}
        className={cn(
          parts.length === 0 ? "text-accent" : "text-secondary hover:text-accent",
        )}
      >
        ~music
      </button>
      {parts.map((part, i) => {
        const target = `${root}/${parts.slice(0, i + 1).join("/")}`;
        const last = i === parts.length - 1;
        return (
          <span key={target} className="flex items-center gap-1">
            <span className="text-muted">/</span>
            <button
              type="button"
              onClick={() => onNavigate(last ? target : target)}
              className={cn(
                "max-w-40 truncate",
                last ? "text-accent" : "text-secondary hover:text-accent",
              )}
            >
              {part}
            </button>
          </span>
        );
      })}
    </div>
  );
}
