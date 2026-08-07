// Keyboard layer: mode stack + sequence handling (docs/09-keyboard.md).
// One global keydown listener; components register list handlers via
// registerListHandler when focused.

import { create } from "zustand";

export type Mode = "normal" | "palette" | "search" | "help";

interface KeyboardState {
  mode: Mode;
  setMode: (mode: Mode) => void;
}

export const useKeyboardStore = create<KeyboardState>((set) => ({
  mode: "normal",
  setMode: (mode) => set({ mode }),
}));

/// Actions a focused list view can respond to.
export interface ListHandler {
  move?: (delta: 1 | -1) => void;
  /** Grids only: h/l · ←/→ step sideways while move steps a whole row. */
  moveCol?: (delta: 1 | -1) => void;
  top?: () => void;
  bottom?: () => void;
  open?: () => void;
  stage?: () => void; // 'a' — add to queue (git-add metaphor)
  remove?: () => void; // 'x'
  back?: () => void; // Esc
  fav?: () => void; // 'f' — toggle favorite
  rate?: (rating: number) => void; // 'r' then 0-5
  /** '.' — put the cursor on what is playing, returning whether the list held
   *  it. A false answer (or no handler) falls back to opening its album. */
  jump?: () => boolean;
}

let listHandler: ListHandler | null = null;

export function registerListHandler(handler: ListHandler): () => void {
  listHandler = handler;
  return () => {
    if (listHandler === handler) listHandler = null;
  };
}

export function currentListHandler(): ListHandler | null {
  return listHandler;
}

// gg sequence state: last 'g' press timestamp
let pendingG = 0;
const SEQ_TIMEOUT_MS = 500;

export function handleSequenceG(): "top" | "pending" {
  const now = Date.now();
  if (now - pendingG < SEQ_TIMEOUT_MS) {
    pendingG = 0;
    return "top";
  }
  pendingG = now;
  return "pending";
}

// r+digit rating sequence
let pendingR = 0;

export function armRating() {
  pendingR = Date.now();
}

/** Returns true when a digit arrives inside the r-sequence window. */
export function ratingArmed(): boolean {
  if (Date.now() - pendingR < SEQ_TIMEOUT_MS) {
    pendingR = 0;
    return true;
  }
  return false;
}
