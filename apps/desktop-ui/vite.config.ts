import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig(({ mode }) => ({
  plugins: [svelte()],
  ...(mode === "test" ? { resolve: { conditions: ["browser"] } } : {}),
  clearScreen: false,
  server: {
    strictPort: true,
    port: 5173,
  },
}));
