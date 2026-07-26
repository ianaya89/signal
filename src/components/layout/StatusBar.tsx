import { NowPlayingLabel } from "@/components/player/TransportBar";
import { setWindowMode } from "@/lib/miniMode";
import { cn } from "@/lib/utils";
import { useScanStore } from "@/stores/scanStore";
import { useUiStore } from "@/stores/uiStore";

export function StatusBar() {
  const { scanning, processed, total, currentPath, lastError, summary } =
    useScanStore();

  return (
    <footer className="mx-2 mb-2 mt-1.5 flex h-7 shrink-0 items-center justify-between gap-4 border border-subtle bg-surface px-2 text-[11px]">
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
          title="mini player"
          className="text-[13px] text-muted hover:text-accent"
        >
          ▣
        </button>
        <button
          type="button"
          onClick={() => void setWindowMode("dot")}
          title="pulse mode (cmd+p)"
          className="text-[13px] text-muted hover:text-accent"
        >
          ●
        </button>
        <PaneToggles />
        <span className="text-muted">signal v0.1.0</span>
      </span>
    </footer>
  );
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
