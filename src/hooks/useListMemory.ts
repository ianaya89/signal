import { useEffect, useRef, useState } from "react";

/**
 * Where you were in a list, kept for the session.
 *
 * Routing unmounts a view, so albums → artists → back used to drop you at the
 * top with the cursor on row 0. An editor reopens a file on the line you left
 * it on; a library browser should reopen a list on the record you left it on.
 *
 * Module-level maps rather than a store: nothing renders off this state, it is
 * read once on mount and written as you move.
 */
const cursors = new Map<string, number>();
const offsets = new Map<string, number>();

/** `useState` for a list cursor, seeded from the last visit. */
export function useListCursor(key: string) {
  const [cursor, setCursor] = useState(() => cursors.get(key) ?? 0);
  useEffect(() => {
    cursors.set(key, cursor);
  }, [key, cursor]);
  return [cursor, setCursor] as const;
}

/**
 * Restores the scroll offset of `ref` once `ready` (the query has rows, so the
 * container has height to scroll), then tracks it.
 */
export function useScrollMemory(
  key: string,
  ref: React.RefObject<HTMLElement | null>,
  ready: boolean,
) {
  const restored = useRef(false);
  useEffect(() => {
    const el = ref.current;
    if (!el || !ready) return;
    if (!restored.current) {
      restored.current = true;
      const saved = offsets.get(key);
      if (saved) el.scrollTop = saved;
    }
    const onScroll = () => offsets.set(key, el.scrollTop);
    el.addEventListener("scroll", onScroll, { passive: true });
    return () => el.removeEventListener("scroll", onScroll);
  }, [key, ref, ready]);
}
