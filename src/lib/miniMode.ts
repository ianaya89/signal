import { getCurrentWindow, LogicalSize, PhysicalSize } from "@tauri-apps/api/window";

const MINI_W = 380;
const MINI_H = 132;

let savedSize: PhysicalSize | null = null;

export async function enterMiniWindow() {
  const win = getCurrentWindow();
  savedSize = await win.innerSize();
  await win.setAlwaysOnTop(true);
  await win.setSize(new LogicalSize(MINI_W, MINI_H));
}

export async function exitMiniWindow() {
  const win = getCurrentWindow();
  await win.setAlwaysOnTop(false);
  if (savedSize) {
    await win.setSize(savedSize);
    savedSize = null;
  } else {
    await win.setSize(new LogicalSize(1280, 800));
  }
}
