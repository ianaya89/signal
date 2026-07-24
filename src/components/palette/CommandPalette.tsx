import { useNavigate } from "@tanstack/react-router";
import { useEffect, useMemo, useRef, useState } from "react";

import { api } from "@/ipc/invoke";
import { useKeyboardStore } from "@/lib/keyboard";
import { pickFolder } from "@/lib/pickFolder";
import { cn } from "@/lib/utils";
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
        label: "rescan library",
        run: async () => {
          const root = await api.settingsGet("library.root");
          if (!root) {
            useScanStore.getState().fail("no previous scan — use scan folder…");
            return;
          }
          useScanStore.getState().start();
          await api.scanLibrary(root);
        },
      },
    ],
    [navigate],
  );

  const open = mode === "palette";

  const matches = useMemo(() => {
    const q = input.trim().toLowerCase();
    if (!q) return commands;
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
      className="absolute inset-0 z-50 flex items-start justify-center bg-black/30 pt-[15vh] backdrop-blur-[2px]"
      onMouseDown={() => setMode("normal")}
    >
      <div
        className="w-[560px] overflow-hidden rounded-[var(--radius)] border border-focus/80 bg-raised shadow-[0_18px_50px_-12px_rgba(0,0,0,0.55),0_0_0_1px_color-mix(in_srgb,var(--accent)_20%,transparent)]"
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
