import type { ReactNode } from "react";

import { cn } from "@/lib/utils";
import { useUiStore, type PaneId } from "@/stores/uiStore";

interface PaneProps {
  id: PaneId;
  title: string;
  className?: string;
  children: ReactNode;
}

export function Pane({ id, title, className, children }: PaneProps) {
  const focused = useUiStore((s) => s.focusedPane === id);
  const focusPane = useUiStore((s) => s.focusPane);

  return (
    <section
      onMouseDown={() => focusPane(id)}
      className={cn(
        "flex min-h-0 flex-col overflow-hidden rounded-[var(--radius)] border bg-surface transition-colors duration-120",
        focused
          ? "border-focus/70 shadow-[0_0_0_1px_color-mix(in_srgb,var(--accent)_25%,transparent)]"
          : "border-subtle",
        className,
      )}
    >
      <header
        className={cn(
          "flex h-6 shrink-0 items-center justify-between border-b border-subtle px-2",
          focused ? "bg-raised/60" : undefined,
        )}
      >
        <span className={cn("text-[11px]", focused ? "text-accent" : "text-muted")}>
          {title}
        </span>
        {focused && <span className="text-[9px] text-accent">●</span>}
      </header>
      <div className="min-h-0 flex-1 overflow-auto">{children}</div>
    </section>
  );
}
