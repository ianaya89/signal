import { useScanStore } from "@/stores/scanStore";

export function StatusBar() {
  const { scanning, processed, total, currentPath } = useScanStore();

  return (
    <footer className="flex h-6 shrink-0 items-center justify-between gap-4 border border-subtle bg-surface px-2 text-[11px]">
      <span className="shrink-0 text-muted">■ stopped</span>
      {scanning ? (
        <span className="min-w-0 truncate text-accent">
          scanning {processed}/{total} · {basename(currentPath)}
        </span>
      ) : (
        <span className="truncate text-muted">
          tab: switch pane · space: play · /: search · ctrl+p: palette
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
