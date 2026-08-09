import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { sharedPackageAliases } from './vite.config';

const rootDir = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [react()],
  // Reuses vite.config.ts's table rather than restating it: without these, any
  // test touching `@vibe/shared/*` fails to resolve, and a *partial* copy
  // resolves React twice and fails with "invalid hook call" instead.
  resolve: {
    alias: sharedPackageAliases(rootDir),
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
    css: true,
    // GitHub Actions Ubuntu runners are 3-4× slower than dev macOS at
    // async render + state propagation. Default 5s test / 1s waitFor
    // produces flake on otherwise-correct tests. Raise globally so CI
    // matches local behavior; individual tests can still override.
    testTimeout: 15000,
  },
});
