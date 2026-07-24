import { create } from "zustand";

export interface LogLine {
  id: number;
  level: string;
  target: string;
  message: string;
  at: string;
}

const MAX_LINES = 500;
let nextId = 0;

interface LogState {
  lines: LogLine[];
  push: (line: Omit<LogLine, "id" | "at">) => void;
  clear: () => void;
}

export const useLogStore = create<LogState>((set) => ({
  lines: [],
  push: (line) =>
    set((s) => {
      nextId += 1;
      const entry: LogLine = {
        ...line,
        id: nextId,
        at: new Date().toLocaleTimeString("en-GB"),
      };
      const lines = [...s.lines, entry];
      return { lines: lines.length > MAX_LINES ? lines.slice(-MAX_LINES) : lines };
    }),
  clear: () => set({ lines: [] }),
}));
