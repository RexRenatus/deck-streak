// The planted card suite (SPEC-341 R8 to R12): tests-card/card.spec.ts in Chromium, WebKit and Firefox, one
// worker, against the card harness server (vite.card.config.ts). The listeners are module state of
// the one worker, and the suite's visits are timed, so nothing runs in parallel. Vite runs directly,
// not through `pnpm run`, as the main e2e configuration does.
import { defineConfig, devices } from '@playwright/test';

const port = Number(process.env.CARD_PORT ?? 4175);

export default defineConfig({
  testDir: 'tests-card',
  forbidOnly: !!process.env.CI,
  workers: 1,
  fullyParallel: false,
  timeout: 60_000,
  reporter: 'list',
  use: { baseURL: `http://127.0.0.1:${port}` },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
    { name: 'firefox', use: { ...devices['Desktop Firefox'] } }
  ],
  webServer: {
    command: 'vite --config vite.card.config.ts',
    url: `http://127.0.0.1:${port}/index.html`,
    reuseExistingServer: false,
    env: { CARD_PORT: String(port) }
  }
});
