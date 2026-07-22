import { create } from "zustand";

export type PaneId = "library" | "main" | "inspector";

const PANE_ORDER: PaneId[] = ["library", "main", "inspector"];

interface UiState {
  focusedPane: PaneId;
  focusPane: (pane: PaneId) => void;
  cycleFocus: (direction: 1 | -1) => void;
}

export const useUiStore = create<UiState>((set) => ({
  focusedPane: "library",
  focusPane: (pane) => set({ focusedPane: pane }),
  cycleFocus: (direction) =>
    set((s) => {
      const idx = PANE_ORDER.indexOf(s.focusedPane);
      const next = (idx + direction + PANE_ORDER.length) % PANE_ORDER.length;
      return { focusedPane: PANE_ORDER[next] };
    }),
}));
