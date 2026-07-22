import { useQuery } from "@tanstack/react-query";

import { api } from "@/ipc/invoke";

export function ArtistsView() {
  const { data: artists, isLoading } = useQuery({
    queryKey: ["artists"],
    queryFn: api.listArtists,
  });

  if (isLoading) {
    return <p className="p-3 text-muted">loading…</p>;
  }
  if (!artists || artists.length === 0) {
    return <p className="p-3 text-muted">no artists yet — scan your library first</p>;
  }

  return (
    <div className="py-1">
      {artists.map((artist) => (
        <div
          key={artist.id}
          className="flex h-7 cursor-default items-center justify-between px-3 hover:bg-raised"
        >
          <span className="truncate text-[12px] text-primary">{artist.name}</span>
          <span className="shrink-0 text-[11px] text-muted">
            {artist.albumCount} albums · {artist.trackCount} tracks
          </span>
        </div>
      ))}
    </div>
  );
}
