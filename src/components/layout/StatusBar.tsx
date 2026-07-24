import { TransportBar } from "@/components/player/TransportBar";
import { useScanStore } from "@/stores/scanStore";

export function StatusBar() {
  const { scanning, processed, total, currentPath, lastError, summary } =
    useScanStore();

  return (
    <footer className="mx-1.5 mb-1.5 mt-1.5 flex h-8 shrink-0 items-center justify-between gap-4 rounded-[var(--radius)] border border-subtle bg-surface px-3 text-[11px]">
      <TransportBar />
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
      <span className="shrink-0 text-muted">signal v0.1.0</span>
    </footer>
  );
}

function basename(path: string): string {
  const idx = path.lastIndexOf("/");
  return idx === -1 ? path : path.slice(idx + 1);
}
