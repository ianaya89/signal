import { cn } from "@/lib/utils";

/**
 * Section label inside a browse list.
 *
 * Deliberately the lightest thing that can still divide a list: a tick, a
 * label and a count, in the same 10px uppercase the rest of the app uses for
 * section headings. It sticks so you always know where you are in a long
 * scroll, and it is opaque rather than blurred — this is a terminal, not a
 * frosted panel.
 */
export function GroupHeader({
  label,
  count,
  className,
}: {
  label: string;
  count: number;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "sticky top-0 z-10 flex items-center gap-1.5 border-b border-subtle bg-surface px-3 py-0.5 text-[10px] uppercase tracking-wider text-muted",
        className,
      )}
    >
      <span
        aria-hidden
        className="h-2 w-0.5 shrink-0 bg-[color:var(--section,var(--accent-dim))]"
      />
      <span className="min-w-0 truncate">{label}</span>
      <span className="ml-auto shrink-0 tabular-nums">{count}</span>
    </div>
  );
}
