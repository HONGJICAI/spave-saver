import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from '@tailwindcss/vite';

const host = process.env.TAURI_DEV_HOST;

// Determine build mode: 'tauri' or 'web'
const mode = process.env.VITE_MODE || 'tauri';

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [
    sveltekit(),
    tailwindcss(),
  ],

  // Define global constants based on mode
  define: {
    '__TAURI__': mode === 'tauri',
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: mode === 'web' ? 5173 : 1420,
    strictPort: mode === 'tauri',
    host: host || false,
    hmr: host
      ? {
        protocol: "ws",
        host,
        port: 1421,
      }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
