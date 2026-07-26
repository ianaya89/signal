import { invoke } from "@tauri-apps/api/core";
import {
  currentMonitor,
  getCurrentWindow,
  LogicalSize,
  PhysicalPosition,
  PhysicalSize,
} from "@tauri-apps/api/window";

import { useUiStore, type WindowMode } from "@/stores/uiStore";

const SIZES: Record<Exclude<WindowMode, "full">, [number, number]> = {
  mini: [440, 118],
  dot: [76, 76],
};

let savedSize: PhysicalSize | null = null;
let savedPosition: PhysicalPosition | null = null;
// where the user was before collapsing to the dot — restore goes back there
let dotCameFrom: Exclude<WindowMode, "dot"> = "full";

/** Switches between full / mini / dot window shapes; remembers the full
 *  window's size AND position across the compact modes, clamping back
 *  on-screen if the restore would land outside the monitor. */
export async function setWindowMode(next: WindowMode) {
  const win = getCurrentWindow();
  const prev = useUiStore.getState().windowMode;
  if (prev === next) return;

  if (prev === "full") {
    savedSize = await win.innerSize();
    savedPosition = await win.outerPosition();
  }
  if (next === "dot") {
    dotCameFrom = prev === "mini" ? "mini" : "full";
  }

  // hide the native chrome (traffic lights) while compact
  await invoke("window_set_compact", { compact: next !== "full" }).catch(() => {});

  if (next === "full") {
    await win.setAlwaysOnTop(false);
    if (savedSize) {
      await win.setSize(savedSize);
    } else {
      await win.setSize(new LogicalSize(1280, 800));
    }
    if (savedPosition) {
      await win.setPosition(savedPosition);
    }
    await clampOnScreen();
    savedSize = null;
    savedPosition = null;
  } else {
    const [w, h] = SIZES[next];
    await win.setAlwaysOnTop(true);
    await win.setSize(new LogicalSize(w, h));
    // the compact window keeps its dragged spot, but never off-screen
    await clampOnScreen();
  }

  useUiStore.getState().setWindowMode(next);
}

/** Returns to whatever mode was active before collapsing into the dot. */
export async function exitDotMode() {
  await setWindowMode(dotCameFrom);
}

/** Repositions the window so it fits fully inside its monitor; centers
 *  only when it cannot fit at all. */
async function clampOnScreen() {
  const win = getCurrentWindow();
  const monitor = await currentMonitor();
  if (!monitor) return;

  const pos = await win.outerPosition();
  const size = await win.outerSize();
  const m = monitor.position;
  const ms = monitor.size;

  if (size.width > ms.width || size.height > ms.height) {
    await win.center();
    return;
  }

  const x = Math.min(Math.max(pos.x, m.x), m.x + ms.width - size.width);
  const y = Math.min(Math.max(pos.y, m.y), m.y + ms.height - size.height);
  if (x !== pos.x || y !== pos.y) {
    await win.setPosition(new PhysicalPosition(x, y));
  }
}
