import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Off Tauri's default 1420 so this can run beside another Tauri app in dev.
// strictPort matters: without it vite would quietly pick a free port and the
// webview would keep loading whatever still answers on the configured one.
// Override with VITE_DEV_PORT, and pass the same port to tauri (see README).
//
// process is reached through globalThis because tsconfig keeps types to
// ["vite/client"] — one env read is not worth pulling in @types/node.
const env = (
  globalThis as { process?: { env?: Record<string, string | undefined> } }
).process?.env;
const DEV_PORT = Number(env?.VITE_DEV_PORT) || 1421;

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: DEV_PORT,
    strictPort: true,
    hmr: { port: DEV_PORT },
    watch: {
      ignored: ["**/src-tauri/**", "**/crates/**", "**/target/**"],
    },
  },
  build: {
    target: "es2022",
  },
  resolve: {
    alias: {
      "@": "/src",
    },
  },
});
