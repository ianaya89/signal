import { create } from "zustand";

import { api } from "@/ipc/invoke";
import type { QueueEntry } from "@/ipc/types";

interface QueueState {
  entries: QueueEntry[];
  refresh: () => Promise<void>;
}

export const useQueueStore = create<QueueState>((set) => ({
  entries: [],
  refresh: async () => {
    const entries = await api.queueList();
    set({ entries });
  },
}));
