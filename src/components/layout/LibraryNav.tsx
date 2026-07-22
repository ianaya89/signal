const SECTIONS = [
  { key: "a", label: "artists" },
  { key: "b", label: "albums" },
  { key: "g", label: "genres" },
  { key: "f", label: "folders" },
] as const;

export function LibraryNav() {
  return (
    <nav className="py-1">
      {SECTIONS.map((s) => (
        <div
          key={s.label}
          className="flex h-7 cursor-default items-center justify-between px-2 hover:bg-raised"
        >
          <span className="text-secondary">{s.label}</span>
          <kbd className="rounded-sm bg-raised px-1 text-[11px] text-muted">
            {s.key}
          </kbd>
        </div>
      ))}
    </nav>
  );
}
