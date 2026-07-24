import { create } from "zustand";

import { api } from "@/ipc/invoke";

export type PaneId = "library" | "main" | "inspector";
export type Theme = "dark" | "light";

function applyTheme(theme: Theme) {
  document.documentElement.dataset.theme = theme;
}

export interface LayoutState {
  libraryVisible: boolean;
  inspectorVisible: boolean;
  libraryWidth: number;
  inspectorWidth: number;
}

const LAYOUT_DEFAULTS: LayoutState = {
  libraryVisible: true,
  inspectorVisible: true,
  libraryWidth: 224,
  inspectorWidth: 288,
};

export const LIBRARY_WIDTH_RANGE = [160, 420] as const;
export const INSPECTOR_WIDTH_RANGE = [220, 500] as const;

const clamp = (v: number, [min, max]: readonly [number, number]) =>
  Math.min(Math.max(v, min), max);

let persistTimer: ReturnType<typeof setTimeout> | undefined;
function persistLayout(layout: LayoutState) {
  clearTimeout(persistTimer);
  persistTimer = setTimeout(() => {
    void api.settingsSet("ui.layout", JSON.stringify(layout)).catch(() => {});
  }, 400);
}

interface UiState extends LayoutState {
  focusedPane: PaneId;
  theme: Theme;
  focusPane: (pane: PaneId) => void;
  cycleFocus: (direction: 1 | -1) => void;
  setTheme: (theme: Theme, persist?: boolean) => void;
  toggleTheme: () => void;
  togglePane: (pane: "library" | "inspector") => void;
  setPaneWidth: (pane: "library" | "inspector", width: number) => void;
  restoreLayout: (layout: Partial<LayoutState>) => void;
}

export const useUiStore = create<UiState>((set, get) => ({
  focusedPane: "library",
  theme: "dark",
  ...LAYOUT_DEFAULTS,
  focusPane: (pane) => set({ focusedPane: pane }),
  cycleFocus: (direction) =>
    set((s) => {
      const order: PaneId[] = [
        ...(s.libraryVisible ? (["library"] as const) : []),
        "main",
        ...(s.inspectorVisible ? (["inspector"] as const) : []),
      ];
      const idx = Math.max(order.indexOf(s.focusedPane), 0);
      const next = (idx + direction + order.length) % order.length;
      return { focusedPane: order[next] };
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
  togglePane: (pane) => {
    set((s) => {
      const visible =
        pane === "library" ? !s.libraryVisible : !s.inspectorVisible;
      const patch: Partial<UiState> =
        pane === "library"
          ? { libraryVisible: visible }
          : { inspectorVisible: visible };
      if (!visible && s.focusedPane === pane) {
        patch.focusedPane = "main";
      }
      return patch;
    });
    persistLayout(pickLayout(get()));
  },
  setPaneWidth: (pane, width) => {
    set(
      pane === "library"
        ? { libraryWidth: clamp(width, LIBRARY_WIDTH_RANGE) }
        : { inspectorWidth: clamp(width, INSPECTOR_WIDTH_RANGE) },
    );
    persistLayout(pickLayout(get()));
  },
  restoreLayout: (layout) => {
    set({
      ...layout,
      libraryWidth: clamp(
        layout.libraryWidth ?? LAYOUT_DEFAULTS.libraryWidth,
        LIBRARY_WIDTH_RANGE,
      ),
      inspectorWidth: clamp(
        layout.inspectorWidth ?? LAYOUT_DEFAULTS.inspectorWidth,
        INSPECTOR_WIDTH_RANGE,
      ),
    });
  },
}));

function pickLayout(s: UiState): LayoutState {
  return {
    libraryVisible: s.libraryVisible,
    inspectorVisible: s.inspectorVisible,
    libraryWidth: s.libraryWidth,
    inspectorWidth: s.inspectorWidth,
  };
}
