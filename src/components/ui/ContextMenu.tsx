import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

import { cn } from "@/lib/utils";

export interface MenuItem {
  label: string;
  onClick?: () => void;
  /** Renders a nested submenu on hover. */
  submenu?: MenuItem[];
  disabled?: boolean;
  separator?: boolean;
}

interface MenuPosition {
  x: number;
  y: number;
}

/** Imperative context menu: call `open(event, items)` from onContextMenu. */
export function useContextMenu() {
  const [state, setState] = useState<{ pos: MenuPosition; items: MenuItem[] } | null>(
    null,
  );

  const open = (e: React.MouseEvent, items: MenuItem[]) => {
    e.preventDefault();
    e.stopPropagation();
    setState({ pos: { x: e.clientX, y: e.clientY }, items });
  };

  const close = () => setState(null);

  const menu = state ? (
    <ContextMenuOverlay pos={state.pos} items={state.items} onClose={close} />
  ) : null;

  return { open, menu };
}

function ContextMenuOverlay({
  pos,
  items,
  onClose,
}: {
  pos: MenuPosition;
  items: MenuItem[];
  onClose: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [adjusted, setAdjusted] = useState(pos);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    setAdjusted({
      x: Math.min(pos.x, window.innerWidth - rect.width - 8),
      y: Math.min(pos.y, window.innerHeight - rect.height - 8),
    });
  }, [pos]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return createPortal(
    <div className="fixed inset-0 z-[95]" onMouseDown={onClose} onContextMenu={(e) => e.preventDefault()}>
      <div
        ref={ref}
        style={{ left: adjusted.x, top: adjusted.y }}
        className="absolute min-w-44 border border-focus bg-raised py-0.5"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <MenuList items={items} onClose={onClose} />
      </div>
    </div>,
    document.body,
  );
}

function MenuList({ items, onClose }: { items: MenuItem[]; onClose: () => void }) {
  const [openSub, setOpenSub] = useState<number | null>(null);

  return (
    <ul>
      {items.map((item, i) =>
        item.separator ? (
          <li key={i} className="my-0.5 border-t border-subtle" />
        ) : (
          <li
            key={i}
            className="relative"
            onMouseEnter={() => setOpenSub(item.submenu ? i : null)}
          >
            <button
              type="button"
              disabled={item.disabled}
              onClick={() => {
                if (item.submenu) return;
                item.onClick?.();
                onClose();
              }}
              className={cn(
                "flex w-full items-center justify-between gap-4 px-2 py-1 text-left text-[11px]",
                item.disabled
                  ? "text-muted"
                  : "text-secondary hover:bg-surface hover:text-accent",
              )}
            >
              {item.label}
              {item.submenu && <span className="text-muted">▸</span>}
            </button>
            {item.submenu && openSub === i && (
              <div className="absolute left-full top-0 min-w-40 border border-focus bg-raised py-0.5">
                <MenuList items={item.submenu} onClose={onClose} />
              </div>
            )}
          </li>
        ),
      )}
    </ul>
  );
}
