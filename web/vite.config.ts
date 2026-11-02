import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vite';

// In development the Rust server runs on :3000 (mise run server:dev); Vite
// forwards the API and the world files to it.
const server = 'http://127.0.0.1:3000';

export default defineConfig({
  plugins: [svelte({ configFile: false })],
  server: {
    proxy: {
      '/api': server,
      '/worlds': server,
      '/healthz': server,
    },
  },
  build: {
    // three.js alone is ~700 kB: one chunk is fine for a local tool.
    chunkSizeWarningLimit: 1500,
  },
});
