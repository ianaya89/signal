import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import { ScanForm } from "@/components/library/ScanForm";
import { EditableText } from "@/components/ui/EditableText";
import { api } from "@/ipc/invoke";
import type { AlbumSummary } from "@/ipc/types";
import { artworkUrl } from "@/lib/artwork";
import { useMainTitle } from "@/hooks/useMainTitle";
import { useScanStore } from "@/stores/scanStore";

export function AlbumsView() {
  useMainTitle("albums");
  const scanning = useScanStore((s) => s.scanning);
  const { data: albums, isLoading } = useQuery({
    queryKey: ["albums"],
    queryFn: api.listAlbums,
  });

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
        <AlbumCard key={album.id} album={album} />
      ))}
    </div>
  );
}

function AlbumCard({ album }: { album: AlbumSummary }) {
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
      <div className="aspect-square overflow-hidden border border-subtle bg-raised group-hover:border-focus">
        {album.artworkPath && !artError ? (
          <img
            src={artworkUrl(album.id)}
            alt=""
            loading="lazy"
            onError={() => setArtError(true)}
            className="h-full w-full object-cover"
          />
        ) : (
          <div className="flex h-full w-full items-center justify-center text-2xl text-muted">
            ♪
          </div>
        )}
      </div>
      <EditableText
        value={album.name}
        className="text-[12px] text-primary group-hover:text-accent"
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
