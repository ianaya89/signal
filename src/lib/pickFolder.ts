import { open } from "@tauri-apps/plugin-dialog";

// Native NSOpenPanel — folders picked here get an automatic macOS TCC grant,
// which typed paths into protected locations (iCloud Drive) do not.
export async function pickFolder(): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "choose music folder",
  });
  return typeof selected === "string" ? selected : null;
}
