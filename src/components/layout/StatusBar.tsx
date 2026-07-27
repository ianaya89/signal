import { useQuery } from "@tanstack/react-query";

import { NowPlayingLabel } from "@/components/player/TransportBar";
import { api } from "@/ipc/invoke";
import { dragWindow } from "@/lib/drag";
import { useKeyboardStore } from "@/lib/keyboard";
import { setWindowMode } from "@/lib/miniMode";
import { installUpdate } from "@/lib/updater";
import { cn } from "@/lib/utils";
import { useScanStore } from "@/stores/scanStore";
import { useUiStore } from "@/stores/uiStore";
import { useUpdateStore } from "@/stores/updateStore";

/** Top status strip: doubles as the window titlebar (macOS traffic lights sit
 *  in the left inset), so it stays draggable on any inert spot. */
export function StatusBar() {
  const { scanning, processed, total, currentPath, lastError, summary } =
    useScanStore();

  return (
    <header
      onMouseDown={dragWindow}
      className="flex h-9 shrink-0 select-none items-center justify-between gap-4 border-b border-subtle bg-surface pl-[76px] pr-3 text-[11px]"
    >
      <NowPlayingLabel />
      {scanning ? (
        <span className="min-w-0 shrink-0 truncate text-accent">
          scanning {processed}/{total} · {basename(currentPath)}
        </span>
      ) : lastError ? (
        <span
          className="min-w-0 truncate text-error"
          title={lastError}
        >
          ✕ {lastError}
        </span>
      ) : summary ? (
        <span className="min-w-0 shrink-0 truncate text-ok">{summary}</span>
      ) : (
        <span className="hidden shrink-0 text-muted lg:inline">
          tab: panes · space: play · /: search · ctrl+p: palette
        </span>
      )}
      <span className="flex shrink-0 items-center gap-2">
        <button
          type="button"
          onClick={() => void setWindowMode("mini")}
          title="mini player (M)"
          className="flex items-center text-[13px] leading-none text-muted hover:text-accent"
        >
          ▣
        </button>
        <button
          type="button"
          onClick={() => void setWindowMode("dot")}
          title="pulse mode (P)"
          className="flex items-center text-[11px] leading-none text-muted hover:text-accent"
        >
          ●
        </button>
        <PaneToggles />
        <button
          type="button"
          onClick={() => useKeyboardStore.getState().setMode("help")}
          title="keyboard shortcuts (?)"
          className="flex items-center text-[12px] leading-none text-muted hover:text-accent"
        >
          ?
        </button>
        <VersionLabel />
      </span>
    </header>
  );
}

function VersionLabel() {
  const { data: info } = useQuery({
    queryKey: ["app-info"],
    queryFn: api.appInfo,
    staleTime: Infinity,
  });
  const status = useUpdateStore((s) => s.status);
  const version = useUpdateStore((s) => s.version);
  const downloaded = useUpdateStore((s) => s.downloaded);
  const total = useUpdateStore((s) => s.total);

  if (status === "downloading") {
    const pct = total ? Math.round((downloaded / total) * 100) : null;
    return (
      <span className="text-accent">
        updating{pct === null ? "…" : ` ${pct}%`}
      </span>
    );
  }

  if (status === "available" && version) {
    return (
      <button
        type="button"
        onClick={() => void installUpdate()}
        title={`install v${version} and restart`}
        className="text-accent hover:underline"
      >
        ↑ v{version}
      </button>
    );
  }

  return <span className="text-muted">v{info?.version ?? "—"}</span>;
}

function PaneToggles() {
  const libraryVisible = useUiStore((s) => s.libraryVisible);
  const inspectorVisible = useUiStore((s) => s.inspectorVisible);
  const togglePane = useUiStore((s) => s.togglePane);

  return (
    <span className="flex gap-1">
      <button
        type="button"
        onClick={() => togglePane("library")}
        title="toggle library pane (b)"
        className={cn(
          "text-[13px]",
          libraryVisible ? "text-accent" : "text-muted hover:text-secondary",
        )}
      >
        ◧
      </button>
      <button
        type="button"
        onClick={() => togglePane("inspector")}
        title="toggle inspector pane (i)"
        className={cn(
          "text-[13px]",
          inspectorVisible ? "text-accent" : "text-muted hover:text-secondary",
        )}
      >
        ◨
      </button>
    </span>
  );
}

function basename(path: string): string {
  const idx = path.lastIndexOf("/");
  return idx === -1 ? path : path.slice(idx + 1);
}
