import { defineConfig } from 'vitest/config';

export default defineConfig({
  base: '/',
  build: { target: 'es2022' },
  test: { include: ['unit/**/*.test.ts'] },
});
