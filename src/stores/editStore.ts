import { create } from "zustand";

export type EditTarget =
  | { kind: "track"; id: number }
  | { kind: "album"; id: number };

interface EditState {
  target: EditTarget | null;
  openTrack: (id: number) => void;
  openAlbum: (id: number) => void;
  close: () => void;
}

export const useEditStore = create<EditState>((set) => ({
  target: null,
  openTrack: (id) => set({ target: { kind: "track", id } }),
  openAlbum: (id) => set({ target: { kind: "album", id } }),
  close: () => set({ target: null }),
}));
