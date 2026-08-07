import type { ReactNode } from "react";

import { cn } from "@/lib/utils";
import { useUiStore, type PaneId } from "@/stores/uiStore";

interface PaneProps {
  id: PaneId;
  title: string;
  className?: string;
  style?: React.CSSProperties;
  children: ReactNode;
}

export function Pane({ id, title, className, style, children }: PaneProps) {
  const focused = useUiStore((s) => s.focusedPane === id);
  const focusPane = useUiStore((s) => s.focusPane);
  const togglePane = useUiStore((s) => s.togglePane);
  const setMainHeaderSlot = useUiStore((s) => s.setMainHeaderSlot);

  return (
    <section
      onMouseDown={() => focusPane(id)}
      style={style}
      className={cn(
        "flex min-h-0 flex-col overflow-hidden border bg-surface",
        focused ? "border-focus" : "border-subtle",
        className,
      )}
    >
      <header className="flex h-6 shrink-0 items-center gap-2 border-b border-subtle px-2">
        <span
          className={cn(
            "shrink-0 text-[11px]",
            focused ? "text-accent" : "text-muted",
          )}
        >
          [ {title} ]
        </span>
        {/* the routed view fills this with its sort / filters / actions */}
        {id === "main" && (
          <span
            ref={(el) => {
              setMainHeaderSlot(el);
              return () => setMainHeaderSlot(null);
            }}
            className="flex min-w-0 flex-1 items-center justify-end gap-1 overflow-hidden"
          />
        )}
        <span className={cn("flex items-center gap-1.5", id !== "main" && "ml-auto")}>
          {focused && <span className="text-[10px] text-accent">▮</span>}
          {id !== "main" && (
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                togglePane(id);
              }}
              title={`hide pane (${id === "library" ? "b" : "i"})`}
              className="text-[13px] text-muted hover:text-error"
            >
              ✕
            </button>
          )}
        </span>
      </header>
      <div className="min-h-0 flex-1 overflow-auto">{children}</div>
    </section>
  );
}
