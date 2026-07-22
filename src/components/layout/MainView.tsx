export function MainView() {
  return (
    <div className="flex h-full flex-col">
      <div className="flex h-7 shrink-0 items-center gap-4 border-b border-subtle px-2 text-[11px] text-muted">
        <span className="w-8 text-right">#</span>
        <span className="flex-1">title</span>
        <span className="w-32">artist</span>
        <span className="w-24">codec</span>
        <span className="w-12 text-right">dur</span>
      </div>
      <div className="flex flex-1 items-center justify-center">
        <p className="text-muted">
          library empty — run <span className="text-accent">scan ~/Music</span>{" "}
          from the palette (<kbd className="rounded-sm bg-raised px-1">ctrl+p</kbd>)
        </p>
      </div>
    </div>
  );
}
