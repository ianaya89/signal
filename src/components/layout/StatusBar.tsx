export function StatusBar() {
  return (
    <footer className="flex h-6 shrink-0 items-center justify-between border border-subtle bg-surface px-2 text-[11px]">
      <span className="text-muted">■ stopped</span>
      <span className="text-muted">
        tab: switch pane · space: play · /: search · ctrl+p: palette
      </span>
      <span className="text-muted">signal v0.1.0</span>
    </footer>
  );
}
