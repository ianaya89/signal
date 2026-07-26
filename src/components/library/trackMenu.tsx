import type { QueryClient } from "@tanstack/react-query";

import type { MenuItem } from "@/components/ui/ContextMenu";
import { api } from "@/ipc/invoke";
import type { PlaylistSummary, Track } from "@/ipc/types";
import { useEditStore } from "@/stores/editStore";
import { toast } from "@/stores/toastStore";

interface TrackMenuDeps {
  track: Track;
  playlists: PlaylistSummary[] | undefined;
  queryClient: QueryClient;
  navigate: (opts: { to: string; params?: Record<string, string> }) => void;
  onPlay?: () => void;
}

/** Shared right-click menu for any track row. */
export function buildTrackMenu({
  track,
  playlists,
  queryClient,
  navigate,
  onPlay,
}: TrackMenuDeps): MenuItem[] {
  const statics = (playlists ?? []).filter((p) => !p.smart);

  return [
    {
      label: "play",
      onClick: () => (onPlay ? onPlay() : void api.play(track.id)),
    },
    {
      label: "play next",
      onClick: () => {
        void api.queueAddNext(track.id).then(() => toast.ok("staged next"));
      },
    },
    {
      label: "add to queue (a)",
      onClick: () => {
        void api.queueAdd(track.id).then(() => toast.ok("staged to queue"));
      },
    },
    {
      label: "add to playlist",
      disabled: statics.length === 0,
      submenu: statics.map((p) => ({
        label: p.name,
        onClick: () => {
          void api
            .playlistAddTracks(p.id, [track.id])
            .then(() => {
              toast.ok(`added to ${p.name}`);
              void queryClient.invalidateQueries({ queryKey: ["playlists"] });
              void queryClient.invalidateQueries({ queryKey: ["playlist"] });
            })
            .catch(() => toast.error("could not add"));
        },
      })),
    },
    { label: "", separator: true },
    {
      label: "go to album",
      disabled: track.albumId <= 0,
      onClick: () =>
        navigate({
          to: "/albums/$albumId",
          params: { albumId: String(track.albumId) },
        }),
    },
    {
      label: "go to artist",
      onClick: () =>
        navigate({
          to: "/artists/$artistId",
          params: { artistId: String(track.artistId) },
        }),
    },
    { label: "", separator: true },
    {
      label: "rate",
      submenu: [
        {
          label: "✦ like",
          onClick: () => {
            void api.setRating(track.id, 4).then(() => {
              toast.ok("✦ liked");
              void queryClient.invalidateQueries();
            });
          },
        },
        {
          label: "✦✦ love",
          onClick: () => {
            void api.setRating(track.id, 5).then(() => {
              toast.ok("✦✦ loved");
              void queryClient.invalidateQueries();
            });
          },
        },
        {
          label: "clear rating",
          onClick: () => {
            void api.setRating(track.id, 0).then(() => {
              void queryClient.invalidateQueries();
            });
          },
        },
      ],
    },
    {
      label: "edit metadata…",
      onClick: () => useEditStore.getState().openTrack(track.id),
    },
    { label: "", separator: true },
    {
      label: "reveal in finder",
      onClick: () => {
        void api
          .revealFile(track.technical.filePath)
          .catch(() => toast.error("file not found"));
      },
    },
  ];
}
