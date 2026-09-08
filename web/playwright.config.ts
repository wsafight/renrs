import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests',
  workers: 1,
  use: {
    baseURL: process.env.RENRS_WEB_URL || 'http://127.0.0.1:4173',
    screenshot: 'only-on-failure',
  },
  reporter: 'list',
});
