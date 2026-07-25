import { useQuery, useQueryClient } from "@tanstack/react-query";

import { useContextMenu } from "@/components/ui/ContextMenu";
import { api } from "@/ipc/invoke";
import { toast } from "@/stores/toastStore";

/** Floating action bar shown while a multi-selection is active. */
export function SelectionBar({
  selected,
  onClear,
}: {
  selected: ReadonlySet<number>;
  onClear: () => void;
}) {
  const queryClient = useQueryClient();
  const { open: openMenu, menu } = useContextMenu();
  const { data: playlists } = useQuery({
    queryKey: ["playlists"],
    queryFn: api.playlistList,
    staleTime: 30_000,
  });

  if (selected.size === 0) return null;
  const ids = [...selected];

  const queueAll = async () => {
    for (const id of ids) {
      await api.queueAdd(id);
    }
    toast.ok(`${ids.length} tracks staged`);
    onClear();
  };

  return (
    <div className="absolute bottom-2 left-1/2 z-40 flex -translate-x-1/2 items-center gap-3 border border-focus bg-raised px-3 py-1.5 text-[11px]">
      <span className="text-accent">{selected.size} selected</span>
      <button
        type="button"
        onClick={() => void queueAll()}
        className="text-secondary hover:text-accent"
      >
        add to queue
      </button>
      <button
        type="button"
        onClick={(e) => {
          const statics = (playlists ?? []).filter((p) => !p.smart);
          openMenu(
            e,
            statics.length === 0
              ? [{ label: "no playlists yet", disabled: true }]
              : statics.map((p) => ({
                  label: p.name,
                  onClick: () => {
                    void api
                      .playlistAddTracks(p.id, ids)
                      .then(() => {
                        toast.ok(`${ids.length} added to ${p.name}`);
                        onClear();
                        return queryClient.invalidateQueries();
                      })
                      .catch(() => toast.error("could not add"));
                  },
                })),
          );
        }}
        className="text-secondary hover:text-accent"
      >
        add to playlist ▸
      </button>
      <button type="button" onClick={onClear} className="text-muted hover:text-error">
        clear (esc)
      </button>
      {menu}
    </div>
  );
}
