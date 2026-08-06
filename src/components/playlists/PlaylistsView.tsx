import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";

import { SmartEditor } from "@/components/playlists/SmartEditor";
import { EditableText } from "@/components/ui/EditableText";
import { Loading } from "@/components/ui/States";
import { BTN, BTN_PRIMARY, INPUT } from "@/components/ui/controls";
import { useMainTitle } from "@/hooks/useMainTitle";
import { api } from "@/ipc/invoke";
import type { PlaylistSummary, SmartRules } from "@/ipc/types";
import { registerListHandler } from "@/lib/keyboard";
import { pickM3u } from "@/lib/pickFolder";
import { cn, errText } from "@/lib/utils";
import { toast } from "@/stores/toastStore";

export function PlaylistsView() {
  useMainTitle("playlists");
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const { data: playlists, isLoading } = useQuery({
    queryKey: ["playlists"],
    queryFn: api.playlistList,
  });
  const [name, setName] = useState("");
  const [smartOpen, setSmartOpen] = useState(false);
  const [cursor, setCursor] = useState(0);
  const containerRef = useRef<HTMLDivElement>(null);

  const smart = playlists?.filter((p) => p.smart) ?? [];
  const statics = playlists?.filter((p) => !p.smart) ?? [];
  // one flat cursor across both sections: smart first, then static
  const ordered = [...smart, ...statics];
  const orderedRef = useRef<PlaylistSummary[]>(ordered);
  orderedRef.current = ordered;
  const cursorRef = useRef(cursor);
  cursorRef.current = cursor;

  useEffect(() => {
    const goTo = (index: number) => {
      containerRef.current
        ?.querySelector(`[data-idx="${index}"]`)
        ?.scrollIntoView({ block: "nearest" });
      return index;
    };
    return registerListHandler({
      move: (delta) =>
        setCursor((c) =>
          goTo(
            Math.min(
              Math.max(c + delta, 0),
              Math.max(orderedRef.current.length - 1, 0),
            ),
          ),
        ),
      top: () => setCursor(goTo(0)),
      bottom: () => setCursor(goTo(Math.max(orderedRef.current.length - 1, 0))),
      open: () => {
        const playlist = orderedRef.current[cursorRef.current];
        if (playlist) {
          void navigate({
            to: "/playlists/$kind/$playlistId",
            params: {
              kind: playlist.smart ? "smart" : "static",
              playlistId: String(playlist.id),
            },
          });
        }
      },
    });
  }, [navigate]);

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
      toast.error(errText(err));
    }
  };

  if (isLoading) {
    return <Loading />;
  }

  return (
    <div ref={containerRef} className="flex flex-col gap-3 p-3">
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
          className={cn("w-64", INPUT)}
        />
        <button
          type="submit"
          disabled={!name.trim()}
          className={BTN_PRIMARY}
        >
          create
        </button>
        <button
          type="button"
          onClick={() => setSmartOpen((v) => !v)}
          className={cn(BTN, "hover:text-hires")}
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
                toast.error(errText(err));
              }
            })();
          }}
          title="import an .m3u/.m3u8 — lines match against library file paths"
          className={BTN}
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
        indexOffset={0}
        cursor={cursor}
        onFocus={setCursor}
        onChanged={() =>
          void queryClient.invalidateQueries({ queryKey: ["playlists"] })
        }
      />
      <Section
        title="playlists"
        items={statics}
        indexOffset={smart.length}
        cursor={cursor}
        onFocus={setCursor}
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
  indexOffset,
  cursor,
  onFocus,
  onChanged,
}: {
  title: string;
  items: { id: number; name: string; trackCount: number; smart: boolean }[];
  smartBadge?: boolean;
  indexOffset: number;
  cursor: number;
  onFocus: (index: number) => void;
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
        {items.map((p, i) => {
          const index = indexOffset + i;
          return (
          <li
            key={`${p.smart}-${p.id}`}
            data-idx={index}
            className="group flex items-center"
          >
            <div
              onClick={() => {
                onFocus(index);
                void navigate({
                  to: "/playlists/$kind/$playlistId",
                  params: {
                    kind: p.smart ? "smart" : "static",
                    playlistId: String(p.id),
                  },
                });
              }}
              className={cn(
                "flex h-7 min-w-0 flex-1 cursor-pointer items-center justify-between border-l-2 px-2",
                index === cursor
                  ? "border-focus bg-raised"
                  : "border-transparent hover:bg-raised/50",
              )}
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
          );
        })}
      </ul>
    </section>
  );
}
