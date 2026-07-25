import { useCallback, useRef, useState } from "react";

import type { Track } from "@/ipc/types";

/** Multi-selection over an ordered track list: cmd/ctrl+click toggles,
 *  shift+click extends from the last anchor. */
export function useMultiSelect(tracks: Track[]) {
  const [selected, setSelected] = useState<ReadonlySet<number>>(new Set());
  const anchor = useRef<number | null>(null);
  const tracksRef = useRef(tracks);
  tracksRef.current = tracks;

  const handleRowClick = useCallback(
    (index: number, e: React.MouseEvent): boolean => {
      const list = tracksRef.current;
      const id = list[index]?.id;
      if (id === undefined) return false;

      if (e.shiftKey && anchor.current !== null) {
        const [from, to] = [
          Math.min(anchor.current, index),
          Math.max(anchor.current, index),
        ];
        const range = list.slice(from, to + 1).map((t) => t.id);
        setSelected((prev) => new Set([...prev, ...range]));
        return true;
      }
      if (e.metaKey || e.ctrlKey) {
        anchor.current = index;
        setSelected((prev) => {
          const next = new Set(prev);
          if (next.has(id)) {
            next.delete(id);
          } else {
            next.add(id);
          }
          return next;
        });
        return true;
      }
      anchor.current = index;
      return false; // plain click: caller moves the cursor
    },
    [],
  );

  const clear = useCallback(() => {
    setSelected(new Set());
    anchor.current = null;
  }, []);

  return { selected, handleRowClick, clear };
}
