import { useMemo, useState } from "react";

import type { Track } from "@/ipc/types";

export type SortKey = "default" | "title" | "duration" | "codec" | "rating";

export interface TrackSort {
  sorted: Track[];
  key: SortKey;
  desc: boolean;
  toggle: (key: SortKey) => void;
}

export function useTrackSort(tracks: Track[]): TrackSort {
  const [key, setKey] = useState<SortKey>("default");
  const [desc, setDesc] = useState(false);

  const toggle = (next: SortKey) => {
    if (next === key) {
      if (!desc) {
        setDesc(true);
      } else {
        setKey("default");
        setDesc(false);
      }
    } else {
      setKey(next);
      setDesc(false);
    }
  };

  const sorted = useMemo(() => {
    if (key === "default") return tracks;
    const copy = [...tracks];
    copy.sort((a, b) => {
      let cmp = 0;
      switch (key) {
        case "title":
          cmp = a.title.localeCompare(b.title);
          break;
        case "duration":
          cmp = a.durationMs - b.durationMs;
          break;
        case "codec":
          cmp = a.technical.codec.localeCompare(b.technical.codec);
          break;
        case "rating":
          cmp = (a.rating ?? 0) - (b.rating ?? 0);
          break;
        default:
          break;
      }
      return desc ? -cmp : cmp;
    });
    return copy;
  }, [tracks, key, desc]);

  return { sorted, key, desc, toggle };
}
