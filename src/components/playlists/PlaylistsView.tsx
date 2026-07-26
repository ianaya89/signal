import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import { SmartEditor } from "@/components/playlists/SmartEditor";
import { EditableText } from "@/components/ui/EditableText";
import { useMainTitle } from "@/hooks/useMainTitle";
import { api } from "@/ipc/invoke";
import type { SmartRules } from "@/ipc/types";
import { pickM3u } from "@/lib/pickFolder";
import { toast } from "@/stores/toastStore";

export function PlaylistsView() {
  useMainTitle("playlists");
  const queryClient = useQueryClient();
  const { data: playlists, isLoading } = useQuery({
    queryKey: ["playlists"],
    queryFn: api.playlistList,
  });
  const [name, setName] = useState("");
  const [smartOpen, setSmartOpen] = useState(false);

  const create = useMutation({
    mutationFn: (n: string) => api.playlistCreate(n),
    onSuccess: () => {
      setName("");
      void queryClient.invalidateQueries({ queryKey: ["playlists"] });
    },
  });

  const saveSmart = async (smartName: string, rules: SmartRules) => {
    try {
      await api.smartCreate(smartName, JSON.stringify(rules));
      setSmartOpen(false);
      toast.ok(`smart playlist "${smartName}" created`);
      void queryClient.invalidateQueries({ queryKey: ["playlists"] });
    } catch (err) {
      toast.error(String(err));
    }
  };

  if (isLoading) {
    return <p className="p-3 text-muted">loading…</p>;
  }

  const smart = playlists?.filter((p) => p.smart) ?? [];
  const statics = playlists?.filter((p) => !p.smart) ?? [];

  return (
    <div className="flex flex-col gap-3 p-3">
      <form
        className="flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          if (name.trim()) create.mutate(name);
        }}
      >
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="new playlist name…"
          spellCheck={false}
          className="w-64 rounded-[var(--radius-sm)] border border-subtle bg-base/60 px-2 py-1 text-[12px] text-primary outline-none focus:border-focus"
        />
        <button
          type="submit"
          className="rounded-[var(--radius-sm)] border border-subtle bg-raised px-3 py-1 text-[12px] text-secondary hover:border-focus hover:text-accent"
        >
          create
        </button>
        <button
          type="button"
          onClick={() => setSmartOpen((v) => !v)}
          className="rounded-[var(--radius-sm)] border border-subtle bg-raised px-3 py-1 text-[12px] text-secondary hover:border-focus hover:text-hires"
        >
          + smart
        </button>
        <button
          type="button"
          onClick={() => {
            void (async () => {
              const file = await pickM3u();
              if (!file) return;
              try {
                const result = await api.importM3u(file);
                toast.ok(
                  `"${result.name}" imported · ${result.matched}/${result.total} matched`,
                );
                void queryClient.invalidateQueries({ queryKey: ["playlists"] });
              } catch (err) {
                toast.error(String(err));
              }
            })();
          }}
          title="import an .m3u/.m3u8 — lines match against library file paths"
          className="rounded-[var(--radius-sm)] border border-subtle bg-raised px-3 py-1 text-[12px] text-secondary hover:border-focus hover:text-accent"
        >
          import m3u…
        </button>
      </form>

      {smartOpen && (
        <SmartEditor onSave={saveSmart} onCancel={() => setSmartOpen(false)} />
      )}

      <Section
        title="smart"
        items={smart}
        smartBadge
        onChanged={() =>
          void queryClient.invalidateQueries({ queryKey: ["playlists"] })
        }
      />
      <Section
        title="playlists"
        items={statics}
        onChanged={() =>
          void queryClient.invalidateQueries({ queryKey: ["playlists"] })
        }
      />
    </div>
  );
}

function Section({
  title,
  items,
  smartBadge = false,
  onChanged,
}: {
  title: string;
  items: { id: number; name: string; trackCount: number; smart: boolean }[];
  smartBadge?: boolean;
  onChanged?: () => void;
}) {
  const navigate = useNavigate();
  if (items.length === 0) return null;
  return (
    <section>
      <h2 className="mb-1 text-[10px] uppercase tracking-wider text-muted">
        {title}
      </h2>
      <ul className="flex flex-col gap-px">
        {items.map((p) => (
          <li key={`${p.smart}-${p.id}`} className="group flex items-center">
            <div
              onClick={() =>
                void navigate({
                  to: "/playlists/$kind/$playlistId",
                  params: {
                    kind: p.smart ? "smart" : "static",
                    playlistId: String(p.id),
                  },
                })
              }
              className="flex h-7 min-w-0 flex-1 cursor-pointer items-center justify-between px-2 hover:bg-raised"
            >
              <span className="flex min-w-0 items-center gap-2">
                {p.smart ? (
                  <span className="truncate text-[12px] text-primary">
                    {p.name}
                  </span>
                ) : (
                  <EditableText
                    value={p.name}
                    className="min-w-0 text-[12px] text-primary"
                    inputClassName="w-48 text-[12px] text-primary"
                    onSave={async (name) => {
                      await api.playlistRename(p.id, name);
                      onChanged?.();
                    }}
                  />
                )}
                {smartBadge && (
                  <span className="shrink-0 bg-raised px-1 text-[9px] text-hires">
                    smart
                  </span>
                )}
              </span>
              {!p.smart && (
                <span className="shrink-0 text-[11px] text-muted">
                  {p.trackCount} tracks
                </span>
              )}
            </div>
            {onChanged && (
              <button
                type="button"
                onClick={() => {
                  void (p.smart
                    ? api.smartDelete(p.id)
                    : api.playlistDelete(p.id)
                  ).then(onChanged);
                }}
                title={p.smart ? "delete smart playlist" : "delete playlist"}
                className="hidden px-2 text-[13px] text-muted hover:text-error group-hover:block"
              >
                ✕
              </button>
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}
