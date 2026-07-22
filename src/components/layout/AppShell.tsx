import { Outlet } from "@tanstack/react-router";
import { useEffect } from "react";

import { InspectorPane } from "@/components/layout/InspectorPane";
import { LibraryNav } from "@/components/layout/LibraryNav";
import { Pane } from "@/components/layout/Pane";
import { StatusBar } from "@/components/layout/StatusBar";
import { useUiStore } from "@/stores/uiStore";

export function AppShell() {
  const cycleFocus = useUiStore((s) => s.cycleFocus);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Tab") {
        e.preventDefault();
        cycleFocus(e.shiftKey ? -1 : 1);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [cycleFocus]);

  return (
    <div className="flex h-full flex-col gap-px bg-base p-px">
      <div className="flex min-h-0 flex-1 gap-px">
        <Pane id="library" title="library" className="w-56 shrink-0">
          <LibraryNav />
        </Pane>
        <Pane id="main" title="albums" className="flex-1">
          <Outlet />
        </Pane>
        <Pane id="inspector" title="inspector" className="w-72 shrink-0">
          <InspectorPane />
        </Pane>
      </div>
      <StatusBar />
    </div>
  );
}
