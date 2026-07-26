import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import { ScanForm } from "@/components/library/ScanForm";
import { CoverPlaceholder } from "@/components/ui/CoverPlaceholder";
import { EditableText } from "@/components/ui/EditableText";
import { EqBars } from "@/components/ui/HeartEqualizer";
import { api } from "@/ipc/invoke";
import type { AlbumSummary } from "@/ipc/types";
import { artworkUrl } from "@/lib/artwork";
import { cn } from "@/lib/utils";
import { useMainTitle } from "@/hooks/useMainTitle";
import { usePlayerStore } from "@/stores/playerStore";
import { useScanStore } from "@/stores/scanStore";

export function AlbumsView() {
  useMainTitle("albums");
  const scanning = useScanStore((s) => s.scanning);
  const status = usePlayerStore((s) => s.status);
  const trackId = usePlayerStore((s) => s.trackId);
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

  if (isLoading) {
    return <p className="p-3 text-muted">loading…</p>;
  }
  if (!albums || albums.length === 0) {
    return scanning ? (
      <p className="p-3 text-muted">scanning…</p>
    ) : (
      <ScanForm />
    );
  }

  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-3 p-3">
      {albums.map((album) => (
        <AlbumCard
          key={album.id}
          album={album}
          playing={album.id === playingAlbumId}
          animate={status === "playing"}
        />
      ))}
    </div>
  );
}

function AlbumCard({
  album,
  playing,
  animate,
}: {
  album: AlbumSummary;
  playing: boolean;
  animate: boolean;
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
          playing ? "border-accent" : "border-subtle group-hover:border-focus",
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
