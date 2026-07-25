import { useCallback, useEffect, useState } from "react";

const OVERSCAN = 20;

/** Fixed-row-height windowing for long lists — render only what's visible
 *  plus overscan, pad the rest. No dependency, table-friendly. */
export function useVirtualWindow(
  count: number,
  rowHeight: number,
  containerRef: React.RefObject<HTMLElement | null>,
  enabled: boolean,
) {
  const [range, setRange] = useState({ start: 0, end: count });

  const recompute = useCallback(() => {
    const el = containerRef.current;
    if (!el || !enabled) {
      setRange({ start: 0, end: count });
      return;
    }
    const start = Math.max(Math.floor(el.scrollTop / rowHeight) - OVERSCAN, 0);
    const visible = Math.ceil(el.clientHeight / rowHeight) + OVERSCAN * 2;
    setRange({ start, end: Math.min(start + visible, count) });
  }, [containerRef, count, rowHeight, enabled]);

  useEffect(() => {
    recompute();
    const el = containerRef.current;
    if (!el || !enabled) return;
    el.addEventListener("scroll", recompute, { passive: true });
    window.addEventListener("resize", recompute);
    return () => {
      el.removeEventListener("scroll", recompute);
      window.removeEventListener("resize", recompute);
    };
  }, [recompute, containerRef, enabled]);

  /** Scrolls the container so `index` is visible (keyboard cursor moves). */
  const ensureVisible = useCallback(
    (index: number) => {
      const el = containerRef.current;
      if (!el || !enabled) return;
      const top = index * rowHeight;
      if (top < el.scrollTop) {
        el.scrollTop = top;
      } else if (top + rowHeight > el.scrollTop + el.clientHeight) {
        el.scrollTop = top + rowHeight - el.clientHeight;
      }
    },
    [containerRef, rowHeight, enabled],
  );

  if (!enabled) {
    return {
      start: 0,
      end: count,
      padTop: 0,
      padBottom: 0,
      ensureVisible,
    };
  }

  return {
    start: range.start,
    end: range.end,
    padTop: range.start * rowHeight,
    padBottom: Math.max(count - range.end, 0) * rowHeight,
    ensureVisible,
  };
}
