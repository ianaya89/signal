import { create } from "zustand";

import { api } from "@/ipc/invoke";
import type { PlayMode, RepeatMode } from "@/ipc/types";

const REPEAT_CYCLE: RepeatMode[] = ["off", "all", "one"];

interface PlayModeState extends PlayMode {
  restore: (mode: PlayMode) => void;
  toggleShuffle: () => void;
  cycleRepeat: () => void;
}

export const usePlayModeStore = create<PlayModeState>((set, get) => ({
  shuffle: false,
  repeat: "off",
  restore: (mode) => set(mode),
  toggleShuffle: () => {
    const next = { shuffle: !get().shuffle, repeat: get().repeat };
    set(next);
    void api.setPlayMode(next);
  },
  cycleRepeat: () => {
    const idx = REPEAT_CYCLE.indexOf(get().repeat);
    const next = {
      shuffle: get().shuffle,
      repeat: REPEAT_CYCLE[(idx + 1) % REPEAT_CYCLE.length] ?? "off",
    };
    set(next);
    void api.setPlayMode(next);
  },
}));
