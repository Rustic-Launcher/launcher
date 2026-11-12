import { defineConfig, devices } from '@playwright/test';

/**
 * E2E runs against the browser mock (no Tauri). Chromium only (disk budget).
 * `pnpm e2e` runs the journeys; `pnpm screenshots` captures docs/screenshots + navigation video.
 */
export default defineConfig({
  testDir: './e2e',
  timeout: 60_000,
  expect: { timeout: 10_000 },
  fullyParallel: true,
  workers: 4,
  reporter: [['list']],
  outputDir: process.env.PW_OUTPUT ?? 'test-results',
  use: {
    baseURL: 'http://localhost:1420',
    viewport: { width: 1440, height: 900 },
    colorScheme: 'dark',
    locale: 'en-US',
    trace: 'retain-on-failure',
  },
  projects: [
    {
      name: 'e2e',
      testIgnore: /(screenshots|video)\.spec\.ts/,
      use: { ...devices['Desktop Chrome'], viewport: { width: 1440, height: 900 } },
    },
    {
      name: 'screenshots',
      testMatch: /(screenshots|video)\.spec\.ts/,
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 1440, height: 900 },
        deviceScaleFactor: 1,
      },
    },
  ],
  webServer: {
    command: 'pnpm exec vite --port 1420 --strictPort',
    url: 'http://localhost:1420',
    reuseExistingServer: true,
    timeout: 120_000,
  },
});
