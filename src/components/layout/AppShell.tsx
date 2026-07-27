import { Outlet, useNavigate } from "@tanstack/react-router";
import { useEffect } from "react";

import { MetadataDialog } from "@/components/edit/MetadataDialog";
import { HelpOverlay } from "@/components/help/HelpOverlay";
import { InspectorPane } from "@/components/layout/InspectorPane";
import { LibraryNav } from "@/components/layout/LibraryNav";
import { Pane } from "@/components/layout/Pane";
import { StatusBar } from "@/components/layout/StatusBar";
import { CommandPalette } from "@/components/palette/CommandPalette";
import { DotPlayer } from "@/components/player/DotPlayer";
import { MiniPlayer } from "@/components/player/MiniPlayer";
import {
  ModeButtons,
  Timeline,
  TransportControls,
  VolumeSlider,
} from "@/components/player/TransportBar";
import { QueuePanel } from "@/components/queue/QueuePanel";
import { HeartEqualizer } from "@/components/ui/HeartEqualizer";
import { Toasts } from "@/components/ui/Toasts";
import { TooltipLayer } from "@/components/ui/TooltipLayer";
import { api } from "@/ipc/invoke";
import {
  armRating,
  currentListHandler,
  handleSequenceG,
  ratingArmed,
  useKeyboardStore,
} from "@/lib/keyboard";
import { dragWindow } from "@/lib/drag";
import { exitDotMode, setWindowMode } from "@/lib/miniMode";
import { cn } from "@/lib/utils";
import { usePlayerStore } from "@/stores/playerStore";
import { useUiStore } from "@/stores/uiStore";

// remembered across mute toggles (module-level; survives re-renders)
let lastVolume = 1;

export function AppShell() {
  const cycleFocus = useUiStore((s) => s.cycleFocus);
  const windowMode = useUiStore((s) => s.windowMode);
  const mainTitle = useUiStore((s) => s.mainTitle);
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

      // compact modes: space + esc only (esc steps up: dot → mini → full)
      const windowMode = useUiStore.getState().windowMode;
      if (windowMode !== "full") {
        if (e.key === " ") {
          e.preventDefault();
          void api.toggle();
        } else if (e.key === "Escape") {
          if (windowMode === "dot") {
            void exitDotMode();
          } else {
            void setWindowMode("full");
          }
        }
        return;
      }

      if (mode === "help") {
        if (e.key === "Escape" || e.key === "?") {
          e.preventDefault();
          setMode("normal");
        }
        return;
      }

      switch (e.key) {
        case "?":
          e.preventDefault();
          setMode("help");
          return;
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
        // arrows mirror j/k. preventDefault only when a list actually consumes
        // them, so views without one (logs, settings) keep native scrolling.
        case "ArrowDown":
        case "ArrowUp": {
          const move = currentListHandler()?.move;
          if (move) {
            e.preventDefault();
            move(e.key === "ArrowDown" ? 1 : -1);
          }
          break;
        }
        case "Home":
        case "End": {
          const handler = currentListHandler();
          const jump = e.key === "Home" ? handler?.top : handler?.bottom;
          if (jump) {
            e.preventDefault();
            jump();
          }
          break;
        }
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
        case "D":
          void navigate({ to: "/discover" });
          break;
        case "M":
          void setWindowMode("mini");
          break;
        case "P":
          void setWindowMode("dot");
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
        case "4":
        case "5":
          if (ratingArmed()) {
            currentListHandler()?.rate?.(Number(e.key));
          }
          break;
        case "1":
        case "2":
        case "3":
          if (ratingArmed()) {
            currentListHandler()?.rate?.(Number(e.key));
          } else {
            const panes = ["library", "main", "inspector"] as const;
            const pane = panes[Number(e.key) - 1];
            if (pane) useUiStore.getState().focusPane(pane);
          }
          break;
        case "[":
          void api.seek(
            Math.max(usePlayerStore.getState().positionMs - 5000, 0),
          );
          break;
        case "]":
          void api.seek(usePlayerStore.getState().positionMs + 5000);
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

  if (windowMode === "dot") {
    return <DotPlayer />;
  }
  if (windowMode === "mini") {
    return (
      <>
        <MiniPlayer />
        <TooltipLayer />
      </>
    );
  }

  return (
    <div className="relative flex h-full flex-col">
      <TitleBar />
      {/* 8px outer margin keeps square pane corners clear of the native
          window's rounded corners */}
      <div className="flex min-h-0 flex-1 px-2 pt-1.5">
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
        <Pane id="main" title={mainTitle} className="min-w-0 flex-1">
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
      <HelpOverlay />
      <MetadataDialog />
      <Toasts />
      <TooltipLayer />
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

/** Integrated titlebar mirroring the pane columns: brand block sits over
 *  the library pane, core transport + timeline over the main pane.
 *  Drags anywhere inert. */
function TitleBar() {
  const status = usePlayerStore((s) => s.status);
  const libraryVisible = useUiStore((s) => s.libraryVisible);
  const libraryWidth = useUiStore((s) => s.libraryWidth);

  return (
    <header
      onMouseDown={dragWindow}
      className="flex h-9 shrink-0 select-none items-center border-b border-subtle bg-surface pl-2 pr-3"
    >
      <span
        className="pointer-events-none flex shrink-0 items-center gap-1.5 pl-[76px] text-[11px]"
        style={{
          width: libraryVisible ? Math.max(libraryWidth + 4, 176) : undefined,
        }}
      >
        <HeartEqualizer size={16} playing={status === "playing"} />
        <span className="text-accent">❯</span>{" "}
        <span className="text-secondary">signal</span>
      </span>
      <span
        className={cn(
          "flex min-w-0 flex-1 items-center gap-3 text-[13px]",
          !libraryVisible && "pl-4",
        )}
      >
        <TransportControls />
        <Timeline className="flex-1" />
        <span className="flex shrink-0 items-center gap-3 text-[11px]">
          <ModeButtons />
          <VolumeSlider />
        </span>
      </span>
    </header>
  );
}
