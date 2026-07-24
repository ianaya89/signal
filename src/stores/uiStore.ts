import { create } from "zustand";

import { api } from "@/ipc/invoke";

export type PaneId = "library" | "main" | "inspector";
export type Theme = "dark" | "light";

const PANE_ORDER: PaneId[] = ["library", "main", "inspector"];

function applyTheme(theme: Theme) {
  document.documentElement.dataset.theme = theme;
}

interface UiState {
  focusedPane: PaneId;
  theme: Theme;
  focusPane: (pane: PaneId) => void;
  cycleFocus: (direction: 1 | -1) => void;
  setTheme: (theme: Theme, persist?: boolean) => void;
  toggleTheme: () => void;
}

export const useUiStore = create<UiState>((set, get) => ({
  focusedPane: "library",
  theme: "dark",
  focusPane: (pane) => set({ focusedPane: pane }),
  cycleFocus: (direction) =>
    set((s) => {
      const idx = PANE_ORDER.indexOf(s.focusedPane);
      const next = (idx + direction + PANE_ORDER.length) % PANE_ORDER.length;
      return { focusedPane: PANE_ORDER[next] };
    }),
  setTheme: (theme, persist = true) => {
    applyTheme(theme);
    set({ theme });
    if (persist) {
      void api.settingsSet("ui.theme", theme).catch(() => {});
    }
  },
  toggleTheme: () => {
    get().setTheme(get().theme === "dark" ? "light" : "dark");
  },
}));
