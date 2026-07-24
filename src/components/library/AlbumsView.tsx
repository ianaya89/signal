import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useState } from "react";

import { ScanForm } from "@/components/library/ScanForm";
import { api } from "@/ipc/invoke";
import type { AlbumSummary } from "@/ipc/types";
import { artworkUrl } from "@/lib/artwork";
import { useScanStore } from "@/stores/scanStore";

export function AlbumsView() {
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

  return (
    <Link
      to="/albums/$albumId"
      params={{ albumId: String(album.id) }}
      className="group flex flex-col gap-1"
    >
      <div className="aspect-square overflow-hidden rounded-[var(--radius)] border border-subtle bg-raised transition-all duration-120 group-hover:-translate-y-0.5 group-hover:border-focus group-hover:shadow-[0_6px_20px_-6px_color-mix(in_srgb,var(--accent)_35%,transparent)]">
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
      <span className="truncate text-[12px] text-primary group-hover:text-accent">
        {album.name}
      </span>
      <span className="truncate text-[11px] text-muted">
        {album.artistName}
        {album.year ? ` · ${album.year}` : ""}
      </span>
    </Link>
  );
}
