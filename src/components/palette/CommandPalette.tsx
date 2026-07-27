import { useNavigate } from "@tanstack/react-router";
import { useEffect, useMemo, useRef, useState } from "react";

import { api } from "@/ipc/invoke";
import { useKeyboardStore } from "@/lib/keyboard";
import { setWindowMode } from "@/lib/miniMode";
import { pickFolder } from "@/lib/pickFolder";
import { checkForUpdate, installUpdate } from "@/lib/updater";
import { cn } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";
import { useScanStore } from "@/stores/scanStore";
import { useUiStore } from "@/stores/uiStore";

interface Command {
  id: string;
  label: string;
  hint?: string;
  run: (arg?: string) => void | Promise<void>;
  /// Commands taking free text (e.g. "scan <path>") match on prefix.
  takesArg?: boolean;
}

export function CommandPalette() {
  const mode = useKeyboardStore((s) => s.mode);
  const setMode = useKeyboardStore((s) => s.setMode);
  const navigate = useNavigate();
  const [input, setInput] = useState("");
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const commands = useMemo<Command[]>(
    () => [
      {
        id: "play-pause",
        label: "play / pause",
        hint: "space",
        run: () => api.toggle(),
      },
      {
        id: "stop",
        label: "stop",
        run: () => api.stop(),
      },
      {
        id: "play-next",
        label: "queue: play next",
        run: async () => {
          await api.queuePlayNext();
        },
      },
      {
        id: "queue-clear",
        label: "queue: clear",
        run: () => api.queueClear(),
      },
      {
        id: "current",
        label: "go to current (now playing album)",
        run: async () => {
          const trackId = usePlayerStore.getState().trackId;
          if (trackId === null) throw new Error("nothing playing");
          const data = await api.getTrack(trackId);
          await navigate({
            to: "/albums/$albumId",
            params: { albumId: String(data.track.albumId) },
          });
        },
      },
      {
        id: "albums",
        label: "go to albums",
        run: () => navigate({ to: "/" }),
      },
      {
        id: "artists",
        label: "go to artists",
        run: () => navigate({ to: "/artists" }),
      },
      {
        id: "search",
        label: "go to search",
        hint: "/",
        run: () => navigate({ to: "/search" }),
      },
      {
        id: "stats",
        label: "go to stats",
        hint: "S",
        run: () => navigate({ to: "/stats" }),
      },
      {
        id: "logs",
        label: "go to logs",
        hint: "L",
        run: () => navigate({ to: "/logs" }),
      },
      {
        id: "playlists",
        label: "go to playlists",
        run: () => navigate({ to: "/playlists" }),
      },
      {
        id: "settings",
        label: "go to settings",
        run: () => navigate({ to: "/settings" }),
      },
      {
        id: "doctor",
        label: "library doctor",
        run: () => navigate({ to: "/doctor" }),
      },
      {
        id: "discover",
        label: "go to discover",
        hint: "D",
        run: () => navigate({ to: "/discover" }),
      },
      {
        id: "check-updates",
        label: "check for updates",
        run: async () => {
          await checkForUpdate();
        },
      },
      {
        id: "install-update",
        label: "install update + restart",
        run: () => installUpdate(),
      },
      {
        id: "edit-config",
        label: "edit config.toml",
        run: async () => {
          await api.openConfigFile();
        },
      },
      {
        id: "save-queue",
        label: "save-queue <name>",
        takesArg: true,
        run: async (arg) => {
          if (!arg?.trim()) throw new Error("usage: save-queue <name>");
          await api.queueSaveAsPlaylist(arg.trim());
        },
      },
      {
        id: "next",
        label: "next track",
        hint: "}",
        run: async () => {
          await api.next();
        },
      },
      {
        id: "theme",
        label: "theme: toggle dark / light",
        run: () => useUiStore.getState().toggleTheme(),
      },
      {
        id: "mini",
        label: "mini player",
        hint: "M",
        run: () => setWindowMode("mini"),
      },
      {
        id: "pulse",
        label: "pulse mode (floating dot)",
        hint: "P",
        run: () => setWindowMode("dot"),
      },
      {
        id: "toggle-library",
        label: "layout: toggle library pane",
        hint: "b",
        run: () => useUiStore.getState().togglePane("library"),
      },
      {
        id: "toggle-inspector",
        label: "layout: toggle inspector pane",
        hint: "i",
        run: () => useUiStore.getState().togglePane("inspector"),
      },
      {
        id: "scan-folder",
        label: "scan folder…",
        run: async () => {
          const folder = await pickFolder();
          if (folder) {
            useScanStore.getState().start();
            await api.scanLibrary(folder);
          }
        },
      },
      {
        id: "scan",
        label: "scan <path>",
        takesArg: true,
        run: async (arg) => {
          useScanStore.getState().start();
          await api.scanLibrary(arg ?? "~/Music");
        },
      },
      {
        id: "reset-library",
        label: "reset library (wipe + rescan)",
        run: async () => {
          useScanStore.getState().start();
          await api.resetAndRescan();
        },
      },
      {
        id: "rescan",
        label: "rescan library (all roots)",
        run: async () => {
          useScanStore.getState().start();
          await api.rescanAll();
        },
      },
      {
        id: "remove-folder",
        label: "remove-folder <path>",
        takesArg: true,
        run: async (arg) => {
          if (!arg?.trim()) throw new Error("usage: remove-folder <path>");
          await api.removeFolder(arg.trim());
        },
      },
    ],
    [navigate],
  );

  const open = mode === "palette";

  const matches = useMemo(() => {
    const q = input.trim().toLowerCase();
    if (!q) {
      // recently used commands float to the top
      const recent = loadRecent();
      return [...commands].sort((a, b) => {
        const ra = recent.indexOf(a.id);
        const rb = recent.indexOf(b.id);
        return (ra === -1 ? 99 : ra) - (rb === -1 ? 99 : rb);
      });
    }
    return commands.filter((c) => {
      if (c.takesArg) {
        const verb = c.id.toLowerCase();
        return verb.startsWith(q.split(" ")[0] ?? "") || fuzzy(c.label, q);
      }
      return fuzzy(c.label, q);
    });
  }, [commands, input]);

  useEffect(() => {
    if (open) {
      setInput("");
      setSelected(0);
      requestAnimationFrame(() => inputRef.current?.focus());
    }
  }, [open]);

  useEffect(() => {
    setSelected(0);
  }, [input]);

  if (!open) return null;

  const execute = async () => {
    const cmd = matches[selected];
    if (!cmd) return;
    saveRecent(cmd.id);
    setMode("normal");
    const arg = cmd.takesArg ? input.split(" ").slice(1).join(" ") || undefined : undefined;
    try {
      await cmd.run(arg);
    } catch (err) {
      const message =
        typeof err === "object" && err !== null && "message" in err
          ? String((err as { message: unknown }).message)
          : String(err);
      useScanStore.getState().fail(message);
    }
  };

  return (
    <div
      className="absolute inset-0 z-50 flex items-start justify-center bg-black/40 pt-[15vh]"
      onMouseDown={() => setMode("normal")}
    >
      <div
        className="w-[560px] overflow-hidden border border-focus bg-raised"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <input
          ref={inputRef}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              e.preventDefault();
              setMode("normal");
            } else if (e.key === "Enter") {
              e.preventDefault();
              void execute();
            } else if (e.key === "ArrowDown" || (e.key === "n" && e.ctrlKey)) {
              e.preventDefault();
              setSelected((s) => Math.min(s + 1, matches.length - 1));
            } else if (e.key === "ArrowUp" || (e.key === "p" && e.ctrlKey)) {
              e.preventDefault();
              setSelected((s) => Math.max(s - 1, 0));
            }
          }}
          placeholder="type a command…"
          spellCheck={false}
          className="w-full border-b border-subtle bg-transparent px-3 py-2 text-[13px] text-primary outline-none"
        />
        <ul className="max-h-72 overflow-auto py-1">
          {matches.map((cmd, i) => (
            <li
              key={cmd.id}
              onMouseEnter={() => setSelected(i)}
              onClick={() => void execute()}
              className={cn(
                "flex h-7 cursor-default items-center justify-between px-3 text-[12px]",
                i === selected ? "bg-surface text-accent" : "text-secondary",
              )}
            >
              <span>{cmd.label}</span>
              {cmd.hint && (
                <kbd className="rounded-sm bg-base px-1 text-[10px] text-muted">
                  {cmd.hint}
                </kbd>
              )}
            </li>
          ))}
          {matches.length === 0 && (
            <li className="px-3 py-2 text-[12px] text-muted">no matches</li>
          )}
        </ul>
      </div>
    </div>
  );
}

const RECENT_KEY = "palette.recent";

function loadRecent(): string[] {
  try {
    const raw = localStorage.getItem(RECENT_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed.filter((x) => typeof x === "string") : [];
  } catch {
    return [];
  }
}

function saveRecent(id: string) {
  const next = [id, ...loadRecent().filter((x) => x !== id)].slice(0, 8);
  try {
    localStorage.setItem(RECENT_KEY, JSON.stringify(next));
  } catch {
    // storage full/blocked: recents are a nicety, not a requirement
  }
}

function fuzzy(haystack: string, needle: string): boolean {
  let i = 0;
  const h = haystack.toLowerCase();
  for (const ch of needle) {
    if (ch === " ") continue;
    i = h.indexOf(ch, i);
    if (i === -1) return false;
    i += 1;
  }
  return true;
}
