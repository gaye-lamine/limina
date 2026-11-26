import { defineConfig } from '@playwright/test';

/**
 * End-to-end checks against a running app (ng serve on 4200) and API (3000).
 * Uses the installed Microsoft Edge, so no browser download is needed; set
 * PW_CHANNEL=chrome (or empty for Playwright's Chromium) to change it.
 */
export default defineConfig({
  testDir: './e2e',
  timeout: 180_000,
  workers: 1,
  reporter: [['list']],
  use: {
    baseURL: process.env['LIMINA_URL'] ?? 'http://localhost:4200',
    channel: process.env['PW_CHANNEL'] ?? 'msedge',
    colorScheme: 'dark',
  },
});
