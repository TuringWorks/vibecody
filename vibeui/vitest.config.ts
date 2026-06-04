import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
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
