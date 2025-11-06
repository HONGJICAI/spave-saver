import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import path from 'path';

export default defineConfig({
  plugins: [svelte({ hot: !process.env.VITEST })],
  test: {
    environment: 'happy-dom',
    include: ['src/**/*.{test,spec}.{js,ts}'],
    globals: true,
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      exclude: [
        'node_modules/',
        'src/routes/**',
        '**/*.d.ts',
        '**/*.config.*',
        '**/mockData/**',
        'dist/**',
      ],
    },
    setupFiles: ['./src/test/setup.ts'],
  },
  resolve: {
    alias: {
      $lib: path.resolve('./src/lib'),
    },
  },
});
