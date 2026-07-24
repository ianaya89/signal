import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";

import { EditableText } from "@/components/ui/EditableText";
import { api } from "@/ipc/invoke";

export function ArtistsView() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
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
          onClick={() =>
            void navigate({
              to: "/artists/$artistId",
              params: { artistId: String(artist.id) },
            })
          }
          className="flex h-7 cursor-pointer items-center justify-between px-3 hover:bg-raised"
        >
          <EditableText
            value={artist.name}
            className="min-w-0 text-[12px] text-primary"
            inputClassName="w-56 text-[12px] text-primary"
            onSave={async (name) => {
              await api.renameArtist(artist.id, name);
              await queryClient.invalidateQueries();
            }}
          />
          <span className="shrink-0 text-[11px] text-muted">
            {artist.albumCount} albums · {artist.trackCount} tracks
          </span>
        </div>
      ))}
    </div>
  );
}
