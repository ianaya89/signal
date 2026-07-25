import { open, save } from "@tauri-apps/plugin-dialog";

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

export async function pickSavePath(
  defaultName: string,
  extension: string,
): Promise<string | null> {
  const selected = await save({
    defaultPath: defaultName,
    filters: [{ name: extension, extensions: [extension] }],
  });
  return selected ?? null;
}

export async function pickImage(): Promise<string | null> {
  const selected = await open({
    multiple: false,
    title: "choose album artwork",
    filters: [{ name: "images", extensions: ["jpg", "jpeg", "png"] }],
  });
  return typeof selected === "string" ? selected : null;
}
