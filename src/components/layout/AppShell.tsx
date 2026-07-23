import { Outlet, useNavigate } from "@tanstack/react-router";
import { useEffect } from "react";

import { InspectorPane } from "@/components/layout/InspectorPane";
import { LibraryNav } from "@/components/layout/LibraryNav";
import { Pane } from "@/components/layout/Pane";
import { StatusBar } from "@/components/layout/StatusBar";
import { CommandPalette } from "@/components/palette/CommandPalette";
import { QueuePanel } from "@/components/queue/QueuePanel";
import { api } from "@/ipc/invoke";
import { currentListHandler, handleSequenceG, useKeyboardStore } from "@/lib/keyboard";
import { useUiStore } from "@/stores/uiStore";

export function AppShell() {
  const cycleFocus = useUiStore((s) => s.cycleFocus);
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
        default:
          break;
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [cycleFocus, navigate]);

  return (
    <div className="relative flex h-full flex-col gap-px bg-base p-px">
      <div className="flex min-h-0 flex-1 gap-px">
        <Pane id="library" title="library" className="w-56 shrink-0">
          <LibraryNav />
        </Pane>
        <Pane id="main" title="main" className="flex-1">
          <Outlet />
        </Pane>
        <Pane id="inspector" title="inspector · queue" className="w-72 shrink-0">
          <div className="flex h-full flex-col">
            <div className="min-h-0 flex-1 overflow-auto">
              <InspectorPane />
            </div>
            <div className="max-h-[45%] shrink-0 overflow-auto border-t border-subtle">
              <QueuePanel />
            </div>
          </div>
        </Pane>
      </div>
      <StatusBar />
      <CommandPalette />
    </div>
  );
}
