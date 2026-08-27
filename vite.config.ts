import { defineConfig } from 'vitest/config';
import { svelte, vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte({ preprocess: vitePreprocess({ script: true }) })],
  build: { target: 'es2022', sourcemap: true },
  server: { proxy: { '/api': 'http://localhost:8080' } },
  test: { environment: 'node' }
});
