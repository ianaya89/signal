import { create } from "zustand";

import type { ScannerProgressEvent } from "@/ipc/types";

interface ScanState {
  scanning: boolean;
  processed: number;
  total: number;
  currentPath: string;
  start: () => void;
  progress: (p: ScannerProgressEvent) => void;
  done: () => void;
}

export const useScanStore = create<ScanState>((set) => ({
  scanning: false,
  processed: 0,
  total: 0,
  currentPath: "",
  start: () => set({ scanning: true, processed: 0, total: 0, currentPath: "" }),
  progress: (p) =>
    set({
      scanning: true,
      processed: p.processed,
      total: p.total,
      currentPath: p.currentPath,
    }),
  done: () => set({ scanning: false, currentPath: "" }),
}));
