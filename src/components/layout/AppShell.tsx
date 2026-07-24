import { Outlet, useNavigate } from "@tanstack/react-router";
import { useEffect } from "react";

import { InspectorPane } from "@/components/layout/InspectorPane";
import { LibraryNav } from "@/components/layout/LibraryNav";
import { Pane } from "@/components/layout/Pane";
import { StatusBar } from "@/components/layout/StatusBar";
import { CommandPalette } from "@/components/palette/CommandPalette";
import { QueuePanel } from "@/components/queue/QueuePanel";
import { api } from "@/ipc/invoke";
import {
  armRating,
  currentListHandler,
  handleSequenceG,
  ratingArmed,
  useKeyboardStore,
} from "@/lib/keyboard";
import { usePlayerStore } from "@/stores/playerStore";
import { useUiStore } from "@/stores/uiStore";

// remembered across mute toggles (module-level; survives re-renders)
let lastVolume = 1;

export function AppShell() {
  const cycleFocus = useUiStore((s) => s.cycleFocus);
  const libraryVisible = useUiStore((s) => s.libraryVisible);
  const inspectorVisible = useUiStore((s) => s.inspectorVisible);
  const libraryWidth = useUiStore((s) => s.libraryWidth);
  const inspectorWidth = useUiStore((s) => s.inspectorWidth);
  const navigate = useNavigate();

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const { mode, setMode } = useKeyboardStore.getState();
      const target = e.target as HTMLElement;
      const inInput =
        target.tagName === "INPUT" || target.tagName === "TEXTAREA";

      // palette shortcut works everywhere
      if ((e.key === "p" && (e.ctrlKey || e.metaKey)) || (e.key === "k" && e.metaKey)) {
        e.preventDefault();
        setMode(mode === "palette" ? "normal" : "palette");
        return;
      }

      // input fields swallow everything else
      if (inInput || mode === "palette") return;

      switch (e.key) {
        case "Tab":
          e.preventDefault();
          cycleFocus(e.shiftKey ? -1 : 1);
          break;
        case " ":
          e.preventDefault();
          void api.toggle();
          break;
        case "/":
          e.preventDefault();
          setMode("search");
          void navigate({ to: "/search" });
          break;
        case "j":
          currentListHandler()?.move?.(1);
          break;
        case "k":
          currentListHandler()?.move?.(-1);
          break;
        case "g":
          if (handleSequenceG() === "top") {
            currentListHandler()?.top?.();
          }
          break;
        case "G":
          currentListHandler()?.bottom?.();
          break;
        case "Enter":
          currentListHandler()?.open?.();
          break;
        case "a":
          currentListHandler()?.stage?.();
          break;
        case "x":
          currentListHandler()?.remove?.();
          break;
        case "Escape":
          currentListHandler()?.back?.();
          break;
        case "}":
          void api.next();
          break;
        case "{":
          void api.prev();
          break;
        case "S":
          void navigate({ to: "/stats" });
          break;
        case "L":
          void navigate({ to: "/logs" });
          break;
        case "b":
          useUiStore.getState().togglePane("library");
          break;
        case "i":
          useUiStore.getState().togglePane("inspector");
          break;
        case "f":
          currentListHandler()?.fav?.();
          break;
        case "r":
          armRating();
          break;
        case "0":
        case "1":
        case "2":
        case "3":
        case "4":
        case "5":
          if (ratingArmed()) {
            currentListHandler()?.rate?.(Number(e.key));
          }
          break;
        case "m": {
          const vol = usePlayerStore.getState().volume;
          if (vol > 0) {
            lastVolume = vol;
            void api.setVolume(0);
          } else {
            void api.setVolume(Math.round((lastVolume || 1) * 100));
          }
          break;
        }
        case "=":
        case "+":
          void api.setVolume(
            Math.min(Math.round(usePlayerStore.getState().volume * 100) + 5, 100),
          );
          break;
        case "-":
          void api.setVolume(
            Math.max(Math.round(usePlayerStore.getState().volume * 100) - 5, 0),
          );
          break;
        default:
          break;
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [cycleFocus, navigate]);

  return (
    <div className="relative flex h-full flex-col">
      <TitleBar />
      <div className="flex min-h-0 flex-1 px-1">
        {libraryVisible && (
          <>
            <Pane
              id="library"
              title="library"
              className="shrink-0"
              style={{ width: libraryWidth }}
            >
              <LibraryNav />
            </Pane>
            <Resizer pane="library" />
          </>
        )}
        <Pane id="main" title="main" className="min-w-0 flex-1">
          <Outlet />
        </Pane>
        {inspectorVisible && (
          <>
            <Resizer pane="inspector" />
            <Pane
              id="inspector"
              title="inspector · queue"
              className="shrink-0"
              style={{ width: inspectorWidth }}
            >
              <div className="flex h-full flex-col">
                <div className="min-h-0 flex-1 overflow-auto">
                  <InspectorPane />
                </div>
                <div className="max-h-[45%] shrink-0 overflow-auto border-t border-subtle">
                  <QueuePanel />
                </div>
              </div>
            </Pane>
          </>
        )}
      </div>
      <StatusBar />
      <CommandPalette />
    </div>
  );
}

/** Drag handle between panes; drag to resize, double-click to hide. */
function Resizer({ pane }: { pane: "library" | "inspector" }) {
  const setPaneWidth = useUiStore((s) => s.setPaneWidth);
  const togglePane = useUiStore((s) => s.togglePane);

  const onMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const start =
      pane === "library"
        ? useUiStore.getState().libraryWidth
        : useUiStore.getState().inspectorWidth;

    const onMove = (ev: MouseEvent) => {
      const dx = ev.clientX - startX;
      setPaneWidth(pane, pane === "library" ? start + dx : start - dx);
    };
    const onUp = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      document.body.style.cursor = "";
    };
    document.body.style.cursor = "col-resize";
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  };

  return (
    <div
      onMouseDown={onMouseDown}
      onDoubleClick={() => togglePane(pane)}
      title="drag to resize · double-click to hide"
      className="group flex w-1 shrink-0 cursor-col-resize items-center justify-center"
    >
      <div className="h-10 w-0.5 bg-subtle group-hover:bg-accent" />
    </div>
  );
}

/** Drag region blending the macOS overlay titlebar into the app; leaves
 *  room for the traffic lights on the left. */
function TitleBar() {
  return (
    <header
      data-tauri-drag-region
      className="flex h-9 shrink-0 select-none items-center pl-[84px]"
    >
      <span data-tauri-drag-region className="pointer-events-none text-[11px]">
        <span className="text-accent">❯</span>{" "}
        <span className="text-secondary">signal</span>
        <span className="text-muted"> — local-first hi-fi player</span>
      </span>
    </header>
  );
}
